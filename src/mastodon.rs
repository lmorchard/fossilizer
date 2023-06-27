use anyhow::Result;
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::convert::From;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use tar::{Archive, Entry};

use crate::activitystreams::Outbox;

pub struct Export {
    pub filepath: PathBuf,
    pub archive: Option<Archive<GzDecoder<File>>>,
}

impl From<&String> for Export {
    fn from(filepath: &String) -> Self {
        Self::new(PathBuf::from(filepath))
    }
}

impl From<PathBuf> for Export {
    fn from(filepath: PathBuf) -> Self {
        Self::new(filepath)
    }
}

impl Export {
    pub fn new(filepath: PathBuf) -> Self {
        Self {
            filepath,
            archive: None,
        }
    }

    pub fn reset(&mut self) {
        self.archive = None;
    }

    // todo: define explicit error type
    pub fn open(&mut self) -> Result<(), Box<dyn Error>> {
        self.reset();

        let tar_gz = File::open(self.filepath.as_path())?;
        self.archive = Some(Archive::new(GzDecoder::new(tar_gz)));

        Ok(())
    }

    // todo: re-reading the archive is wasteful, maybe just read all the non-attachment entries at once?
    /*
       -r--r--r-- wheel/wheel  9839493 2023-01-15 04:19 outbox.json
       -r--r--r-- wheel/wheel   324889 2023-01-15 04:19 likes.json
       -r--r--r-- wheel/wheel     1271 2023-01-15 04:19 bookmarks.json
       -r--r--r-- wheel/wheel   256635 2023-01-15 04:19 avatar.png
       -r--r--r-- wheel/wheel   100238 2023-01-15 04:19 header.jpg
       -r--r--r-- wheel/wheel     3705 2023-01-15 04:19 actor.json
    */

    pub fn find_entry(
        &mut self,
        entry_name: &str,
    ) -> Result<Entry<'_, GzDecoder<File>>, Box<dyn Error>> {
        self.open()?;
        let archive = self.archive.as_mut().ok_or("no archive")?;
        let entries = archive.entries()?;
        for entry in entries {
            let entry = entry?;
            if entry.path()?.to_str().ok_or("no path str")? == entry_name {
                return Ok(entry);
            }
        }
        Err("not found".into())
    }

    // todo: define more specific error type
    pub fn outbox<T: for<'de> Deserialize<'de>>(&mut self) -> Result<Outbox<T>, Box<dyn Error>> {
        let entry = self.find_entry("outbox.json")?;
        let outbox: Outbox<T> = serde_json::from_reader(entry)?;
        Ok(outbox)
    }

    pub fn actor<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T, Box<dyn Error>> {
        let entry = self.find_entry("actor.json")?;
        let actor: T = serde_json::from_reader(entry)?;
        Ok(actor)
    }

    // todo: unpack media_attachments dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const TEST_RESOURCES_PATH: &str = "src/resources/test";

    lazy_static! {
        static ref MASTODON_EXPORT_PATH: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(TEST_RESOURCES_PATH)
            .join("mastodon-export.tar.gz");
    }

    #[test]
    fn it_can_load_outbox() -> Result<(), Box<dyn Error>> {
        let mut export = Export::from(MASTODON_EXPORT_PATH.clone());
        let outbox: Outbox<crate::activitystreams::Activity> = export.outbox()?;
        assert!(!outbox.ordered_items.is_empty());
        Ok(())
    }

    #[test]
    fn it_can_load_outbox_as_values() -> Result<(), Box<dyn Error>> {
        let mut export = Export::from(MASTODON_EXPORT_PATH.clone());
        let outbox: Outbox<serde_json::Value> = export.outbox()?;
        assert!(!outbox.ordered_items.is_empty());
        Ok(())
    }

    #[test]
    fn it_can_load_actor_as_value() -> Result<(), Box<dyn Error>> {
        let mut export = Export::from(MASTODON_EXPORT_PATH.clone());
        let actor: serde_json::Value = export.actor()?;
        let json_text = serde_json::to_string_pretty(&actor)?;
        assert!(json_text.contains("@context"));
        Ok(())
    }

    #[test]
    fn it_can_load_actor_as_local_model() -> Result<(), Box<dyn Error>> {
        let mut export = Export::from(MASTODON_EXPORT_PATH.clone());
        let actor: crate::activitystreams::Actor = export.actor()?;
        assert_eq!(actor.id, "https://mastodon.social/users/lmorchard",);
        assert_eq!(actor.url, "https://mastodon.social/@lmorchard",);
        Ok(())
    }

    #[test]
    fn it_can_load_adtor_as_external_model() -> Result<(), Box<dyn Error>> {
        let mut export = Export::from(MASTODON_EXPORT_PATH.clone());
        let actor: activitystreams::actor::ActorBox = export.actor()?;
        let actor: activitystreams::actor::Person = actor.into_concrete()?;
        assert_eq!(
            actor.as_ref().get_id().ok_or("no id")?.as_str(),
            "https://mastodon.social/users/lmorchard",
        );
        assert_eq!(
            actor
                .as_ref()
                .get_url_xsd_any_uri()
                .ok_or("no url")?
                .as_str(),
            "https://mastodon.social/@lmorchard",
        );
        Ok(())
    }
}
