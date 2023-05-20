use core::panic;
use std::{
    collections::HashSet,
    env::{self},
    fs,
    path::PathBuf,
    process::Command,
    time::SystemTime,
};


// TODO : Maybe make methods consume the object for ProjectManager
// TODO : make sure much memory isn't used by program process when starting a new shell ProjectManager.exec(cmd)

use serde::{Deserialize, Serialize};

const PROJECT_FILE: &str = ".project.json";

#[derive(Serialize, Deserialize)]
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
        return Ok(());
    }
}

pub struct ProjectManager{
    root: PathBuf,
    projects: Vec<Project>,
    tags: HashSet<String>
    
}

impl ProjectManager {
    pub fn load(path: PathBuf) -> Self {
        let mut projects = Vec::<Project>::new();
        let mut tags = HashSet::<String>::new();
        if !path.is_dir() {
            panic!("Root directory not found or not a directory!");
        }

        for entry in fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap().path();
            if entry.is_dir()
                && entry
                    .read_dir()
                    .unwrap()
                    .any(|f| f.unwrap().file_name() == PROJECT_FILE)
            {
                let data = fs::read_to_string(entry.join(PROJECT_FILE)).unwrap();
                let project = serde_json::from_str::<Project>(&data);
                if let Ok(p) = project {
                    tags.extend(p.tags.clone());
                    projects.push(p);
                } else {
                    println!("WARINING: broken {} at {:?}", PROJECT_FILE, entry);
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
        if let None = project {
            return Err(format!("Such project({}) doesn't exist", name));
        }

        Ok(project.unwrap())
    }
    pub fn get_tags(&self) -> HashSet<String> {
        self.tags.clone()
    }
    pub fn insert_tag(&mut self, tag: String) {
        self.tags.insert(tag);
    }
    pub fn create(&mut self, project: Project) -> Result<(), String> {
        if let Ok(_) = self.get_mut_project(&project.name) {
            return Err(format!(
                "A project with name '{}' already exists",
                project.name
            ));
        }
        let path = self.get_path(&project.name);
        if !path.is_dir() {
            fs::create_dir(&path).unwrap();
        }
        project.save(path)?;
        Ok(())
    }
    pub fn rename(&mut self, src: &str, dst: &str) -> Result<(), String> {
        if let Ok(_) = self.get_mut_project(dst) {
            return Err(format!("A project with name '{}' already exists", dst));
        }

        let idx = self.projects.iter().position(|p| p.name == src).unwrap();
        let mut project = self.projects.remove(idx);

        let path: PathBuf = self.get_path(src);
        let mut new_path = path.clone();
        new_path.pop();
        new_path = new_path.join(&dst);

        fs::rename(path.clone(), &new_path).unwrap();
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
    pub fn exec(&mut self, name: &str, cmd: &str) -> Result<(), String> {
        let path: PathBuf = self.get_path(name);
        let project = self.get_mut_project(name)?;
        if cmd == "" {
            Command::new(env::var("SHELL").expect("Couldn't get default shell from $SHELL"))
                .current_dir("/home/amin/Codings")
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        } else {
            Command::new(cmd)
                .current_dir("/home/amin/Codings")
                .spawn()
                .unwrap();
        }
        project.accessed = SystemTime::now();
        project.save(path)?;
        Ok(())
    }
}
