use crate::util;
use rust_embed::RustEmbed;
use std::error::Error;
use std::path::{Path, PathBuf};

#[derive(RustEmbed)]
#[folder = "src/resources/themes"]
pub struct ThemeAsset;

pub fn copy_embedded_themes(assets_output_path: &Path) -> Result<(), Box<dyn Error>> {
    util::copy_embedded_assets::<ThemeAsset>(assets_output_path, None)?;
    Ok(())
}

pub fn copy_embedded_web_assets(
    theme_prefix: &str,
    assets_output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let prefix = format!("{theme_prefix}/web");
    util::copy_embedded_assets::<ThemeAsset>(assets_output_path, Some(&prefix))?;
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
