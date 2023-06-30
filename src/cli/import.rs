use anyhow::Result;
use std::convert::From;
use std::error::Error;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

use fossilizer::{app, db, mastodon, activitystreams};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    #[serde(default = "default_data_path")]
    pub data_path: PathBuf,
}

fn default_data_path() -> PathBuf {
    "./data".into()
}

impl ImportConfig {
    pub fn media_path(&self) -> PathBuf {
        self.data_path.join("media")
    }
}

pub fn command_import(filenames: &Vec<String>) -> Result<(), Box<dyn Error>> {
    let config = app::config_try_deserialize::<ImportConfig>()?;
    let data_path = PathBuf::from(&config.data_path);
    fs::create_dir_all(&data_path)?;

    for filename in filenames {
        info!("Importing {:?}", filename);

        let conn = db::conn()?;

        let mut export = mastodon::Export::from(filename);

        let media_path = config.media_path();
        fs::create_dir_all(&media_path)?;
        debug!("extracting media to {:?}", media_path);
        export.unpack_media(&media_path)?;

        let actor: serde_json::Value = export.actor()?;
        let actors = db::actors::Actors::new(&conn);
        actors.import_actor(actor)?;

        let outbox: activitystreams::Outbox<serde_json::Value> = export.outbox()?;
        info!("Found {:?} items", outbox.ordered_items.len());
        let activities = db::activities::Activities::new(&conn);
        activities.import_outbox(outbox)?;

        debug!("Imported {:?}", filename);
    }
    info!("Done");

    Ok(())
}
