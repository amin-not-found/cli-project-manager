// TODO : make sure search works with substrings
use core::panic;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Reverse, collections::HashSet, fmt::Display, fs, io::Write, path::PathBuf,
    process::Command, time::SystemTime,
};
use time::{
    format_description::well_known::{
        iso8601::{self, TimePrecision},
        Iso8601,
    },
    OffsetDateTime,
};

pub const PROJECT_FILE: &str = ".project.json";
const TIME_CONFIG: iso8601::EncodedConfig = iso8601::Config::DEFAULT
    .set_year_is_six_digits(false)
    .set_time_precision(TimePrecision::Second {
        decimal_digits: std::num::NonZeroU8::new(7),
    })
    .encode();
const TIME_FORMAT: Iso8601<TIME_CONFIG> = Iso8601::<TIME_CONFIG>;
time::serde::format_description!(time_format, OffsetDateTime, TIME_FORMAT);

#[derive(Debug, PartialEq, Clone)]
pub enum ProjectErrorTypes {
    DirectoryRead,
    DirectoryWrite,
    ProjectRead,
    ProjectWrite,
    NonExistingProject,
}

#[derive(Debug, Clone)]
pub struct ProjectError {
    pub typ: ProjectErrorTypes,
    pub msg: String,
}

pub enum SortOrder {
    Creation,
    AccessTime,
    Name,
}

fn default_project_atime() -> OffsetDateTime {
    OffsetDateTime::UNIX_EPOCH
}

