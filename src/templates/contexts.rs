use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::activitystreams;

pub type CalendarOutline = HashMap<String, HashMap<String, HashMap<String, IndexDayContext>>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexTemplateContext {
    pub site_root: String,
    pub day_entries: Vec<IndexDayContext>,
    pub calendar_outline: CalendarOutline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DayTemplateContext {
    pub site_root: String,
    pub day: String,
    pub current_day: IndexDayEntry,
    pub previous_day: Option<IndexDayEntry>,
    pub next_day: Option<IndexDayEntry>,
    pub activities: Vec<activitystreams::Activity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDayEntry {
    pub day: String,
    pub day_path: PathBuf,
    pub activity_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDayContext {
    pub previous: Option<IndexDayEntry>,
    pub current: IndexDayEntry,
    pub next: Option<IndexDayEntry>,
}
