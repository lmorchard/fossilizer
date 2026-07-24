use anyhow::{anyhow, Result};
use config::Config;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

const ENV_PREFIX: &str = "APP";

lazy_static! {
    static ref CONTEXT: RwLock<AppContext> = RwLock::new(AppContext::default());
    pub static ref DEFAULT_CONFIG: String =
        include_str!("./resources/default_config.toml").to_string();
}

#[derive(Default, Clone)]
pub struct AppContext {
    pub raw_config: Config,
    pub config: AppConfig,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_build_path")]
    pub build_path: PathBuf,

    #[serde(default = "default_data_path")]
    pub data_path: PathBuf,

    /// Location for downloaded media. Defaults to `<data_path>/media` when
    /// unset. Override with `APP_MEDIA_PATH`. Kept separate from `build_path`
    /// so media is durable state, not regeneratable build output.
    pub media_path: Option<PathBuf>,

    #[serde(default = "default_theme")]
    pub theme: String,

    pub mastodon_access_token: Option<String>,
}
pub fn default_build_path() -> PathBuf {
    PathBuf::from(".").join("build")
}
pub fn default_data_path() -> PathBuf {
    PathBuf::from(".").join("data")
}
pub fn default_theme() -> String {
    "default".into()
}
impl AppConfig {
    // todo: allow each of these to be individually overriden
    pub fn media_path(&self) -> PathBuf {
        self.media_path
            .clone()
            .unwrap_or_else(|| self.data_path.join("media"))
    }
    pub fn database_path(&self) -> PathBuf {
        self.data_path.join("data.sqlite3")
    }
    pub fn config_path(&self) -> PathBuf {
        self.data_path.join("config.toml")
    }
    pub fn themes_path(&self) -> PathBuf {
        self.data_path.join("themes")
    }
    pub fn templates_path(&self) -> PathBuf {
        self.themes_path().join(&self.theme).join("templates")
    }
    pub fn web_assets_path(&self) -> PathBuf {
        self.themes_path().join(&self.theme).join("web")
    }
}

pub fn init(config_path: &Path) -> Result<()> {
    dotenvy::dotenv().ok();

    let mut config = Config::builder();
    if config_path.is_file() {
        config = config.add_source(config::File::from(config_path));
    }
    config = config.add_source(config::Environment::with_prefix(ENV_PREFIX));

    let config = config.build()?;

    let mut context = CONTEXT
        .write()
        .map_err(|e| anyhow!("config context lock poisoned: {e}"))?;
    context.raw_config = config.clone();
    context.config = config.try_deserialize()?;

    Ok(())
}

pub fn config() -> Result<AppConfig> {
    Ok(CONTEXT
        .read()
        .map_err(|e| anyhow!("config context lock poisoned: {e}"))?
        .config
        .clone())
}

pub fn update<U>(updater: U) -> Result<()>
where
    U: FnOnce(&mut AppConfig),
{
    let mut context = CONTEXT
        .write()
        .map_err(|e| anyhow!("config context lock poisoned: {e}"))?;
    updater(&mut context.config);
    Ok(())
}

pub fn get<'de, T: Deserialize<'de>>(key: &str) -> Result<T> {
    let context = CONTEXT
        .read()
        .map_err(|e| anyhow!("config context lock poisoned: {e}"))?;
    let value = context.raw_config.get::<T>(key)?;
    Ok(value)
}

/// Attempt to deserialize the entire configuration into the requested type.
pub fn try_deserialize<'de, T: Deserialize<'de>>() -> Result<T> {
    let context = CONTEXT
        .read()
        .map_err(|e| anyhow!("config context lock poisoned: {e}"))?;
    let config = context.raw_config.clone();
    let value = config.try_deserialize::<T>()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_path_defaults_to_data_path_media() {
        let config = AppConfig {
            data_path: PathBuf::from("/tmp/somedata"),
            ..Default::default()
        };
        assert_eq!(config.media_path(), PathBuf::from("/tmp/somedata/media"));
    }

    #[test]
    fn media_path_uses_explicit_override() {
        let config = AppConfig {
            data_path: PathBuf::from("/tmp/somedata"),
            media_path: Some(PathBuf::from("/mnt/bigdisk/media")),
            ..Default::default()
        };
        assert_eq!(config.media_path(), PathBuf::from("/mnt/bigdisk/media"));
    }
}
