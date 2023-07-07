use std::error::Error;
use std::path::Path;

const DEFAULT_LOG_LEVEL: &str = "info";

use crate::config;

pub fn init(config_path: &Path) -> Result<(), Box<dyn Error>> {
    config::init(config_path)?;
    init_logging()?;
    Ok(())
}

pub fn init_logging() -> Result<(), Box<dyn Error>> {
    let log_level =
        config::get::<String>("log_level").unwrap_or(DEFAULT_LOG_LEVEL.to_string());
    let log_env = env_logger::Env::default().default_filter_or(log_level);
    env_logger::Builder::from_env(log_env).init();

    Ok(())
}
