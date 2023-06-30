use anyhow::Result;
use flate2::read::GzDecoder;
use serde::Deserialize;
use std::convert::From;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{copy, Read};
use std::path::{Path, PathBuf};
use tar::{Archive, Entry};

use crate::activitystreams::{Actor, Outbox};

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
        // todo: need to close an existing archive & file first?
        self.archive = None;
    }

    // todo: define explicit error type
    pub fn open(&mut self) -> Result<(), Box<dyn Error>> {
        self.reset();

        let tar_gz = File::open(self.filepath.as_path())?;
        let tar_uncompressed = GzDecoder::new(tar_gz);
        self.archive = Some(Archive::new(tar_uncompressed));

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
    pub fn find_entry<P: AsRef<Path>>(
        &mut self,
        entry_path: P,
    ) -> Result<Entry<'_, GzDecoder<File>>, Box<dyn Error>> {
        self.open()?;
        let entry_path: &Path = entry_path.as_ref();
        let archive = self.archive.as_mut().ok_or("no archive")?;
        let entries = archive.entries()?;
        for entry in entries {
            let entry = entry?;
            if entry_path == entry.path()? {
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

    pub fn unpack_media<P>(&mut self, dest_path: P) -> Result<(), Box<dyn Error>>
    where
        P: AsRef<Path>,
    {
        use std::path::Component;
        let actor: Actor = self.actor()?;

        // Include a hash of the actor's ID in media path so that exports from
        // multiple instances end up with media in separate directories
        let dest_path = PathBuf::new().join(dest_path).join(&actor.id_hash());

        // todo: do this extraction alongside other non-media files?
        self.open()?;
        let archive = self.archive.as_mut().ok_or("no archive")?;
        let entries = archive.entries()?;

        for entry in entries {
            let mut entry = entry?;
            let entry_path = entry.path().unwrap().into_owned();
            let entry_path_extension = entry_path.extension();

            if entry_path.to_str().unwrap().contains("media_attachments") {
                // HACK: some exports seem to have leading directory paths before `media_attachments`, so strip that off
                let normalized_path: PathBuf = entry_path
                    .components()
                    .skip_while(|c| match c {
                        Component::Normal(name) => name != &"media_attachments",
                        _ => true,
                    })
                    .collect();
                extract_tar_entry(&dest_path, &normalized_path, &mut entry)?;
            } else if let Some(ext) = entry_path_extension {
                // mainly for {avatar,header}.{jpg,png}, but there may be more?
                if "png" == ext || "jpg" == ext {
                    extract_tar_entry(&dest_path, &entry_path, &mut entry)?;
                }
            }
        }

        Ok(())
    }
}

fn extract_tar_entry<P, R>(
    dest_path: P,
    within_dest_path: P,
    input_reader: &mut R,
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path>,
    R: ?Sized,
    R: Read,
{
    let output_path = PathBuf::new().join(dest_path).join(within_dest_path);
    info!("Extracting {:?}", output_path);

    let output_parent_path = output_path.parent().unwrap();
    fs::create_dir_all(&output_parent_path)?;

    let mut output_file = fs::File::create(&output_path)?;
    copy(input_reader, &mut output_file)?;

    Ok(())
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
