use anyhow::Result;
use std::convert::From;
use std::error::Error;

use ap_fossilizer::{db, mastodon, activitystreams};

pub fn command_import(filenames: &Vec<String>) -> Result<(), Box<dyn Error>> {
    for filename in filenames {
        info!("Importing {:?}", filename);

        let conn = db::conn()?;

        let mut export = mastodon::Export::from(filename);

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
