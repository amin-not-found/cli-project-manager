use clap::{Arg, ArgAction, ArgGroup, command, Command};

// TODO : exec last accessed project when no argument is passed for exec subcommand
macro_rules! project_arg {
    ($name:tt,$help:tt) => {
        Arg::new($name).num_args(1).help($help).required(true).value_parser(|name: &str| -> Result<String, &str>{
            if name.ends_with("/"){
                return Ok(name.strip_suffix("/").unwrap().to_owned())
            }
            Ok(name.to_owned())
        })
    };
}
macro_rules! find_flag {
    ($name:tt,$help:tt) => {
        Arg::new($name)
            .help($help)
            .short($name.chars().next().unwrap())
            .action(ArgAction::SetTrue)
            .num_args(0)
    };
}

pub fn build() -> Command {
    command!()
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .short_flag('C')
                .about("Create a new project")
                .arg(project_arg!("project-name", "name of the project and its directory. you can also initiate a project using this command")),
        ).subcommand(
        Command::new("rename")
            .about("Rename an existing project(will change project directory)")
            .short_flag('R')
            .arg(project_arg!("project-name", "name of the existing project"))
            .arg(project_arg!("new-name", "new name of the project")),
    ).subcommand(
        Command::new("modify")
            .about("Modify tags of existing projects")
            .short_flag('M')
            .arg(project_arg!("project-name", "name of the project to modify"))
    ).subcommand(
        Command::new("exec")
            .about("Execute in a project")
            .short_flag('E')
            .arg(Arg::new("command")
                .short('c').help("command to execute in project directory. runs $SHELL by default")
                .required(false)
                .num_args(1)
                .default_value(""))
            .arg(project_arg!("project-name", "name of the project"))
    ).subcommand(
        Command::new("find")
            .short_flag('F')
            .about("interactive prompt to look for a project based on name and tags and then do something with it")
            .arg(find_flag!("invert", "reverse order of projects"))
            .arg(find_flag!("created", "sort projects by time created"))
            .arg(find_flag!("accessed", "sort projects by last time accessed using this program(default option)"))
            .arg(find_flag!("name","sort projects by name"))
            .group(
                ArgGroup::new("order").args(["created", "accessed", "name"]).required(false).multiple(false)
            )
            .arg(find_flag!("rename", "rename selected project"))
            .arg(find_flag!("modify", "modify tags of selected project"))
            .arg(Arg::new("execute")
                .short('e')
                .help("execute command in selected project directory(runs $SHELL if not specified. is default action)")
                .num_args(1)
                .required(false).default_value(""))
            .group(
                ArgGroup::new("action").args(["rename", "modify", "execute"]).required(false).multiple(false))
            .after_help("note: defaults to -Fae $SHELL as specified above"))
        .after_help("Note: to delete a project, just delete the directory containing it")
}
