use core::panic;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Reverse,
    collections::HashSet,
    env::{self},
    fmt::Display,
    fs,
    io::Write,
    path::PathBuf,
    process::Command,
    time::SystemTime,
};
// TODO : make sure search works with substrings
const PROJECT_FILE: &str = ".project.json";

pub enum SortOrder {
    Creation,
    AccessTime,
    Name,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Project {
    name: String,
    created: SystemTime,
    accessed: SystemTime,
    tags: HashSet<String>,
}

impl Project {
    pub fn new(name: String, created_time: SystemTime, tags: HashSet<String>) -> Self {
        Project {
            name,
            created: created_time,
            accessed: created_time,
            tags,
        }
    }
    pub fn get_tags(&self) -> HashSet<String> {
        self.tags.clone()
    }
    pub fn get_name(&self) -> &String {
        &self.name
    }
    fn rename(&mut self, name: String) {
        self.name = name
    }
    fn modify(&mut self, new_tags: HashSet<String>) {
        self.tags = new_tags
    }
    fn save(&self, path: PathBuf) -> Result<(), String> {
        let res = fs::write(
            path.join(PROJECT_FILE),
            serde_json::to_string(self).unwrap(),
        );
        if let Err(e) = res {
            return Err(e.to_string());
        }
        Ok(())
    }
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.name,
            self.tags
                .clone()
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

pub struct ProjectManager {
    root: PathBuf,
    projects: Vec<Project>,
    tags: HashSet<String>,
}

impl ProjectManager {
    pub fn load(path: PathBuf) -> Self {
        let mut projects = Vec::<Project>::new();
        let mut tags = HashSet::<String>::new();
        if !path.is_dir() {
            panic!("Root directory({path:?}) not found or not a directory!");
        }

        for entry in fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap().path();
            if entry.is_dir()
                && entry
                    .read_dir()
                    .unwrap()
                    .any(|f| f.unwrap().file_name() == PROJECT_FILE)
            {
                let data = fs::read_to_string(entry.join(PROJECT_FILE)).unwrap_or_else(|e| {
                    panic!("Couldn't read {} in {:?}: {}", PROJECT_FILE, entry, e)
                });
                let project = serde_json::from_str::<Project>(&data);
                if let Ok(p) = project {
                    tags.extend(p.tags.clone());
                    projects.push(p);
                } else {
                    println!("WARNING: broken {} at {:?}", PROJECT_FILE, entry);
                }
            }
        }
        Self {
            root: path,
            projects,
            tags,
        }
    }
    pub fn get_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
    pub fn get_mut_project(&mut self, name: &str) -> Result<&mut Project, String> {
        let project = self.projects.iter_mut().find(|p| p.name == name);
        if project.is_none() {
            return Err(format!("Such project({}) doesn't exist", name));
        }

        Ok(project.unwrap())
    }
    pub fn get_projects(&self, order: SortOrder) -> Vec<Project> {
        let mut res = self.projects.clone();
        match order {
            SortOrder::Creation => res.sort_by_key(|p| Reverse(p.created)),
            SortOrder::AccessTime => res.sort_by_key(|p| Reverse(p.accessed)),
            SortOrder::Name => res.sort_by_key(|p| p.name.clone()),
        };
        res
    }
    pub fn get_tags(&self) -> HashSet<String> {
        self.tags.clone()
    }
    pub fn insert_tag(&mut self, tag: String) {
        self.tags.insert(tag);
    }
    pub fn create(&mut self, project: Project) -> Result<(), String> {
        if self.get_mut_project(&project.name).is_ok() {
            return Err(format!(
                "A project with name '{}' already exists",
                project.name
            ));
        }
        let path = self.get_path(&project.name);
        if !path.is_dir() {
            fs::create_dir(&path).unwrap();
        }
        let mut gitignore = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path.join(path.join(".gitignore")))
            .unwrap();
        writeln!(gitignore, "{}", PROJECT_FILE).unwrap();
        project.save(path)?;
        Ok(())
    }
    pub fn rename(&mut self, src: &str, dst: &str) -> Result<(), String> {
        if self.get_mut_project(dst).is_ok() {
            return Err(format!("A project with name '{}' already exists", dst));
        }

        let idx = self.projects.iter().position(|p| p.name == src).unwrap();
        let mut project = self.projects.remove(idx);

        let path: PathBuf = self.get_path(src);
        let mut new_path = path.clone();
        new_path.pop();
        new_path = new_path.join(dst);

        fs::rename(path.clone(), &new_path).unwrap_or_else(|e| panic!("Couldn't rename {:?} to {:?}.\n{}", &path, &new_path, e));
        project.rename(dst.to_string());
        project.save(new_path)?;
        self.projects.push(project);
        Ok(())
    }
    pub fn modify(&mut self, name: &str, tags: HashSet<String>) -> Result<(), String> {
        let path: PathBuf = self.get_path(name);
        let project = self.get_mut_project(name)?;
        project.modify(tags);
        project.save(path)?;
        Ok(())
    }
    pub fn exec(mut self, name: &str, cmd: &str) -> Result<(), String> {
        let path: PathBuf = self.get_path(name);
        let project = self.get_mut_project(name)?;

        project.accessed = SystemTime::now();
        project.save(path.clone())?;

        if cmd.is_empty() {
            // we will start $SHELL in project directory and this current
            // rust program is going to wait until we leave the shell.
            // so i'm going to drop some values that might use some memory
            drop(self);

            Command::new(env::var("SHELL").expect("Couldn't get default shell from $SHELL"))
                .current_dir(&path)
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        } else {
            let cmd = cmd.replace("{}", &path.to_string_lossy());
            let cmd: Vec<&str> = cmd.split(' ').collect();
            Command::new(cmd[0])
                .args(&cmd[1..])
                .current_dir(&path)
                .spawn()
                .unwrap();
        }
        Ok(())
    }
}
