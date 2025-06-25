pub mod project;
mod config;
mod cli;
mod app;

// TODO : gen completion
fn main() {
    // TODO : make config customizable
    let conf = config::Config::default();
    let matches = cli::build().get_matches();
    app::handle(conf, matches);
}
