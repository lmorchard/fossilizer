use anyhow::Result;
use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;
use std::path::{Path, PathBuf};

use fossilizer::{app, db};

pub mod build;
pub mod import;
pub mod fetch;
pub mod init;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    quiet: bool,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the data directory
    Init(init::InitArgs),
    /// Upgrade the database
    Upgrade {},
    /// Import Mastodon export tarballs
    Import(import::ImportArgs),
    /// Fetch an ActivityPub outbox URL
    Fetch(fetch::Args),
    /// Build the static site
    Build(build::BuildArgs),
}

pub fn execute() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let config_path = match cli.config.as_deref() {
        Some(path) => path,
        None => &Path::new("./data/config.toml"),
    };

    app::init(config_path)?;

    match &cli.command {
        Commands::Init(args) => init::command(args),
        Commands::Upgrade {} => command_upgrade(),
        Commands::Import(args) => import::command(args),
        Commands::Fetch(args) => fetch::command(args),
        Commands::Build(args) => build::command(args),
    }
}

fn command_upgrade() -> Result<(), Box<dyn Error>> {
    db::upgrade()?;
    Ok(())
}
