use flate2::read::GzDecoder;
use std::convert::From;
use std::fs;
use std::fs::File;
use std::io::{copy, Read};
use tar::Archive;

use crate::activitystreams::Actor;
use crate::{activitystreams, db};
use anyhow::{anyhow, Result};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use rusqlite::Connection;
use std::path::{Component, PathBuf};
use walkdir::WalkDir;

pub struct Importer {
    conn: Connection,
    media_path: PathBuf,
    skip_media: bool,
    current_media_subpath: String,
}

impl Importer {
    pub fn new(conn: Connection, media_path: PathBuf, skip_media: bool) -> Self {
        // Start with a temporary path, until we have found the actor JSON to derive a real path
        let current_media_subpath: String = format!(
            "tmp-{}",
            thread_rng()
                .sample_iter(&Alphanumeric)
                .take(30)
                .map(char::from)
                .collect::<String>()
        );
        Self {
            conn,
            media_path,
            skip_media,
            current_media_subpath,
        }
    }

    pub fn import(&mut self, filepath: PathBuf) -> Result<()> {
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

    pub fn import_tar(&mut self, file: File, use_gzip: bool) -> Result<()> {
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

    pub fn import_zip(&mut self, file: File) -> Result<()> {
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

    fn handle_entry(&mut self, path: &PathBuf, read: &mut impl Read) -> Result<()> {
        if path.ends_with("outbox.json") {
            self.handle_outbox(read)?;
        } else if path.ends_with("actor.json") {
            self.handle_actor(read)?;
        } else if !self.skip_media {
            if path.to_str().unwrap().contains("media_attachments") {
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
        }
        Ok(())
    }

    fn handle_media_attachment<R>(&self, entry_path: &PathBuf, entry_read: &mut R) -> Result<()>
    where
        R: ?Sized,
        R: Read,
    {
        let media_path = self.media_path.as_path();

        let output_path = PathBuf::new()
            .join(media_path)
            .join(&self.current_media_subpath)
            .join(entry_path);

        info!("Extracting {:?}", output_path);

        let output_parent_path = output_path.parent().unwrap();
        fs::create_dir_all(output_parent_path)?;

        let mut output_file = fs::File::create(&output_path)?;
        copy(entry_read, &mut output_file)?;

        Ok(())
    }

    fn handle_outbox(&self, read: &mut impl Read) -> Result<()> {
        let outbox: activitystreams::Outbox<serde_json::Value> = serde_json::from_reader(read)?;
        info!("Found {:?} items", outbox.ordered_items.len());
        let activities = db::activities::Activities::new(&self.conn);
        activities.import_collection(&outbox)?;
        Ok(())
    }

    fn handle_actor(&mut self, read: &mut impl Read) -> Result<()> {
        debug!("Found actor");

        // Grab the Actor as a Value to import it with max fidelity
        let actor: serde_json::Value = serde_json::from_reader(read)?;
        let actors = db::actors::Actors::new(&self.conn);
        actors.import_actor(&actor)?;

        // Convert the actor to our local type and figure out the new media subpath
        let local_actor: Actor = actor.into();
        let previous_media_subpath = String::from(&self.current_media_subpath);
        self.current_media_subpath = local_actor.id_hash();

        let temp_media_path = PathBuf::new()
            .join(&self.media_path)
            .join(&previous_media_subpath);

        // Move everything we have so far to the per-actor media path, if we have anything
        if temp_media_path.is_dir() {
            info!(
                "Moving temporary files from {:?} to {:?}",
                previous_media_subpath, self.current_media_subpath
            );

            let new_media_path = PathBuf::new()
                .join(&self.media_path)
                .join(&self.current_media_subpath);

            for entry in WalkDir::new(&temp_media_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_file() {
                    continue;
                }

                let old_path = entry.path();
                let new_path = &new_media_path.join(old_path.strip_prefix(&temp_media_path)?);

                let new_parent_path = new_path.parent().unwrap();
                fs::create_dir_all(new_parent_path)?;

                trace!(
                    "Moving temporary file from {:?} to {:?}",
                    old_path,
                    new_path
                );
                fs::rename(old_path, new_path)?;
            }

            // Clean up the temporary media path
            fs::remove_dir_all(&temp_media_path)?;
        }

        Ok(())
    }
}
