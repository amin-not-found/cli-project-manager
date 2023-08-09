mod app;
mod cli;
mod config;
mod project;

// TODO : gen completion

fn main() {
    // TODO : make config customizable
    let conf = config::Config::new();
    let matches = cli::build().get_matches();
    app::handle(&conf.dir, matches);
}
