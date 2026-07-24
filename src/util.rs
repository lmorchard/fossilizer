use anyhow::{anyhow, Result};
use rust_embed::RustEmbed;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Create the parent directory (if needed) and open `outpath` for writing.
pub fn open_outfile_with_parent_dir(outpath: &Path) -> Result<fs::File> {
    let outparent = outpath
        .parent()
        .ok_or_else(|| anyhow!("no parent path for {}", outpath.display()))?;
    fs::create_dir_all(outparent)?;
    Ok(fs::File::create(outpath)?)
}

/// Copy every embedded asset from a `RustEmbed` folder into `output_path`.
///
/// If `strip_prefix` is set, only assets whose path begins with that prefix are
/// copied, and the prefix is removed from their output path. When it is `None`,
/// every asset is copied preserving its embedded path.
pub fn copy_embedded_assets<Assets: RustEmbed>(
    output_path: &Path,
    strip_prefix: Option<&str>,
) -> Result<()> {
    for filename in Assets::iter() {
        let name = filename.as_ref();
        let rel = match strip_prefix {
            Some(prefix) => match Path::new(name).strip_prefix(prefix) {
                Ok(rel) => rel.to_path_buf(),
                Err(_) => continue,
            },
            None => Path::new(name).to_path_buf(),
        };
        let asset = Assets::get(name).ok_or_else(|| anyhow!("missing embedded asset {name}"))?;
        let outpath = output_path.join(rel);

        let mut outfile = open_outfile_with_parent_dir(&outpath)?;
        outfile.write_all(asset.data.as_ref())?;

        debug!("Wrote {name} to {}", outpath.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_outfile_creates_missing_parent_dirs() {
        let dir = std::env::temp_dir().join(format!("fossilizer-util-{}", std::process::id()));
        let target = dir.join("a/b/c.txt");
        let _ = fs::remove_dir_all(&dir);

        let mut f = open_outfile_with_parent_dir(&target).unwrap();
        f.write_all(b"ok").unwrap();

        assert_eq!(fs::read_to_string(&target).unwrap(), "ok");
        fs::remove_dir_all(&dir).unwrap();
    }
}
