use anyhow::{anyhow, Result};
use clap::Args;
use flate2::read::GzDecoder;
use std::convert::From;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Component, PathBuf};
use tar::Archive;

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

    let media_path = config.media_path();
    fs::create_dir_all(&media_path)?;
    
    let importer = MastodonImporter::new(media_path);

    for filename in &args.filenames {
        let filename: PathBuf = filename.into();

        info!("Importing {:?}", filename);

        importer.import(filename)?;

        /*
        let conn = db::conn()?;

        let mut export = mastodon::Export::from(filename);

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

pub struct MastodonImporter {
    media_path: PathBuf,
}

impl MastodonImporter {
    pub fn new(media_path: PathBuf) -> Self {
        Self {
            media_path
        }
    }

    pub fn import(&self, filepath: PathBuf) -> Result<()> {
        let filepath = filepath.as_path();
        let file = File::open(filepath)?;

        // todo: do something with filemagic here to auto-detect archive format based on file contents?
        let extension = filepath
            .extension()
            .ok_or(anyhow!("no file extension"))?
            .to_str()
            .ok_or(anyhow!("no file extension"))?;
        match extension {
            "gz" => self.import_tar(file, true)?,
            "tar" => self.import_tar(file, false)?,
            "zip" => self.import_zip(file)?,
            _ => println!("NO SCANNER AVAILABLE"),
        };

        Ok(())
    }

    pub fn import_tar(&self, file: File, use_gzip: bool) -> Result<()> {
        // hack: this optional decompression seems funky, but it works
        let file: Box<dyn Read> = if use_gzip {
            Box::new(GzDecoder::new(file))
        } else {
            Box::new(file)
        };
        let mut archive = Archive::new(file);
        let entries = archive.entries()?;
        for entry in entries {
            let mut entry = entry?;
            let entry_path: PathBuf = entry.path()?.into();
            self.handle_entry(&entry_path, &mut entry)?;
        }
        Ok(())
    }

    pub fn import_zip(&self, file: File) -> Result<()> {
        let mut archive = zip::ZipArchive::new(file).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };
            // is this really the best way to detect that an entry isn't a directory?
            if !(*file.name()).ends_with('/') {
                self.handle_entry(&outpath, &mut file)?;
            }
        }

        Ok(())
    }

    fn handle_entry(&self, path: &PathBuf, read: &mut impl Read) -> Result<()> {
        if path.ends_with("outbox.json") {
            self.handle_outbox(read)?;
        } else if path.ends_with("actor.json") {
            self.handle_actor(read)?;
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

    fn handle_outbox(&self, read: &mut impl Read) -> Result<()> {
        let bytes = read.bytes();
        println!("OUTBOX: {:?}", bytes.count());
        Ok(())
    }

    fn handle_actor(&self, read: &mut impl Read) -> Result<()> {
        let bytes = read.bytes();
        println!("ACTOR: {:?}", bytes.count());
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
