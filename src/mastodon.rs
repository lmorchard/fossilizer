use flate2::read::GzDecoder;
use std::convert::From;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use tar::Archive;

use crate::activitystreams::{Activity, Outbox};

pub struct Export {
    pub filepath: PathBuf,
    pub archive: Option<Archive<GzDecoder<File>>>,
}

impl From<&String> for Export {
    fn from(filepath: &String) -> Self {
        Self::new(PathBuf::from(filepath))
    }
}

impl From<PathBuf> for Export {
    fn from(filepath: PathBuf) -> Self {
        Self::new(filepath)
    }
}

impl Export {
    pub fn new(filepath: PathBuf) -> Self {
        Self {
            filepath,
            archive: None,
        }
    }

    pub fn reset(&mut self) {
        self.archive = None;
    }

    // todo: define explicit error type
    pub fn open(&mut self) -> Result<(), Box<dyn Error>> {
        self.reset();

        let tar_gz = File::open(self.filepath.as_path())?;
        self.archive = Some(Archive::new(GzDecoder::new(tar_gz)));

        Ok(())
    }

    // todo: define more specific error type
    pub fn outbox(&mut self) -> Result<Outbox<Activity>, Box<dyn Error>> {
        self.open()?;
        let archive = self.archive.as_mut().ok_or("no archive")?;
        let entries = archive.entries()?;
        for entry in entries {
            let entry = entry?;
            if entry.path()?.ends_with("outbox.json") {
                let outbox: Outbox<Activity> = serde_json::from_reader(entry)?;
                return Ok(outbox);
            }
        }

        Err("no outbox".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const TEST_RESOURCES_PATH: &str = "src/resources/test";

    #[test]
    fn it_loads_mastodon_export() -> Result<(), Box<dyn Error>> {
        let export_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(TEST_RESOURCES_PATH)
            .join("mastodon-export.tar.gz");

        let mut export = Export::from(export_path);
        let outbox = export.outbox()?;

        println!("outbox {:?}", outbox.ordered_items.len());

        Ok(())
    }
}
