use anyhow::Result;
use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;
use std::path::{Path,PathBuf};

use fossilizer::{app, db};

pub mod build;
pub mod import;
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
    Init {
        /// Delete any existing data directory before initializing
        #[arg(short = 'k', long)]
        clean: bool,
        /// Prepare the data directory with resources for customization
        #[arg(short, long)]
        customize: bool,
    },
    /// Upgrade the database
    Upgrade {},
    /// Adds files to myapp
    Import { filenames: Vec<String> },
    /// Build the static site
    Build {
        #[arg(short = 'k', long)]
        clean: bool,
    },
}

pub fn execute() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let config_path = match cli.config.as_deref() {
        Some(path) => path,
        None => &Path::new("./data/config.toml"),
    };

    app::init(config_path)?;

    match &cli.command {
        Commands::Init { clean, customize } => init::command(&clean, &customize),
        Commands::Upgrade {} => command_upgrade(),
        Commands::Import { filenames } => import::command_import(filenames),
        Commands::Build { clean } => build::command_build(&clean),
    }
}

fn command_upgrade() -> Result<(), Box<dyn Error>> {
    db::upgrade()?;
    Ok(())
}
