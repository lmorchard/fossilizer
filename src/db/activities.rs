use anyhow::Result;
use rusqlite::{params, Connection, Rows};

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

    pub fn import_activity(&self, activity: Activity) -> Result<()> {
        let json_text = serde_json::to_string_pretty(&activity)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO activities (json) VALUES (?1)",
            params![json_text],
        )?;

        Ok(())
    }

    pub fn import_outbox(&self, outbox: Outbox<Activity>) -> Result<()> {
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

    pub fn get_published_years(&self) -> Result<Vec<String>> {
        let stmt = &mut self.conn.prepare(
            r#"
                SELECT publishedYear
                FROM activities
                GROUP BY publishedYear
            "#,
        )?;
        collect_first_column(stmt.query([]))
    }

    pub fn get_published_months_for_year(&self, year: String) -> Result<Vec<String>> {
        let stmt = &mut self.conn.prepare(
            r#"
                SELECT publishedYearMonth
                FROM activities
                WHERE publishedYear = ?1
                GROUP BY publishedYearMonth
            "#,
        )?;
        collect_first_column(stmt.query([year]))
    }

    pub fn get_published_days_for_month(&self, month: String) -> Result<Vec<String>> {
        let stmt = &mut self.conn.prepare(
            r#"
                SELECT publishedYearMonthDay
                FROM activities
                WHERE publishedYearMonth = ?1
                GROUP BY publishedYearMonthDay
            "#,
        )?;
        collect_first_column(stmt.query([month]))
    }
}

fn collect_first_column(rows: std::result::Result<Rows, rusqlite::Error>) -> Result<Vec<String>> {
    let mut rows = rows?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(row.get(0)?);
    }
    Ok(out)
}
