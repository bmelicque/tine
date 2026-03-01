use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tine")]
#[command(about = "Tine programming language")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Build(BuildArgs),
    Check(CheckArgs),
}

#[derive(clap::Args)]
pub struct BuildArgs {
    pub input: String,
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct RunArgs {
    pub input: String,
}

#[derive(clap::Args)]
pub struct CheckArgs {
    pub input: String,
}

#[derive(clap::Args)]
pub struct NewArgs {
    pub name: String,
}
