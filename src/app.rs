use std::{collections::HashSet, path::Path, process::exit, time::SystemTime};

use clap::ArgMatches;
use inquire::{autocompletion::Replacement, validator::Validation, Autocomplete, Select, Text};

use crate::{
    config::Config,
    project::{Project, ProjectManager, SortOrder},
};

#[derive(Clone)]
struct Suggester {
    tags: HashSet<String>,
}

impl Suggester {
    pub fn new(tags: HashSet<String>) -> Self {
        Suggester { tags }
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
    ) -> Result<Replacement, inquire::CustomUserError> {
        Ok(highlighted_suggestion)
    }
}

fn handle_result<T>(res: Result<T, String>) -> T {
    match res {
        Err(e) => {
            eprintln!("ERROR: {}", e);
            exit(-1)
        }
        Ok(value) => value,
    }
}

fn choose_tags(manager: &mut ProjectManager, tags: &mut HashSet<String>) {
    loop {
        //let help_msg = tags.clone().into_iter().collect::<Vec<String>>().join(", ");
        let help_msg = "Press Esc to finish";
        println!("current tags: {:?}", tags);
        let tag = Text::new("Enter a tag to add or remove:")
            .with_help_message(help_msg)
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
            None => {
                println!("selected tags: {:?}", tags);
                return;
            }
        }
    }
}

fn create(mut manager: ProjectManager, args: &ArgMatches) {
    let mut tags = HashSet::<String>::new();
    let name: &String = args.get_one::<String>("project-name").unwrap();
    if manager.get_mut_project(name).is_ok() {
        eprintln!("Such project already exists");
        return;
    }
    choose_tags(&mut manager, &mut tags);
    let project = Project::new(name.to_owned(), SystemTime::now(), tags);
    handle_result(manager.create(project));
}

fn rename(mut manager: ProjectManager, args: &ArgMatches) {
    handle_result(manager.rename(
        args.get_one::<String>("project-name").unwrap(),
        args.get_one::<String>("new-name").unwrap(),
    ));
}

fn modify(mut manager: ProjectManager, args: &ArgMatches) {
    let name = args.get_one::<String>("project-name").unwrap();
    let project = handle_result(manager.get_mut_project(name));
    let mut tags = project.get_tags();
    choose_tags(&mut manager, &mut tags);
    handle_result(manager.modify(name, tags));
}

fn exec(manager: ProjectManager, default_executor: String, args: &ArgMatches) {
    handle_result(manager.exec(
        args.get_one::<String>("project-name").unwrap(),
        default_executor,
        args.get_one::<String>("command").unwrap(),
    ));
}

fn search(mut manager: ProjectManager, default_executor: String, args: &ArgMatches) {
    let order = match true {
        true if args.get_flag("created") => SortOrder::Creation,
        true if args.get_flag("name") => SortOrder::Name,
        _ => SortOrder::AccessTime,
    };
    let mut projects = manager.get_projects(order);
    if args.get_flag("invert") {
        projects.reverse();
    }
    // TODO : Handle case of no projects which results in inquire panicking
    let res = Select::new("Choose a project:", projects)
        .prompt_skippable()
        .unwrap();
    if res.is_none() {
        return;
    }
    let res = res.unwrap();
    match true {
        true if args.get_flag("rename") => {
            let temp = Text::new("New name:").prompt_skippable().unwrap();
            if let Some(name) = temp {
                handle_result(manager.rename(res.get_name(), &name))
            }
        }
        true if args.get_flag("modify") => {
            let name = res.get_name();
            let mut tags = res.get_tags();
            choose_tags(&mut manager, &mut tags);
            handle_result(manager.modify(name, tags))
        }
        // default to exec
        _ => handle_result(manager.exec(
            res.get_name(),
            default_executor,
            args.get_one::<String>("execute").unwrap(),
        )),
    }
}

pub fn handle(conf: Config, matches: ArgMatches) {
    let manager = ProjectManager::load(Path::new(&conf.dir).to_owned());
    if let Some((subcommand, args)) = matches.subcommand() {
        match subcommand {
            "create" => create(manager, args),
            "rename" => rename(manager, args),
            "modify" => modify(manager, args),
            "exec" => exec(manager, conf.exec, args),
            "find" => search(manager, conf.exec, args),
            _ => panic!("such subcommand({}) doesn't exist", subcommand),
        };
    }
}
