use crate::cli::mastodon::Args;
use anyhow::Result;
use fossilizer::{db, mastodon::instance::InstanceConfig};

use std::error::Error;

use fossilizer::{config, mastodon::fetcher::Fetcher};

#[derive(Debug, clap::Args)]
pub struct FetchArgs {
    /// Number of statuses fetched per page
    #[arg(long, short = 'p', default_value = "25")]
    page: u32,
    /// Maximum number of statuses to fetch overall
    #[arg(long, short = 'm', default_value = "100")]
    max: u32,
    /// Stop fetching once a page includes statuses already in the database
    #[arg(long, short = 'n', default_value = "false")]
    incremental: bool,
}

pub async fn command(
    args: &FetchArgs,
    parent_args: &Args,
    instance_config: &mut InstanceConfig,
) -> Result<(), Box<dyn Error>> {
    let max = args.max;
    let page = args.page;
    let incremental: bool = args.incremental;

    let config = config::config()?;
    let media_path = config.media_path();

    let mut fetcher = Fetcher::new(
        db::conn()?,
        parent_args.instance.clone(),
        instance_config.clone(),
        media_path.clone(),
        page,
        max,
        incremental,
    );

    fetcher.fetch().await?;

    Ok(())
}
