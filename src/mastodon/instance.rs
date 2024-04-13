use crate::config;

use anyhow::Result;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use std::error::Error;
use std::fs;

use std::io::prelude::*;

use std::path::PathBuf;

use crate::mastodon::{CLIENT_NAME, CLIENT_WEBSITE, OAUTH_SCOPES, REDIRECT_URI_OOB};

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct InstanceConfig {
    pub host: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub vapid_key: Option<String>,
    pub access_token: Option<String>,
    pub created_at: Option<u32>,
}
impl InstanceConfig {
    pub fn new(instance: &String) -> Self {
        InstanceConfig {
            host: instance.clone(),
            ..Default::default()
        }
    }
}

fn build_instance_config_path(instance: &String) -> Result<PathBuf, Box<dyn Error>> {
    let config = config::config()?;
    let data_path = config.data_path;
    // todo: hash the instance domain string rather than using it verbatim?
    Ok(data_path.join(format!("config-instance-{instance}.toml")))
}

pub fn load_instance_config(instance: &String) -> Result<InstanceConfig, Box<dyn Error>> {
    let config_path = build_instance_config_path(instance)?;
    trace!(
        "Loading {} instance config file from {:?}",
        instance,
        config_path
    );
    if config_path.exists() {
        let instance_config_file = fs::read_to_string(config_path)?;
        Ok(toml::from_str(instance_config_file.as_str())?)
    } else {
        Ok(InstanceConfig::new(instance))
    }
}

pub fn save_instance_config(
    instance: &String,
    instance_config: &InstanceConfig,
) -> Result<(), Box<dyn Error>> {
    let config_path = build_instance_config_path(instance)?;
    trace!(
        "Saving {} instance config file to {:?}",
        instance,
        config_path
    );
    let instance_config_str = toml::to_string_pretty(&instance_config)?;
    let mut file = fs::File::create(config_path)?;
    file.write_all(instance_config_str.as_bytes())?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct AppRegistrationResult {
    client_id: String,
    client_secret: String,
    vapid_key: String,
}

pub async fn register_client_app(
    instance: &String,
    instance_config: &mut InstanceConfig,
) -> Result<()> {
    let mut params = HashMap::new();
    params.insert("client_name", CLIENT_NAME);
    params.insert("website", CLIENT_WEBSITE);
    params.insert("redirect_uris", REDIRECT_URI_OOB);
    params.insert("scopes", OAUTH_SCOPES);

    let url = format!("https://{instance}/api/v1/apps");
    let client = reqwest::ClientBuilder::new().build().unwrap();
    let res = client.post(url).json(&params).send().await?;

    debug!("Registering new app with instance {}", instance);

    if res.status() == reqwest::StatusCode::OK {
        let result: AppRegistrationResult = res.json().await?;
        instance_config.client_id = Some(result.client_id);
        instance_config.client_secret = Some(result.client_secret);
        instance_config.vapid_key = Some(result.vapid_key);
        Ok(())
    } else {
        // todo: throw an error here
        error!("Failed to register app");
        Ok(())
    }
}
