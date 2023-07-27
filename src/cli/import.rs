use anyhow::Result;
use clap::Args;
use std::convert::From;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::{Component, PathBuf};

use fossilizer::{activitystreams, archives, config, db, mastodon};

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// List of Mastodon .tar.gz export filenames to be imported
    filenames: Vec<String>,
}

/*
rework plan:

- a thread for each import file
  - visit every file entry in import in one pass
  - kick off a an outbox import when outbox found
  - kick off a media copy when media file found
- a thread to accept database writes for actors and activities
- a thread to accept media files to copy
*/

pub async fn command(args: &ImportArgs) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let data_path = PathBuf::from(&config.data_path);
    fs::create_dir_all(&data_path)?;

    for filename in &args.filenames {
        let filename: PathBuf = filename.into();
        // let file = File::open(filepath.as_path())?;

        info!("Importing {:?}", filename);

        let visitor = MastodonImporter::new();

        // todo: how to do this with the trait?!
        match filename.extension().unwrap().to_str().unwrap() {
            "gz" => archives::scan_tar(filename, true, &visitor)?,
            "tar" => archives::scan_tar(filename, false, &visitor)?,
            "zip" => archives::scan_zip(filename, &visitor)?,
            _ => println!("NO SCANNER AVAILABLE"),
        }

        /*
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
        activities.import_collection(&outbox)?;

        debug!("Imported {:?}", filename);
         */
    }
    info!("Done");

    Ok(())
}

use std::io::prelude::*;

pub struct MastodonImporter {

}

impl MastodonImporter {
    pub fn new() -> Self {
        Self { }
    }

    fn handle_outbox(&self, read: &mut impl Read) -> Result<()> {
        let bytes = read.bytes();
        println!("OUTBOX: {:?}", bytes.count());
        Ok(())
    }

    fn handle_media_attachment<R>(&self, dest_path: &PathBuf, read: &mut R) -> Result<()>
    where
        R: ?Sized,
        R: Read,
    {
        let bytes = read.bytes();
        println!("MEDIA: {:?} {:?}", dest_path, bytes.count());
        Ok(())
    }
}

impl archives::ArchiveVisitor for MastodonImporter {
    fn visit(&self, path: &PathBuf, read: &mut impl Read) -> Result<()> {
        if path.ends_with("outbox.json") {
            self.handle_outbox(read)?;
        } else if path.to_str().unwrap().contains("media_attachments") {
            // HACK: some exports seem to have leading directory paths before `media_attachments`, so strip that off
            let normalized_path: PathBuf = path
                .components()
                .skip_while(|c| match c {
                    Component::Normal(name) => name != &"media_attachments",
                    _ => true,
                })
                .collect();
            self.handle_media_attachment(&normalized_path, read)?;
        } else if let Some(ext) = path.extension() {
            // mainly for {avatar,header}.{jpg,png}, but there may be more?
            if "png" == ext || "jpg" == ext {
                self.handle_media_attachment(path, read)?;
            }
        }
        Ok(())
    }
}
