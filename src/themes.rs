use rust_embed::RustEmbed;
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "src/resources/themes"]
pub struct ThemeAsset;

pub fn copy_embedded_themes(assets_output_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    for filename in ThemeAsset::iter() {
        let file = ThemeAsset::get(&filename).ok_or("no asset")?;
        let outpath = PathBuf::from(&assets_output_path).join(&filename.to_string());

        let mut outfile = open_outfile_with_parent_dir(&outpath)?;
        outfile.write_all(file.data.as_ref())?;

        debug!("Wrote {} to {:?}", filename, outpath);
    }
    Ok(())
}

pub fn copy_embedded_web_assets(
    theme_prefix: &str,
    assets_output_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let prefix = PathBuf::from(&theme_prefix)
        .join("web")
        .to_string_lossy()
        .into_owned();
    for filename in ThemeAsset::iter() {
        if !filename.to_string().starts_with(&prefix) {
            continue;
        }
        // FIXME: this is all pretty ugly - can be done better?
        let local_path = PathBuf::from(filename.to_string())
            .strip_prefix(&prefix)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let file = ThemeAsset::get(&filename).ok_or("no asset")?;
        let outpath = PathBuf::from(&assets_output_path).join(&local_path);

        let mut outfile = open_outfile_with_parent_dir(&outpath)?;
        outfile.write_all(file.data.as_ref())?;

        debug!("Wrote {} to {:?}", filename, outpath);
    }
    Ok(())
}

pub fn templates_source(theme_prefix: &str) -> Vec<(String, String)> {
    let prefix = PathBuf::from(&theme_prefix)
        .join("templates")
        .to_string_lossy()
        .into_owned();
    ThemeAsset::iter()
        .filter(|filename| filename.to_string().starts_with(&prefix))
        .map(|filename| {
            // FIXME: this is all pretty ugly - can be done better?
            let local_path = PathBuf::from(filename.to_string())
                .strip_prefix(&prefix)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let file = std::str::from_utf8(ThemeAsset::get(&filename).unwrap().data.as_ref())
                .unwrap()
                .to_owned();
            (local_path, file)
        })
        .collect::<Vec<(String, String)>>()
}

pub fn open_outfile_with_parent_dir(outpath: &PathBuf) -> Result<fs::File, Box<dyn Error>> {
    let outparent = outpath.parent().ok_or("no parent path")?;
    fs::create_dir_all(outparent)?;
    let outfile = fs::File::create(outpath)?;
    Ok(outfile)
}
