use anyhow::Result;
use lazy_static::lazy_static;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use std::error::Error;

use crate::app;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_path")]
    pub database_path: String,
}

fn default_database_path() -> String {
    "./data.sqlite3".to_string()
}

pub fn upgrade() -> Result<(), Box<dyn Error>> {
    let config = config()?;

    let mut conn = Connection::open(config.database_path)?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(())
}

pub fn conn() -> Result<Connection, Box<dyn Error>> {
    let config = config()?;
    debug!("database path {:?}", config.database_path);
    let conn = Connection::open(config.database_path)?;
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    Ok(conn)
}

pub fn config() -> Result<DatabaseConfig, Box<dyn Error>> {
    app::config_try_deserialize::<DatabaseConfig>()
}

// Define migrations. These are applied atomically.
lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up(include_str!("./resources/migrations/202306241304-init.sql")),
            /*
            // PRAGMA are better applied outside of migrations, see below for details.
            M::up(r#"
                  ALTER TABLE friend ADD COLUMN birthday TEXT;
                  ALTER TABLE friend ADD COLUMN comment TEXT;
                  "#),

            // This migration can be reverted
            M::up("CREATE TABLE animal(name TEXT);")
            .down("DROP TABLE animal;")
            */

            // In the future, if the need to change the schema arises, put
            // migrations here, like so:
            // M::up("CREATE INDEX UX_friend_email ON friend(email);"),
            // M::up("CREATE INDEX UX_friend_name ON friend(name);"),
        ]);
}
