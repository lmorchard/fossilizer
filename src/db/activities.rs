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

        // todo: use conn.transaction()?
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

    pub fn get_published_years(&self) -> SingleColumnResult {
        query_single_column(
            &self.conn,
            r#"
                SELECT publishedYear
                FROM activities
                GROUP BY publishedYear
            "#,
            [],
        )
    }

    pub fn get_published_months_for_year(&self, year: String) -> SingleColumnResult {
        query_single_column(
            &self.conn,
            r#"
                SELECT publishedYearMonth
                FROM activities
                WHERE publishedYear = ?1
                GROUP BY publishedYearMonth
            "#,
            [year],
        )
    }

    pub fn get_published_days_for_month(&self, month: String) -> SingleColumnResult {
        query_single_column(
            &self.conn,
            r#"
                SELECT publishedYearMonthDay
                FROM activities
                WHERE publishedYearMonth = ?1
                GROUP BY publishedYearMonthDay
            "#,
            [month],
        )
    }

    pub fn get_activities_for_day(&self, day: String) -> Result<Vec<Activity>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare(
            r#"
                SELECT json
                FROM activities
                WHERE publishedYearMonthDay = ?1
            "#            
        )?;
        let mut rows = stmt.query([day])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            let json_text: String = r.get(0)?;
            let activity: Activity = serde_json::from_str(json_text.as_str())?;
            out.push(activity);
        }
        Ok(out)
    }
}

type SingleColumnResult = Result<Vec<String>, rusqlite::Error>;

fn query_single_column<P>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<Vec<String>, rusqlite::Error>
where
    P: rusqlite::Params,
{
    let mut stmt = conn.prepare(sql)?;
    let result = stmt.query_map(params, |r| r.get(0))?.collect();
    result
}
