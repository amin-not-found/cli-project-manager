use std::hash::{DefaultHasher, Hash, Hasher};

pub mod config;
pub mod project;

fn setup() -> config::Config {
    let mut hasher = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut hasher);

    let test_dir = format!("test_{:x}", hasher.finish());
    let dir = std::env::current_dir().unwrap().join(test_dir);
    std::fs::create_dir(&dir).unwrap();

    config::Config {
        dir,
        exec: String::from("bash"),
    }
}

fn cleanup(config: config::Config) {
    if config.dir.exists() {
        std::fs::remove_dir_all(config.dir).unwrap();
    }
}

fn run_test<T>(test: T)
where
    T: FnOnce(&config::Config) + std::panic::UnwindSafe,
{
    let config = setup();
    let result = std::panic::catch_unwind(|| test(&config));
    cleanup(config);
    assert!(result.is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectErrorTypes;
    use std::collections::HashSet;

    // TODO : show values in assert messages

    #[test]
    fn non_existing_dir() {
        run_test(|config| {
            // Non existing root directory
            std::fs::remove_dir(&config.dir).unwrap();
            let (manager, errors) = project::ProjectManager::load(config.dir.clone());
            assert!(
                manager
                    .get_projects(project::SortOrder::AccessTime)
                    .is_empty(),
                "Empty project list with non existing root directory"
            );
            assert!(
                errors.len() == 1 && errors[0].typ == ProjectErrorTypes::DirectoryRead,
                "Only one error of DirectoryRead with non existing root directory"
            );
        })
    }

    #[test]
    fn general() {
        run_test(|config| {
            // empty root directory
            let (mut manager, mut errors) = project::ProjectManager::load(config.dir.clone());
            assert!(
                manager
                    .get_projects(project::SortOrder::AccessTime)
                    .is_empty(),
                "Empty project list in empty root directory"
            );
            assert!(
                errors.is_empty(),
                "Empty error list in empty root directory"
            );
            assert!(
                manager.get_tags().is_empty(),
                "Empty tag list in empty root directory"
            );

            // correct path
            assert!(manager.get_path("test") == config.dir.join("test"));

            // project creation
            let mut tags = HashSet::<String>::new();
            tags.insert("rust".into());
            assert!(
                manager.create(String::from("proj0"), tags.clone()).is_ok(),
                "Valid project creation"
            );
            assert!(
                manager.get_tags() == tags,
                "{:?} == {:?}",
                manager.get_tags(),
                tags
            );
            assert!(
                manager.get_path("proj0").is_dir(),
                "Existence of created project directory"
            );
            assert!(
                config
                    .dir
                    .join("proj0")
                    .join(project::PROJECT_FILE)
                    .is_file(),
                "Existence of created project's config"
            );
            assert!(
                config.dir.join("proj0").join(".gitignore").is_file(),
                "Existence of created project's gitignore"
            );

            // creating project with same name
            assert!(
                manager
                    .create(String::from("proj0"), HashSet::new())
                    .is_err_and(|e| e.typ == ProjectErrorTypes::ProjectWrite),
                "Invalid creation of project with the same name."
            );

            // Testing project listing with different orders
            tags = HashSet::<String>::new();
            tags.insert("python".into());
            manager.create(String::from("proj1"), tags.clone()).unwrap();

            let mut projects_by_atime = manager.get_projects(project::SortOrder::AccessTime);
            assert!(projects_by_atime.len() == 2);
            assert!(projects_by_atime[0].get_name() == "proj1");
            assert!(projects_by_atime[1].get_name() == "proj0");

            // just to check if manager reloads correctly
            (manager, errors) = project::ProjectManager::load(config.dir.clone());
            assert!(errors.is_empty());
            tags.insert("rust".into());
            assert!(
                manager.get_tags() == tags,
                "{:?} == {:?}",
                manager.get_tags(),
                tags
            );

            let mut projects_by_ctime = manager.get_projects(project::SortOrder::Creation);
            assert!(projects_by_ctime.len() == 2 && projects_by_ctime[0].get_name() == "proj1");

            let projects_by_name = manager.get_projects(project::SortOrder::Name);
            assert!(projects_by_name.len() == 2 && projects_by_name[0].get_name() == "proj0");

            // Testing valid and invalid rename
            assert!(manager
                .rename("proj", "proj2")
                .is_err_and(|e| e.typ == ProjectErrorTypes::NonExistingProject));
            assert!(
                manager
                    .rename("proj0", "proj1")
                    .is_err_and(|e| e.typ == ProjectErrorTypes::DirectoryWrite),
                "Rename to existing project"
            );
            assert!(
                manager.rename("proj0", "proj2").is_ok(),
                "Valid project rename"
            );
            assert!(manager.get_path("proj2").is_dir(), "check rename");
            // check changes in ordering after rename
            projects_by_atime = manager.get_projects(project::SortOrder::AccessTime);
            assert!(
                projects_by_atime[0].get_name() == "proj2",
                "access time order change after rename"
            );

            // Testing modify
            let mut new_tags = HashSet::<String>::new();
            new_tags.insert("c".into());
            tags.insert("c".into());
            manager.modify("proj2", new_tags.clone()).unwrap();
            assert!(
                manager.get_tags() == tags,
                "correct tags after modification of a project -> {:?} == {:?}",
                manager.get_tags(),
                tags
            );

            let proj2 = manager.get_mut_project("proj2");
            assert!(
                proj2.is_ok(),
                "retrieval of renamed project(get_mut_project is also first tested here)"
            );

            assert!(proj2.unwrap().get_tags() == new_tags, "check modified tags");
        })
    }
}
