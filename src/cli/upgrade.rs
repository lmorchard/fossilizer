use anyhow::Result;
use clap::Args;
use fossilizer::db;

#[derive(Debug, Args)]
pub struct UpgradeArgs {}

pub async fn command(_args: &UpgradeArgs) -> Result<()> {
    db::upgrade()?;
    Ok(())
}
