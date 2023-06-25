use anyhow::Result;
use lazy_static::lazy_static;
use rusqlite::{params, Connection};
use rusqlite_migration::{Migrations, M};
use std::error::Error;

pub fn init() -> Result<Connection, Box<dyn Error>> {
    let mut conn = Connection::open("./data.sqlite3")?;

    // Update the database schema, atomically
    MIGRATIONS.to_latest(&mut conn)?;
    conn.pragma_update(None, "journal_mode", "WAL").unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();

    Ok(conn)
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
