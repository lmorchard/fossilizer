use anyhow::Result;

use crate::cli::mastodon::Args;
use fossilizer::mastodon::{instance::InstanceConfig, OAUTH_SCOPES, REDIRECT_URI_OOB};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
struct CodeAuthResult {
    access_token: String,
    created_at: u32,
}

#[derive(Debug, clap::Args)]
pub struct CodeArgs {
    /// Authorization code given by Mastodon authorization process
    #[arg()]
    code: String,
}

pub async fn command(
    args: &CodeArgs,
    parent_args: &Args,
    instance_config: &mut InstanceConfig,
) -> Result<(), Box<dyn Error>> {
    let instance = &parent_args.instance;
    let code = &args.code;

    if instance_config.client_id.is_none() {
        // todo: throw an error if no client has been registered yet
        return Ok(());
    }

    let mut params = HashMap::new();
    params.insert("scopes", OAUTH_SCOPES);
    params.insert("redirect_uri", REDIRECT_URI_OOB);
    params.insert("grant_type", "authorization_code");
    params.insert("code", code);

    let client_id = instance_config.client_id.as_ref().unwrap();
    params.insert("client_id", client_id.as_str());

    let client_secret = instance_config.client_secret.as_ref().unwrap();
    params.insert("client_secret", client_secret.as_str());

    let url = format!("https://{instance}/oauth/token");
    let client = reqwest::ClientBuilder::new().build().unwrap();
    let res = client.post(url).json(&params).send().await?;

    if res.status() == reqwest::StatusCode::OK {
        let result: CodeAuthResult = res.json().await?;
        instance_config.access_token = Some(result.access_token);
        instance_config.created_at = Some(result.created_at);
        println!("CODE {} {:?}", args.code, instance_config.access_token);
        Ok(())
    } else {
        // todo: throw an error here
        error!("Failed to authorize with code");
        Ok(())
    }
}
