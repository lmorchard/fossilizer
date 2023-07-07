use config::Config;
use dotenv;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

const ENV_PREFIX: &str = "APP";

lazy_static! {
    static ref CONTEXT: RwLock<AppContext> = RwLock::new(AppContext::default());
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
}
pub fn default_build_path() -> PathBuf {
    "./build".into()
}
pub fn default_data_path() -> PathBuf {
    "./data".into()
}
impl AppConfig {
    pub fn media_path(&self) -> PathBuf {
        // todo: allow this to be individually overridden
        self.data_path.join("media")
    }
    pub fn database_path(&self) -> PathBuf {
        // todo: allow this to be individually overridden
        self.data_path.join("data.sqlite3")
    }
}

pub fn init(config_path: &Path) -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    let mut config = Config::builder();
    if config_path.is_file() {
        config = config.add_source(config::File::from(config_path));
    }
    config = config.add_source(config::Environment::with_prefix(ENV_PREFIX));

    let config = config.build().unwrap();

    let mut context = CONTEXT.write()?;
    context.raw_config = config.clone();
    context.config = config.try_deserialize().unwrap();

    Ok(())
}

pub fn config() -> Result<AppConfig, Box<dyn Error>> {
    Ok(CONTEXT.read()?.config.clone())
}

pub fn get<'de, T: Deserialize<'de>>(key: &str) -> Result<T, Box<dyn Error>> {
    let context = CONTEXT.read()?;
    let value = context.raw_config.get::<T>(key)?;
    Ok(value)
}

/// Attempt to deserialize the entire configuration into the requested type.
pub fn try_deserialize<'de, T: Deserialize<'de>>() -> Result<T, Box<dyn Error>> {
    let context = CONTEXT.read()?;
    let config = context.raw_config.clone();
    let value = config.try_deserialize::<T>()?;
    Ok(value)
}
