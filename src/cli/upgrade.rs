use anyhow::Result;
use clap::Args;
use std::error::Error;
use fossilizer::db;

#[derive(Debug, Args)]
pub struct UpgradeArgs {
}

pub async fn command(_args: &UpgradeArgs) -> Result<(), Box<dyn Error>> {
    db::upgrade()?;
    Ok(())
}
