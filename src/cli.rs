use anyhow::Result;
use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;
use std::path::{Path, PathBuf};

use fossilizer::app;

pub mod build;
pub mod import;
pub mod init;
pub mod upgrade;

#[cfg(feature = "fetch_outbox")]
pub mod fetch;

#[cfg(feature = "fetch_mastodon")]
pub mod fetch_mastodon;

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
    Upgrade(upgrade::UpgradeArgs),
    /// Import Mastodon export tarballs
    Import(import::ImportArgs),
    /// Build the static site
    Build(build::BuildArgs),
    /// Fetch an ActivityPub outbox URL
    #[cfg(feature = "fetch_outbox")]
    Fetch(fetch::Args),
    /// Fetch from a Mastodon API endpoint
    #[cfg(feature = "fetch_mastodon")]
    FetchMastodon(fetch_mastodon::Args),
}

pub async fn execute() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let config_path = match cli.config.as_deref() {
        Some(path) => path,
        None => Path::new("./data/config.toml"),
    };

    app::init(config_path)?;

    match &cli.command {
        Commands::Init(args) => init::command(args).await,
        Commands::Upgrade(args) => upgrade::command(args).await,
        Commands::Import(args) => import::command(args).await,
        Commands::Build(args) => build::command(args).await,
        #[cfg(feature = "fetch_outbox")]
        Commands::Fetch(args) => fetch::command(args).await,
        #[cfg(feature = "fetch_mastodon")]
        Commands::FetchMastodon(args) => fetch_mastodon::command(args).await,
    }
}
