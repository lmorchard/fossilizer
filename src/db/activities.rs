use crate::activitystreams::{Activity, OrderedItems};
use anyhow::anyhow;
use anyhow::Result;
use megalodon::entities::Status;
use rusqlite::types::Value;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::string::ToString;
use std::{rc::Rc, str::FromStr};

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

    pub fn import_many<T: Serialize + WhichActivitySchema>(&self, activities: &[T]) -> Result<()> {
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
        query_activities(
            self.conn,
            r#"
                SELECT json, schema
                FROM activities
                WHERE publishedYearMonthDay = ?1 AND isPublic = 1
                ORDER BY published ASC
            "#,
            [day],
        )
    }

    pub fn get_activities_by_ids(&self, ids: &Vec<String>) -> Result<Vec<Activity>> {
        query_activities(
            self.conn,
            r#"
                SELECT json, schema
                FROM activities
                WHERE id IN rarray(?1)
                ORDER BY published ASC
            "#,
            [ids_to_rarray_param(ids)],
        )
    }

    pub fn count_activities_by_ids(&self, ids: &Vec<String>) -> Result<i16> {
        query_count(
            self.conn,
            r#"
                SELECT COUNT(id)
                FROM activities
                WHERE id IN rarray(?1)
                ORDER BY published ASC
            "#,
            [ids_to_rarray_param(ids)],
        )
    }
}

// todo: move these query utilities into a separate module?

type SingleColumnResult = Result<Vec<String>, rusqlite::Error>;

fn ids_to_rarray_param(ids: &Vec<String>) -> Rc<Vec<rusqlite::types::Value>> {
    Rc::new(ids.iter().cloned().map(Value::from).collect::<Vec<Value>>())
}

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

fn query_count<P>(conn: &Connection, sql: &str, params: P) -> Result<i16>
where
    P: rusqlite::Params,
{
    let mut stmt = conn.prepare_cached(sql)?;
    // todo: wow, this is ugly. find a more elegant way to extract count from rows?
    let count = stmt
        .query(params)?
        .next()?
        .ok_or(anyhow!("no count returned"))?
        .get(0)?;
    Ok(count)
}

fn query_activities<P>(conn: &Connection, sql: &str, params: P) -> Result<Vec<Activity>>
where
    P: rusqlite::Params,
{
    let mut stmt = conn.prepare_cached(sql)?;
    let result: Vec<Activity> = stmt
        .query_and_then(params, |r| -> Result<Option<Activity>> {
            let json_data: String = r.get(0)?;
            let schema_str: String = r.get(1)?;
            match schema_str.parse::<ActivitySchema>()? {
                ActivitySchema::Activity => match serde_json::from_str::<Activity>(&json_data) {
                    Ok(activity) => Ok(Some(activity)),
                    Err(e) => {
                        warn!("Failed to deserialize Activity: {}. Skipping.", e);
                        Ok(None)
                    }
                },
                ActivitySchema::Status => match serde_json::from_str::<Status>(&json_data) {
                    Ok(status) => Ok(Some(status.into())),
                    Err(e) => {
                        warn!("Failed to deserialize Status: {}. Skipping.", e);
                        Ok(None)
                    }
                },
                _ => Err(anyhow!("unknown schema {:?}", schema_str)),
            }
        })?
        .filter_map(|r| r.ok().flatten())
        .collect();
    Ok(result)
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
