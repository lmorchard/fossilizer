use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::activitystreams::{Activity, OrderedItems};

// todo: make this configurable?
const IMPORT_TRANSACTION_PAGE_SIZE: usize = 500;

pub struct Activities<'a> {
    conn: &'a Connection,
}

impl<'a> Activities<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn import_activity<T: Serialize>(&self, activity: T) -> Result<()> {
        let json_text = serde_json::to_string_pretty(&activity)?;
        let mut stmt = self
            .conn
            .prepare_cached("INSERT OR REPLACE INTO activities (json) VALUES (?1)")?;
        stmt.execute(params![json_text])?;

        Ok(())
    }

    pub fn import_collection<T: Serialize>(&self, collection: &impl OrderedItems<T>) -> Result<()> {
        let conn = &self.conn;

        // todo: use conn.transaction()?
        conn.execute("BEGIN TRANSACTION", ())?;

        for (count, item) in collection.ordered_items().into_iter().enumerate() {
            if count > 0 && (count % IMPORT_TRANSACTION_PAGE_SIZE) == 0 {
                info!("Imported {:?} items", count);
                conn.execute("COMMIT TRANSACTION", ())?;
                conn.execute("BEGIN TRANSACTION", ())?;
            }
            trace!("Inserting {:?}", count);
            self.import_activity(item)?;
        }

        conn.execute("COMMIT TRANSACTION", ())?;

        Ok(())
    }

    pub fn get_published_years(&self) -> SingleColumnResult {
        query_single_column(
            self.conn,
            r#"
                SELECT publishedYear
                FROM activities
                WHERE isPublic = 1
                GROUP BY publishedYear
            "#,
            [],
        )
    }

    pub fn get_published_months_for_year(&self, year: &String) -> SingleColumnResult {
        query_single_column(
            self.conn,
            r#"
                SELECT publishedYearMonth
                FROM activities
                WHERE publishedYear = ? AND isPublic = 1
                GROUP BY publishedYearMonth
            "#,
            [year],
        )
    }

    pub fn get_published_days_for_month(&self, month: &String) -> SingleColumnResult {
        query_single_column(
            self.conn,
            r#"
                SELECT publishedYearMonthDay
                FROM activities
                WHERE publishedYearMonth = ?1 AND isPublic = 1
                GROUP BY publishedYearMonthDay
            "#,
            [month],
        )
    }

    pub fn get_published_months(&self) -> SingleColumnResult {
        query_single_column(
            self.conn,
            r#"
                SELECT publishedYearMonth
                FROM activities
                WHERE isPublic = 1
                GROUP BY publishedYearMonth
                ORDER BY publishedYearMonth
            "#,
            [],
        )
    }

    pub fn get_published_days(&self) -> Result<Vec<(String, usize)>, rusqlite::Error> {
        let conn = self.conn;
        let mut stmt = conn.prepare_cached(
            r#"
                SELECT publishedYearMonthDay, count(id)
                FROM activities
                WHERE isPublic = 1
                GROUP BY publishedYearMonthDay
                ORDER BY publishedYearMonthDay
            "#,
        )?;
        let res = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<(String, usize)>, _>>()?;

        Ok(res)
    }

    pub fn get_activities_for_day(&self, day: &String) -> Result<Vec<Activity>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare_cached(
            r#"
                SELECT json
                FROM activities
                WHERE publishedYearMonthDay = ?1 AND isPublic = 1
            "#,
        )?;
        let mut rows = stmt.query([day])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            let json_text: String = r.get(0)?;
            match serde_json::from_str(json_text.as_str()) {
                Ok(activity) => out.push(activity),
                Err(e) => {
                    println!("JSON {json_text}");
                    panic!("oof {e:?}");
                }
            }
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
    let mut stmt = conn.prepare_cached(sql)?;
    let result = stmt.query_map(params, |r| r.get(0))?.collect();
    result
}
