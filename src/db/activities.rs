use anyhow::Result;
use rusqlite::{params, Connection};
use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use std::string::ToString;

use crate::activitystreams::{Activity, OrderedItems};

// todo: make this configurable?
const IMPORT_TRANSACTION_PAGE_SIZE: usize = 500;

const ACTIVITYSCHEMA_ACTIVITY: &str = "fossilizer::activitystreams::Activity";
const ACTIVITYSCHEMA_STATUS: &str = "megalodon::entities::Status";

#[derive(Default, Debug, Clone, PartialEq)]
pub enum ActivitySchema {
    #[default]
    Activity,
    Status,
    Unknown(String),
}
impl FromStr for ActivitySchema {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            ACTIVITYSCHEMA_ACTIVITY => ActivitySchema::Activity,
            ACTIVITYSCHEMA_STATUS => ActivitySchema::Status,
            _ => ActivitySchema::Unknown(s.to_string()),
        })
    }
}
impl ToString for ActivitySchema {
    fn to_string(&self) -> String {
        match self {
            ActivitySchema::Activity => ACTIVITYSCHEMA_ACTIVITY.to_string(),
            ActivitySchema::Status => ACTIVITYSCHEMA_STATUS.to_string(),
            ActivitySchema::Unknown(s) => s.clone(),
        }
    }
}
pub trait WhichActivitySchema {
    fn which_activity_schema(&self) -> ActivitySchema;
}
impl WhichActivitySchema for megalodon::entities::Status {
    fn which_activity_schema(&self) -> ActivitySchema {
        ActivitySchema::Status
    }
}
impl WhichActivitySchema for crate::activitystreams::Activity {
    fn which_activity_schema(&self) -> ActivitySchema {
        ActivitySchema::Activity
    }
}

pub struct Activities<'a> {
    conn: &'a Connection,
}

impl<'a> Activities<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn import<T: Serialize + WhichActivitySchema>(&self, item: &T) -> Result<()> {
        let schema = item.which_activity_schema().to_string();
        let json_text = serde_json::to_string_pretty(&item)?;
        let mut stmt = self.conn.prepare_cached(
            r#"
                INSERT OR REPLACE INTO activities
                (schema, json)
                VALUES
                (?1, ?2)                
            "#,
        )?;
        stmt.execute(params![schema, json_text])?;

        Ok(())
    }

    pub fn import_many<T: Serialize + WhichActivitySchema>(
        &self,
        activities: &Vec<T>,
    ) -> Result<()> {
        let conn = self.conn;

        // todo: use conn.transaction()?
        conn.execute("BEGIN TRANSACTION", ())?;

        for (count, item) in activities.iter().enumerate() {
            if count > 0 && (count % IMPORT_TRANSACTION_PAGE_SIZE) == 0 {
                info!("Imported {:?} items", count);
                conn.execute("COMMIT TRANSACTION", ())?;
                conn.execute("BEGIN TRANSACTION", ())?;
            }
            trace!("Inserting {:?}", count);
            self.import(item)?;
        }

        conn.execute("COMMIT TRANSACTION", ())?;

        Ok(())
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
        let conn = self.conn;

        // todo: use conn.transaction()?
        conn.execute("BEGIN TRANSACTION", ())?;

        for (count, item) in collection.ordered_items().iter().enumerate() {
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
        let conn = self.conn;
        let mut stmt = conn.prepare_cached(
            r#"
                SELECT json, schema
                FROM activities
                WHERE publishedYearMonthDay = ?1 AND isPublic = 1
            "#,
        )?;

        let mut rows = stmt.query([day])?;
        let mut out = Vec::new();

        while let Some(r) = rows.next()? {
            let json_data: String = r.get(0)?;
            let schema_str: String = r.get(1)?;
            
            match schema_str.parse::<ActivitySchema>()? {
                ActivitySchema::Activity => {
                    let activity: Activity = serde_json::from_str::<Activity>(&json_data)?;
                    out.push(activity);
                },
                ActivitySchema::Status => {
                    let status: megalodon::entities::Status = serde_json::from_str(&json_data)?;
                    let activity: Activity = status.into();
                    out.push(activity);
                }
                ActivitySchema::Unknown(_) => {
                    trace!("unknown schema {:?}", schema_str);
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

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_activityschema_serde() -> Result<()> {
        let cases = vec![
            (ActivitySchema::Activity, ACTIVITYSCHEMA_ACTIVITY),
            (ActivitySchema::Status, ACTIVITYSCHEMA_STATUS),
            (
                ActivitySchema::Unknown(String::from("lolbutts")),
                "lolbutts",
            ),
        ];
        for (expected_schema, expected_str) in cases {
            assert_eq!(expected_schema.to_string(), expected_str);
            let result_schema: ActivitySchema = expected_str.parse()?;
            assert_eq!(result_schema, expected_schema);
        }
        Ok(())
    }
}
