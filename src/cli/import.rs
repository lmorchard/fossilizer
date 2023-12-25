use anyhow::Result;
use clap::Args;
use std::convert::From;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use fossilizer::{config, db, mastodon};

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// List of Mastodon .tar.gz export filenames to be imported
    filenames: Vec<String>,
    /// Skip importing media files
    #[arg(long)]
    skip_media: bool,
}

pub async fn command(args: &ImportArgs) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let skip_media = args.skip_media;

    let data_path = PathBuf::from(&config.data_path);
    fs::create_dir_all(&data_path)?;

    let media_path = config.media_path();
    fs::create_dir_all(&media_path)?;

    let conn = db::conn()?;
    let mut importer = mastodon::Importer::new(conn, media_path, skip_media);

    for filename in &args.filenames {
        let filename: PathBuf = filename.into();
        info!("Importing {:?}", filename);
        importer.import(filename)?;
    }
    info!("Done");

    Ok(())
}
