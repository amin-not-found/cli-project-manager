mod app;
mod cli;
mod config;
mod project;

// TODO : gen completion

fn main() {
    let conf = config::Config::default();
    let matches = cli::build().get_matches();
    app::handle(conf.root, matches);
}
