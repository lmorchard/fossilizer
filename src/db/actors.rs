use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

use crate::activitystreams;

pub struct Actors<'a> {
    conn: &'a Connection,
}

impl<'a> Actors<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn import_actor<T: Serialize>(&self, activity: T) -> Result<()> {
        let json_text = serde_json::to_string_pretty(&activity)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO actors (json) VALUES (?1)",
            params![json_text],
        )?;
        Ok(())
    }

    pub fn get_actor<T: for<'de> Deserialize<'de>>(
        &self,
        id: &String,
    ) -> Result<T, Box<dyn Error>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare("SELECT json FROM actors WHERE id = ?")?;
        let json_text: String = stmt.query_row([id], |row| row.get(0))?;
        let actor: T = serde_json::from_str(json_text.as_str())?;
        Ok(actor)
    }

    pub fn get_actors<T: for<'de> Deserialize<'de>>(&self) -> Result<Vec<T>> {
        let conn = &self.conn;
        let mut stmt = conn.prepare("SELECT json FROM actors")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(r) = rows.next()? {
            let json_text: String = r.get(0)?;
            let actor: T = serde_json::from_str(json_text.as_str())?;
            out.push(actor);
        }
        Ok(out)
    }

    pub fn get_actors_by_id(
        &self,
    ) -> Result<HashMap<String, activitystreams::Actor>, Box<dyn Error>> {
        Ok(self
            .get_actors::<activitystreams::Actor>()?
            .into_iter()
            .map(|actor| (actor.id.clone(), actor))
            .collect())
    }
}
