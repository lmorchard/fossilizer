use anyhow::Result;

use serde::{Deserialize, Serialize};

use crate::cli::mastodon::Args;
use fossilizer::mastodon::instance::InstanceConfig;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
struct AuthVerifyResult {
    username: String,
    url: String,
    display_name: String,
    created_at: String,
}

#[derive(Debug, clap::Args)]
pub struct VerifyArgs {}

pub async fn command(
    _args: &VerifyArgs,
    parent_args: &Args,
    instance_config: &mut InstanceConfig,
) -> Result<(), Box<dyn Error>> {
    let instance = &parent_args.instance;

    if instance_config.access_token.is_none() {
        // todo: throw error if no access_token has been acquired
        return Ok(());
    }

    let access_token = instance_config.access_token.as_ref().unwrap();
    let url = format!("https://{instance}/api/v1/accounts/verify_credentials");
    let client = reqwest::ClientBuilder::new().build().unwrap();
    let res = client
        .get(url)
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await?;

    if res.status() == reqwest::StatusCode::OK {
        let result: AuthVerifyResult = res.json().await?;
        info!("Verified as {:?}", result);
        Ok(())
    } else {
        // todo: throw an error here
        error!("Failed to verify authorized user");
        Ok(())
    }
}
