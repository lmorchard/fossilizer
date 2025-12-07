use crate::cli::mastodon::Args;
use anyhow::Result;
use fossilizer::mastodon::{
    instance::{register_client_app, InstanceConfig},
    OAUTH_SCOPES, REDIRECT_URI_OOB,
};
use std::error::Error;

#[derive(Debug, clap::Args)]
pub struct LinkArgs {}

pub async fn command(
    _args: &LinkArgs,
    parent_args: &Args,
    instance_config: &mut InstanceConfig,
) -> Result<(), Box<dyn Error>> {
    let instance = &parent_args.instance;

    if instance_config.client_id.is_none() {
        register_client_app(instance, instance_config).await?;
    }

    let base_url = format!("https://{instance}/oauth/authorize");
    let client_id = instance_config.client_id.as_ref().unwrap();
    let params = [
        ("client_id", client_id.as_str()),
        ("scope", OAUTH_SCOPES),
        ("redirect_uri", REDIRECT_URI_OOB),
        ("response_type", "code"),
    ];
    let link = reqwest::Url::parse_with_params(&base_url, &params)?;

    info!("Visit this link to begin authorization:");
    info!("{link}");
    Ok(())
}
