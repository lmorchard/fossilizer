use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use tar::Archive;

pub trait ArchiveVisitor {
    fn visit(&self, path: &PathBuf, read: &mut impl Read) -> Result<()>;
}

pub fn scan_tar(filepath: PathBuf, use_gzip: bool, visitor: &impl ArchiveVisitor) -> Result<()> {
    let filepath = filepath.as_path();
    let file = File::open(filepath)?;
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
        visitor.visit(&entry_path, &mut entry)?;
    }
    Ok(())
}

pub fn scan_zip(filepath: PathBuf, visitor: &impl ArchiveVisitor) -> Result<()> {
    let filepath = filepath.as_path();
    let file = File::open(filepath)?;
    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        if !(*file.name()).ends_with('/') {
            visitor.visit(&outpath, &mut file)?;
        }
    }

    Ok(())
}

pub struct PrintingArchiveVisitor();
impl PrintingArchiveVisitor {
    pub fn new() -> Self {
        Self()
    }
}
impl ArchiveVisitor for PrintingArchiveVisitor {
    fn visit(&self, path: &PathBuf, read: &mut impl Read) -> Result<()> {
        let bytes = read.bytes();
        println!("ENTRY: {:?} {:?}", path, bytes.count());
        Ok(())
    }
}
