use anyhow::Result;
use rusqlite::Connection;
use std::error::Error;
use rusqlite::params;

use crate::activitystreams::{Activity, Outbox};

// todo: make this configurable?
const IMPORT_TRANSACTION_PAGE_SIZE: usize = 500;

pub struct Activities {
    conn: Connection,
}

impl Activities {
    pub fn new(conn: Connection) -> Self {
        Activities { conn }
    }

    pub fn import_activity(&self, activity: Activity) -> Result<(), Box<dyn Error>> {
        let json_text = serde_json::to_string_pretty(&activity)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO activities (json) VALUES (?1)",
            params![json_text],
        )?;

        Ok(())
    }

    pub fn import_outbox(&self, outbox: Outbox<Activity>) -> Result<(), Box<dyn Error>> {
        let conn = &self.conn;

        conn.execute("BEGIN TRANSACTION", ())?;

        for (count, item) in outbox.ordered_items.into_iter().enumerate() {
            if count > 0 && (count % IMPORT_TRANSACTION_PAGE_SIZE) == 0 {
                info!("Imported {:?} items", count);                
                conn.execute("COMMIT TRANSACTION", ())?;
                conn.execute("BEGIN TRANSACTION", ())?;
            }
            debug!("Inserting {:?}", count);
            self.import_activity(item)?;
        }

        conn.execute("COMMIT TRANSACTION", ())?;

        Ok(())
    }
}
