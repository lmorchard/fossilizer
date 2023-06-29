use anyhow::Result;
use lazy_static::lazy_static;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use std::fs;

pub mod activities;
pub mod actors;

use crate::app;

lazy_static! {
    // TODO: iterate through directory using rust_embed?
    static ref MIGRATIONS: Migrations<'static> = Migrations::new(vec![
        M::up(include_str!("./db/migrations/202306241304-init.sql")),
        M::up(include_str!(
            "./db/migrations/202306261338-object-type-and-indexes-up.sql"
        )),
        M::up(include_str!("./db/migrations/202306262036-actors-up.sql")),
    ]);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_path")]
    pub database_path: String,
}

fn default_database_path() -> String {
    "./data/data.sqlite3".to_string()
}

pub fn config() -> Result<DatabaseConfig, Box<dyn Error>> {
    app::config_try_deserialize::<DatabaseConfig>()
}

pub fn conn() -> Result<Connection, Box<dyn Error>> {
    let config = config()?;

    let database_path = Path::new(&config.database_path);
    let database_parent_path = database_path.parent().ok_or("no parent path")?;
    trace!("database path {:?}", database_path);
    fs::create_dir_all(&database_parent_path).unwrap();

    let conn = Connection::open(database_path)?;
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    trace!("database connection opened {:?}", database_path);
    Ok(conn)
}

pub fn upgrade() -> Result<(), Box<dyn Error>> {
    let config = config()?;

    let mut conn = Connection::open(config.database_path)?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(())
}
