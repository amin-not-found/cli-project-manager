use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub dir: String,
}

impl Config {
    pub fn new() -> Config {
        let path = dirs::config_dir()
            .expect("Couldn't retrieve config location for your system")
            .join("cli-project-manager.json");

        let config_text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Couldn't open file {:?}:\n{}", &path, e));

        serde_json::from_str(&config_text).unwrap()
    }
}
