pub struct Config {
    pub root: &'static str,
}

impl Config {
    pub fn default() -> Config {
        Config {
            root: "/home/amin/Coding",
        }
    }
}
