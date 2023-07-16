use anyhow::Result;
use clap::Args;
use fossilizer::db;
use std::error::Error;

#[derive(Debug, Args)]
pub struct UpgradeArgs {}

pub async fn command(_args: &UpgradeArgs) -> Result<(), Box<dyn Error>> {
    db::upgrade()?;
    Ok(())
}
