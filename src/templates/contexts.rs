//! Structs associated with templates to define available variables.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::activitystreams;

/// Base template context for the `index.html` template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexTemplateContext {
    /// Relative path to the site root from the current page
    pub site_root: String,
    /// Calendar of nested hashmaps, organizing [IndexDayContext]s by year, month, and day
    pub calendar: CalendarContext,
}

/// Base template context for the `day.html` template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayTemplateContext {
    /// Relative path to the site root from the current page
    pub site_root: String,
    /// Context for the page's current day
    pub day: IndexDayContext,
    /// The set of activities posted on the current day
    pub activities: Vec<activitystreams::Activity>,
}

/// Context for a day, including previous and next days for navigation purposes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDayContext {
    pub previous: Option<IndexDayEntry>,
    pub current: IndexDayEntry,
    pub next: Option<IndexDayEntry>,
}

/// Details on a single day
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDayEntry {
    /// The date in yyyy/mm/dd format
    pub date: String,
    /// The file path to the day's HTML page
    pub path: PathBuf,
    /// A count of activities posted on this day
    pub count: usize,
}

/// Calendar of [IndexDayContext] structs, organized by into nested [HashMap]s 
/// indexed by year, month, and day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarContext(HashMap<String, HashMap<String, HashMap<String, IndexDayContext>>>);

impl CalendarContext {
    pub fn new() -> Self {
        CalendarContext(HashMap::new())
    }

    /// Insert an [IndexDayContext] into the [CalendarContext], using
    /// year / month / day keys based on the date property
    pub fn insert(&mut self, day_context: &IndexDayContext) {
        let calendar = &mut self.0;

        let parts = day_context
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
            month_map.insert(day.to_string(), day_context.clone());
        }
        // else: throw an error because the date format was bunk?
    }
}
impl Default for CalendarContext {
    fn default() -> Self {
        Self::new()
    }
}
impl From<&Vec<IndexDayContext>> for CalendarContext {
    /// Produce a [CalendarContext] from a [`Vec<IndexDayContext>`]
    fn from(value: &Vec<IndexDayContext>) -> Self {
        let mut calendar = Self::new();
        for entry in value {
            calendar.insert(entry);
        }
        calendar
    }
}
