use std::{collections::HashSet, path::Path, time::SystemTime, process::exit};

use clap::ArgMatches;
use inquire::{autocompletion::Replacement, validator::Validation, Autocomplete, Text};

use crate::project::{Project, ProjectManager};

#[derive(Clone)]
struct Suggester {
    tags: HashSet<String>,
}

impl Suggester {
    pub fn new(tags: HashSet<String>) -> Self {
        Suggester { tags: tags }
    }
}

impl Autocomplete for Suggester {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        Ok(self
            .tags
            .clone()
            .into_iter()
            .filter(|t| t.starts_with(&input.to_lowercase()))
            .collect::<Vec<_>>())
    }
    fn get_completion(
        &mut self,
        _: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<inquire::autocompletion::Replacement, inquire::CustomUserError> {
        Ok(match highlighted_suggestion {
            Some(suggestion) => Replacement::Some(suggestion),
            None => Replacement::None,
        })
    }
}

fn handle_result<T>(res: Result<T, String>) -> T{
    match res{
        Err(e) => {
            {
                eprintln!("ERROR: {}", e);
                exit(-1)
            }
        },
        Ok(value) => value 
    }
}

fn choose_tags(manager: &mut ProjectManager, tags: &mut HashSet<String>) {
    loop {
        println!("current tags: {:?}", tags);
        let tag = Text::new("Enter a tag to add or remove:")
            .with_help_message("prss ESC to finish")
            .with_autocomplete(Suggester::new(manager.get_tags()))
            .with_validator(|tag: &str| {
                if tag.contains(char::is_whitespace) {
                    return Ok(Validation::Invalid(
                        "Tag shouldn't contain whitespace".into(),
                    ));
                }
                Ok(Validation::Valid)
            })
            .with_formatter(&|s: &str| s.to_lowercase())
            .prompt_skippable()
            .unwrap();
        match tag {
            Some(tag) => {
                if tags.contains(&tag) {
                    tags.remove(&tag);
                } else {
                    manager.insert_tag(tag.to_owned());
                    tags.insert(tag.to_owned());
                }
            }
            None => return,
        }
    }
}

fn create(manager: &mut ProjectManager, args: &ArgMatches) {
    let mut tags = HashSet::<String>::new();
    let name: &String = args.get_one::<String>("project-name").unwrap();
    if let Ok(_) = manager.get_mut_project(name) {
        eprintln!("Such project already exists");
        return;
    }
    choose_tags(manager, &mut tags);
    let project = Project::new(name.to_owned(), SystemTime::now(), tags);
    handle_result(manager.create(project));
}

fn rename(manager: &mut ProjectManager, args: &ArgMatches) {
    handle_result(manager.rename(
        args.get_one::<String>("project-name").unwrap(),
        args.get_one::<String>("new-name").unwrap(),
    ));
}

fn modify(manager: &mut ProjectManager, args: &ArgMatches) {
    let name = args.get_one::<String>("project-name").unwrap();
    let project = handle_result(manager.get_mut_project(name));
    let mut tags = project.get_tags();
    choose_tags(manager, &mut tags);
    handle_result(manager.modify(name, tags));
}

fn exec(manager: &mut ProjectManager, args: &ArgMatches) {
    handle_result(manager.exec(
        args.get_one::<String>("project-name").unwrap(),
        args.get_one::<String>("command").unwrap(),
    ));
}

pub fn handle(root: &str, macthes: ArgMatches) {
    let mut manager = ProjectManager::load(Path::new(root).to_owned());
    if let Some((subcommand, args)) = macthes.subcommand() {
        match subcommand {
            "create" => create(&mut manager, args),
            "rename" => rename(&mut manager, args),
            "modify" => modify(&mut manager, args),
            "exec" => exec(&mut manager, args),
            "search" => (),
            _ => (),
        };
    }
}
