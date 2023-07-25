use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::activitystreams;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexTemplateContext {
    pub site_root: String,
    pub calendar: CalendarContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayTemplateContext {
    pub site_root: String,
    pub day: IndexDayContext,
    pub activities: Vec<activitystreams::Activity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDayContext {
    pub previous: Option<IndexDayEntry>,
    pub current: IndexDayEntry,
    pub next: Option<IndexDayEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDayEntry {
    pub date: String,
    pub path: PathBuf,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarContext(HashMap<String, HashMap<String, HashMap<String, IndexDayContext>>>);
impl CalendarContext {
    pub fn new() -> Self {
        CalendarContext(HashMap::new())
    }

    pub fn insert(&mut self, day_entry: &IndexDayContext) {
        let calendar = &mut self.0;

        let parts = day_entry
            .current
            .date
            .split('/')
            .take(3)
            .collect::<Vec<&str>>();

        if let [year, month, day] = parts[..] {
            let year_map = match calendar.entry(year.to_string()) {
                Vacant(entry) => entry.insert(HashMap::new()),
                Occupied(entry) => entry.into_mut(),
            };
            let month_map = match year_map.entry(month.to_string()) {
                Vacant(entry) => entry.insert(HashMap::new()),
                Occupied(entry) => entry.into_mut(),
            };
            month_map.insert(day.to_string(), day_entry.clone());
        }
    }
}
impl Default for CalendarContext {
    fn default() -> Self {
        Self::new()
    }
}
impl From<&Vec<IndexDayContext>> for CalendarContext {
    fn from(value: &Vec<IndexDayContext>) -> Self {
        let mut calendar = Self::new();
        for entry in value {
            calendar.insert(entry);
        }
        calendar
    }
}
