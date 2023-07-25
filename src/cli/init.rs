use anyhow::Result;
use clap::Args;

use std::error::Error;

use fossilizer::{db, site_generator};

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Delete any existing data directory before initializing
    #[arg(short = 'k', long)]
    clean: bool,
    /// Prepare the data directory with resources for customization
    #[arg(short, long)]
    customize: bool,
}

pub async fn command(args: &InitArgs) -> Result<(), Box<dyn Error>> {
    site_generator::setup_data_path(&args.clean)?;
    db::upgrade()?;
    if args.customize {
        site_generator::unpack_customizable_resources()?;
    }
    Ok(())
}
