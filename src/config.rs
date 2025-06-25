use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub dir: PathBuf,  // root directory
    pub exec: String, // default program to execute/open projects with
}

impl Default for Config {
    fn default() -> Self {
        let path = dirs::config_dir()
            .expect("Couldn't retrieve config location for your system")
            .join("cli-project-manager.json");

        let config_text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Couldn't open file {:?}:\n{}", &path, e));

        serde_json::from_str(&config_text).unwrap()
    }
}
