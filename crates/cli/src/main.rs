mod cli;
mod commands;
mod loader;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build(args) => commands::build::run(args),
        Commands::Check(args) => commands::check::run(args),
    }
}
