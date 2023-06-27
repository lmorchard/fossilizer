use anyhow::Result;
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;

use ap_fossilizer::{app, db, templates};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_build_path")]
    pub build_path: String,
}

fn default_build_path() -> String {
    "./build".to_string()
}

pub fn command_build() -> Result<(), Box<dyn Error>> {
    let config = app::config_try_deserialize::<BuildConfig>()?;
    let build_path = PathBuf::from(config.build_path);

    if let Err(err) = fs::remove_dir_all(&build_path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            // todo: improve error handling here
            return Err(Box::new(err));
        }
    }
    fs::create_dir_all(&build_path)?;

    let tera = templates::init()?;

    let conn = db::conn()?;
    let activities = db::activities::Activities::new(&conn);

    let all_months = activities.get_published_months()?;
    for month in all_months {
        let month_path = PathBuf::from(&build_path).join(&month);
        fs::create_dir_all(&month_path)?;

        let days = activities.get_published_days_for_month(&month)?;
        for day in days {
            let day_path = PathBuf::from(&build_path).join(&day).with_extension("html");
            info!("DAY PATH {:?}", day_path);
            let items = activities.get_activities_for_day(&day)?;

            let mut context = tera::Context::new();
            context.insert("day", &day);
            context.insert("activities", &items);

            let day_source = tera.render("day.html", &context)?;
            let mut day_file = fs::File::create(day_path)?;
            day_file.write_all(day_source.as_bytes())?;
        }
    }
    Ok(())
}
