use anyhow::Result;
use std::error::Error;
use std::fs;
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

    fs::remove_dir_all(&build_path)?;
    fs::create_dir_all(&build_path)?;

    let tera = templates::init()?;
    let mut context = tera::Context::new();

    let conn = db::conn()?;
    let activities = db::activities::Activities::new(conn);

    let all_days = activities.get_published_days()?;
    for day in all_days {
        let day_path = PathBuf::from(&build_path).join(&day);
        fs::create_dir_all(day_path)?;
    }

    /*
    let years = activities.get_published_years()?;
    info!("YEARS {:?}", years);

    for year in years {
        let months = activities.get_published_months_for_year(year)?;
        info!("MONTHS {:?}", months);
        for month in months {
            let days = activities.get_published_days_for_month(month)?;
            info!("DAYS {:?}", days);

            let day = days.get(0).ok_or("no first day")?;
            let items = activities.get_activities_for_day(day.to_string());
            //info!("ITEMS {:?}", items);
            //break;
        }
    }

    context.insert("number", &1234);

    let result = tera.render("index.html", &context)?;

    info!("RESULT: {:?}", result);
     */

    Ok(())
}
