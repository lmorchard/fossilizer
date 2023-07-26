use anyhow::Result;
use clap::Args;
use std::convert::From;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use fossilizer::{activitystreams, config, db, mastodon};

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

        info!("Importing {:?}", filename);

        let visitor = PrintingArchiveVisitor::new();

        // todo: how to do this with the trait?!
        match filename.extension().unwrap().to_str().unwrap() {
            "gz" => TarGzArchive::new(filename).scan(&visitor)?,
            "tar" => TarArchive::new(filename).scan(&visitor)?,
            "zip" => ZipArchive::new(filename).scan(&visitor)?,
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

use flate2::read::GzDecoder;
use std::fs::File;
use std::io::prelude::*;
use tar::{Archive, Entry};

pub trait ArchiveVisitor {
    fn visit(&self, path: &PathBuf, read: &impl Read) -> Result<()>;
}

pub struct PrintingArchiveVisitor();
impl PrintingArchiveVisitor {
    pub fn new() -> Self {
        Self()
    }
}
impl ArchiveVisitor for PrintingArchiveVisitor {
    fn visit(&self, path: &PathBuf, read: &impl Read) -> Result<()> {
        println!("ENTRY: {:?}", path);
        Ok(())
    }
}

pub trait ArchiveScanner {
    fn scan(&self, visitor: &impl ArchiveVisitor) -> Result<()>;
}

pub struct TarGzArchive {
    pub filepath: PathBuf,
}
impl TarGzArchive {
    pub fn new(filepath: PathBuf) -> Self {
        Self { filepath }
    }
}
impl ArchiveScanner for TarGzArchive {
    fn scan(&self, visitor: &impl ArchiveVisitor) -> Result<()> {
        let tar_gz = File::open(self.filepath.as_path())?;
        let tar_uncompressed = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar_uncompressed);
        let entries = archive.entries()?;
        for entry in entries {
            let entry = entry?;
            let entry_path: PathBuf = entry.path()?.into();
            visitor.visit(&entry_path, &entry)?;
        }
        Ok(())
    }
}

pub struct TarArchive {
    pub filepath: PathBuf,
}
impl TarArchive {
    pub fn new(filepath: PathBuf) -> Self {
        Self { filepath }
    }
}
impl ArchiveScanner for TarArchive {
    fn scan(&self, visitor: &impl ArchiveVisitor) -> Result<()> {
        let tar_uncompressed = File::open(self.filepath.as_path())?;
        let mut archive = Archive::new(tar_uncompressed);
        let entries = archive.entries()?;
        for entry in entries {
            let entry = entry?;
            let entry_path: PathBuf = entry.path()?.into();
            visitor.visit(&entry_path, &entry)?;
        }
        Ok(())
    }
}

pub struct ZipArchive {
    pub filepath: PathBuf,
}
impl ZipArchive {
    pub fn new(filepath: PathBuf) -> Self {
        Self { filepath }
    }
}
impl ArchiveScanner for ZipArchive {
    fn scan(&self, visitor: &impl ArchiveVisitor) -> Result<()> {
        let file = File::open(self.filepath.as_path())?;
        let mut archive = zip::ZipArchive::new(file).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };
            if !(*file.name()).ends_with('/') {
                visitor.visit(&outpath, &file)?;
            }
        }

        Ok(())
    }
}
