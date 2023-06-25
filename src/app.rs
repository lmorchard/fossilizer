use config::Config;
use dotenv;
use serde::Deserialize;
use std::error::Error;
use std::path::Path;
use std::sync::RwLock;

const ENV_PREFIX: &str = "APP";
const DEFAULT_LOG_LEVEL: &str = "info";

lazy_static! {
    static ref CONTEXT: RwLock<AppContext> = RwLock::new(AppContext::default());
}

pub fn init(config_path: Option<&Path>) -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    init_config(config_path)?;
    init_logging()?;
    Ok(())
}

#[derive(Default)]
pub struct AppContext {
    pub raw_config: Config,
}

fn init_config(config_path: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let mut context = CONTEXT.write()?;

    let mut config = Config::builder();

    if let Some(config_path) = config_path {
        let config_path = config_path.to_str().unwrap();
        config = config.add_source(config::File::with_name(config_path));
    }

    config = config.add_source(config::Environment::with_prefix(ENV_PREFIX));
    context.raw_config = config.build().unwrap();

    Ok(())
}

pub fn config_get<'de, T: Deserialize<'de>>(key: &str) -> Result<T, Box<dyn Error>> {
    let context = CONTEXT.read()?;
    let value = context.raw_config.get::<T>(key)?;
    Ok(value)
}

/// Attempt to deserialize the entire configuration into the requested type.
pub fn config_try_deserialize<'de, T: Deserialize<'de>>() -> Result<T, Box<dyn Error>> {
    let context = CONTEXT.read()?;
    let config = context.raw_config.clone();
    let value = config.try_deserialize::<T>()?;
    Ok(value)
}

pub fn init_logging() -> Result<(), Box<dyn Error>> {
    let log_level = config_get::<String>("log_level").unwrap_or(DEFAULT_LOG_LEVEL.to_string());
    let log_env = env_logger::Env::default().default_filter_or(log_level);
    env_logger::Builder::from_env(log_env).init();

    Ok(())
}
