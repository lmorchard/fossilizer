use anyhow::Result;
use lazy_static::lazy_static;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::error::Error;
use std::fs;
use std::path::Path;

pub mod activities;
pub mod actors;

use crate::config;

lazy_static! {
    // TODO: iterate through directory using rust_embed?
    static ref MIGRATIONS: Migrations<'static> = Migrations::new(vec![
        M::up(include_str!("./db/migrations/202306241304-init.sql")),
        M::up(include_str!(
            "./db/migrations/202306261338-object-type-and-indexes-up.sql"
        )),
        M::up(include_str!("./db/migrations/202306262036-actors-up.sql")),
        M::up(include_str!("./db/migrations/202307021314-ispublic-up.sql")),
        M::up(include_str!("./db/migrations/202307021325-index-ispublic-up.sql")),
    ]);
}

pub fn conn() -> Result<Connection, Box<dyn Error>> {
    let config = config::config()?;

    let database_path = config.database_path();
    let database_parent_path = Path::new(&database_path).parent().ok_or("no parent path")?;
    trace!("database path {:?}", database_path);
    fs::create_dir_all(&database_parent_path).unwrap();

    let conn = Connection::open(&database_path)?;
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    trace!("database connection opened {:?}", database_path);
    Ok(conn)
}

pub fn upgrade() -> Result<(), Box<dyn Error>> {
    let config = config::config()?;

    let mut conn = Connection::open(config.database_path())?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(())
}