fn empty_hash_set() -> HashSet<String> {
    HashSet::new()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectInfo {
    #[serde(default = "default_project_atime")]
    #[serde(with = "time_format")]
    accessed: OffsetDateTime,
    #[serde(default = "empty_hash_set")]
    tags: HashSet<String>,
}

impl ProjectInfo {
    pub fn new(accessed_time: OffsetDateTime, tags: HashSet<String>) -> Self {
        ProjectInfo {
            accessed: accessed_time,
            tags,
        }
    }
    pub fn save(&self, path: PathBuf) -> Result<(), ProjectError> {
        let res = fs::write(
            path.join(PROJECT_FILE),
            serde_json::to_string(self).unwrap(),
        );
        if let Err(e) = res {
            return Err(ProjectError {
                typ: ProjectErrorTypes::ProjectWrite,
                msg: e.to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    name: String,
    created: SystemTime,
    info: ProjectInfo,
}

impl Project {
    pub fn new(name: String, created_time: SystemTime, tags: HashSet<String>) -> Self {
        Self::with_info(
            name,
            created_time,
            ProjectInfo::new(created_time.into(), tags),
        )
    }
    pub fn with_info(name: String, created_time: SystemTime, info: ProjectInfo) -> Self {
        Project {
            name,
            created: created_time,
            info,
        }
    }
    pub fn get_tags(&self) -> HashSet<String> {
        self.info.tags.clone()
    }
    pub fn get_name(&self) -> &String {
        &self.name
    }
    fn rename(&mut self, name: String) {
        self.name = name
    }
    fn modify(&mut self, new_tags: HashSet<String>) {
        self.info.tags = new_tags
    }
}

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.name,
            self.info
                .tags
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
    pub fn load(path: PathBuf) -> (Self, Vec<ProjectError>) {
        let mut manager = ProjectManager {
            root: path.clone(),
            projects: Vec::<Project>::new(),
            tags: HashSet::<String>::new(),
        };
        let mut errors = Vec::<ProjectError>::new();

        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(e) => {
                errors.push(ProjectError {
                    typ: ProjectErrorTypes::DirectoryRead,
                    msg: format!("Couldn't read root directory({:?}). Error:\n{}\n", path, e),
                });
                return (manager, errors);
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e.path(),
                Err(e) => {
                    errors.push(ProjectError {
                        typ: ProjectErrorTypes::DirectoryRead,
                        msg: format!("Error while reading item in root directory:\n {}\n", e),
                    });
                    continue;
                }
            };

            if !entry.is_dir() {
                continue;
            }

            if !entry
                .read_dir()
                .unwrap()
                .any(|f| f.is_ok_and(|f| f.file_name() == PROJECT_FILE))
            {
                continue;
            }

            let created_time = match entry.metadata() {
                Err(e) => {
                    errors.push(ProjectError {
                        typ: ProjectErrorTypes::DirectoryRead,
                        msg: format!("Couldn't get metadata for directory {:?}:\n{}\n", path, e),
                    });
                    continue;
                }
                Ok(m) => m.created().unwrap_or_else(|e| {
                    panic!("Couldn't get created time for {:?}:\n{}\n", path, e)
                }),
            };

            let data = match fs::read_to_string(entry.join(PROJECT_FILE)) {
                Ok(data) => data,
                Err(e) => {
                    errors.push(ProjectError {
                        typ: ProjectErrorTypes::ProjectRead,
                        msg: format!("Couldn't read {} in {:?}:\n{}\n", PROJECT_FILE, entry, e),
                    });
                    continue;
                }
            };

            let project_info = serde_json::from_str::<ProjectInfo>(&data);
            let name = match entry.file_name().unwrap().to_str() {
                Some(name) => name,
                None => {
                    errors.push(ProjectError {
                        typ: ProjectErrorTypes::DirectoryRead,
                        msg: format!("Non UTF-8 paths aren't supported(path: {:?})", path),
                    });
                    continue;
                }
            };

            if let Ok(info) = project_info {
                manager.tags.extend(info.tags.clone());
                manager
                    .projects
                    .push(Project::with_info(name.into(), created_time, info));
            } else {
                println!("WARNING: broken {} at {:?}", PROJECT_FILE, entry);
            }
        }
        (manager, errors)
    }
    pub fn get_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
    pub fn get_mut_project(&mut self, name: &str) -> Result<&mut Project, ProjectError> {
        let project = self.projects.iter_mut().find(|p| p.name == name);

        match project {
            Some(project) => Ok(project),
            None => Err(ProjectError {
                typ: ProjectErrorTypes::NonExistingProject,
                msg: format!("A project with name '{}' doesn't exist", name),
            }),
        }
    }
    pub fn get_projects(&self, order: SortOrder) -> Vec<Project> {
        let mut res = self.projects.clone();
        match order {
            SortOrder::Creation => res.sort_by_key(|p| Reverse(p.created)),
            SortOrder::AccessTime => res.sort_by_key(|p| Reverse(p.info.accessed)),
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
    pub fn create(&mut self, name: String, tags: HashSet<String>) -> Result<(), ProjectError> {
        if self.get_mut_project(&name).is_ok() {
            return Err(ProjectError {
                typ: ProjectErrorTypes::ProjectWrite,
                msg: format!("A project with name '{}' already exists", name),
            });
        }
        let path = self.get_path(&name);
        if !path.is_dir() {
            if let Err(e) = fs::create_dir(&path) {
                return Err(ProjectError {
                    typ: ProjectErrorTypes::DirectoryWrite,
                    msg: format!(
                        "Couldn't create directory for project with path {:?}:\n{}\n",
                        path, e
                    ),
                });
            }
        }

        let gitignore = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path.join(path.join(".gitignore")));

        match gitignore {
            Err(e) => eprintln!(
                "Couldn't open/create gitignore in project directory({:?}):\n{}\n",
                path, e
            ),
            Ok(mut file) => {
                if let Err(e) = writeln!(&mut file, "{}", PROJECT_FILE) {
                    eprintln!("Couldn't write to gitignore in project directory({:?}) after successful open:\n{}\n",path, e);
                };
            }
        };

        self.tags.extend(tags.clone());
        let project = Project::new(name, SystemTime::now(), tags);
        project.info.save(path)?;
        self.projects.push(project);
        Ok(())
    }
    pub fn rename(&mut self, src: &str, dst: &str) -> Result<(), ProjectError> {
        let path: PathBuf = self.get_path(src);
        let mut new_path = path.clone();
        new_path.pop();
        new_path = new_path.join(dst);

        if new_path.exists() {
            return Err(ProjectError {
                typ: ProjectErrorTypes::DirectoryWrite,
                msg: format!("A directory with name '{}' already exists", dst),
            });
        }

        let project = self.get_mut_project(src)?;

        if let Err(e) = fs::rename(path.clone(), &new_path) {
            return Err(ProjectError {
                typ: ProjectErrorTypes::ProjectWrite,
                msg: format!("Couldn't rename {:?} to {:?}.\n{}\n", &path, &new_path, e),
            });
        }
        project.rename(dst.to_string());
        project.info.accessed = OffsetDateTime::now_utc();
        project.info.save(new_path)?;
        Ok(())
    }
    pub fn modify(&mut self, name: &str, tags: HashSet<String>) -> Result<(), ProjectError> {
        let path: PathBuf = self.get_path(name);
        let project = self.get_mut_project(name)?;
        project.modify(tags.clone());
        project.info.save(path)?;
        self.tags.extend(tags);
        Ok(())
    }
    pub fn exec(
        mut self,
        name: &str,
        default_executor: String,
        cmd: &str,
    ) -> Result<(), ProjectError> {
        let mut cmd = cmd;
        let path: PathBuf = self.get_path(name);
        let project = self.get_mut_project(name)?;

        project.info.accessed = OffsetDateTime::now_utc();
        project.info.save(path.clone())?;

        // we will start a program in project directory and this current
        // rust program might need to wait until the program finishes. so
        // i'm going to drop projects data just in case it uses too much memory
        drop(self);

        if cmd.is_empty() {
            cmd = &default_executor;
        }
        let path = match path.to_str() {
            Some(p) => p,
            None => {
                return Err(ProjectError {
                    typ: ProjectErrorTypes::DirectoryRead,
                    msg: format!("Non UTF-8 paths aren't supported(path: {:?})", path),
                });
            }
        };
        let cmd = cmd.replace("{}", path);
        let cmd: Vec<&str> = cmd.split(' ').collect();

        Command::new(cmd[0])
            .args(&cmd[1..])
            .current_dir(path)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        Ok(())
    }
}
