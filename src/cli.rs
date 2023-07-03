use anyhow::Result;
use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;
use std::path::PathBuf;

use fossilizer::{app, db};

pub mod build;
pub mod import;

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
    /// Initialize the database
    Init {},
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

    app::init(cli.config.as_deref())?;

    // todo: come up with a more useful way to report status after subcommand
    match &cli.command {
        Commands::Init {} => info!("INIT {:?}", command_init()),
        Commands::Upgrade {} => info!("UPGRADE {:?}", command_upgrade()),
        Commands::Import { filenames } => info!("IMPORT {:?}", import::command_import(filenames)),
        Commands::Build { clean } => info!("BUILD {:?}", build::command_build(&clean)),
    };

    Ok(())
}

fn command_init() -> Result<(), Box<dyn Error>> {
    // todo: remove existing DB?
    db::upgrade()?;
    Ok(())
}

fn command_upgrade() -> Result<(), Box<dyn Error>> {
    db::upgrade()?;
    Ok(())
}
