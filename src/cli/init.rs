use anyhow::Result;
use clap::Args;
use rust_embed::RustEmbed;
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;

use crate::cli;
use fossilizer::{config, db, templates};

lazy_static! {
    pub static ref DEFAULT_CONFIG: String =
        include_str!("../resources/default_config.toml").to_string();
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Delete any existing data directory before initializing
    #[arg(short = 'k', long)]
    clean: bool,
    /// Prepare the data directory with resources for customization
    #[arg(short, long)]
    customize: bool,
}

pub async fn command(args: &InitArgs) -> Result<(), Box<dyn Error>> {
    setup_data_path(&args.clean)?;
    db::upgrade()?;
    if args.customize {
        unpack_customizable_resources()?;
    }
    Ok(())
}

fn setup_data_path(clean: &bool) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let data_path = &config.data_path;

    if *clean {
        info!("Cleaning data path");
        if let Err(err) = fs::remove_dir_all(data_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                // todo: improve error handling here
                return Err(Box::new(err));
            }
        }
    }

    fs::create_dir_all(data_path)?;
    Ok(())
}

fn unpack_customizable_resources() -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let data_path = &config.data_path;

    let config_outpath = data_path.join("config.toml");
    let mut config_outfile = open_outfile_with_parent_dir(&config_outpath)?;
    config_outfile.write_all(DEFAULT_CONFIG.as_bytes())?;

    copy_embedded_assets::<cli::build::WebAsset>(&config.web_assets_path())?;
    copy_embedded_assets::<templates::TemplateAsset>(&config.templates_path())?;

    Ok(())
}

// todo: move this to a shared utils module? build.rs also uses
pub fn copy_embedded_assets<Assets: RustEmbed>(
    assets_output_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    for filename in Assets::iter() {
        let file = Assets::get(&filename).ok_or("no asset")?;
        let outpath = PathBuf::from(&assets_output_path).join(&filename.to_string());

        let mut outfile = open_outfile_with_parent_dir(&outpath)?;
        outfile.write_all(file.data.as_ref())?;

        debug!("Wrote {} to {:?}", filename, outpath);
    };
    Ok(())
}

pub fn open_outfile_with_parent_dir(outpath: &PathBuf) -> Result<fs::File, Box<dyn Error>> {
    let outparent = outpath.parent().ok_or("no parent path")?;
    fs::create_dir_all(outparent)?;
    let outfile = fs::File::create(outpath)?;
    Ok(outfile)
}
