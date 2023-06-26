use anyhow::Result;
use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;
use std::path::PathBuf;

use ap_fossilizer::{app, db, mastodon};

pub mod build;

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
    Build {},
}

pub fn execute() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    app::init(cli.config.as_deref())?;

    // todo: come up with a more useful way to report status after subcommand
    match &cli.command {
        Commands::Init {} => info!("INIT {:?}", command_init()),
        Commands::Upgrade {} => info!("UPGRADE {:?}", command_upgrade()),
        Commands::Import { filenames } => info!("IMPORT {:?}", command_import(filenames)),
        Commands::Build {} => info!("BUILD {:?}", build::command_build()),
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

fn command_import(filenames: &Vec<String>) -> Result<(), Box<dyn Error>> {
    for filename in filenames {
        info!("Importing {:?}", filename);

        let mut export = mastodon::Export::from(filename);
        let outbox = export.outbox()?;

        info!("Found {:?} items", outbox.ordered_items.len());

        let conn = db::conn()?;
        let activities = db::activities::Activities::new(conn);
        activities.import_outbox(outbox)?;

        debug!("Imported {:?}", filename);
    }
    info!("Done");

    Ok(())
}
