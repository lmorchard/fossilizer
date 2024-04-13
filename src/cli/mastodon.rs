use anyhow::Result;
use clap::Subcommand;
use fossilizer::mastodon::instance::{load_instance_config, save_instance_config};
use std::error::Error;

mod code;
mod fetch;
mod link;
mod verify;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Host name of Mastodon instance
    #[arg(long, short = 'i', default_value = "mastodon.social")]
    instance: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Get a link to begin Mastodon authorization process
    Link(link::LinkArgs),
    /// Complete Mastodon authorization process with a code
    Code(code::CodeArgs),
    /// Verify authorized Mastodon account
    Verify(verify::VerifyArgs),
    /// Fetch new statuses from Mastodon account
    Fetch(fetch::FetchArgs),
}

pub async fn command(args: &Args) -> Result<(), Box<dyn Error>> {
    let instance = args.instance.clone();
    let mut instance_config = load_instance_config(&instance)?;
    match &args.command {
        Commands::Link(subcommand_args) => {
            link::command(subcommand_args, args, &mut instance_config).await?
        }
        Commands::Code(subcommand_args) => {
            code::command(subcommand_args, args, &mut instance_config).await?
        }
        Commands::Verify(subcommand_args) => {
            verify::command(subcommand_args, args, &mut instance_config).await?
        }
        Commands::Fetch(subcommand_args) => {
            fetch::command(subcommand_args, args, &mut instance_config).await?
        }
    }
    save_instance_config(&instance, &instance_config)?;
    Ok(())
}
