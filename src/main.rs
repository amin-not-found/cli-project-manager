mod config;
pub mod project;
mod app;
mod cli;

// TODO : gen completion

fn main() {
    let conf = config::Config::default();
    let matches = cli::build().get_matches();
    app::handle(conf.root, matches);
}
