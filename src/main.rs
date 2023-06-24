use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Init the thingy
    Init {},
    /// Adds files to myapp
    Import { filename: Option<String> },
}

fn main() {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Init {} => {
            println!("init!")
        }
        Commands::Import { filename } => match command_import(filename) {
            Ok(_) => println!("OK"),
            Err(err) => println!("ERR {:?}", err),
        },
    }
}

use flate2::read::GzDecoder;
use std::fs::File;
use std::io::Read;
use tar::Archive;

fn command_import(filename: &Option<String>) -> Result<(), String> {
    println!("'myapp import' was used, name is: {filename:?}");

    let filename = filename.as_ref().ok_or("no filename")?;
    let tar_gz = File::open(filename).or(Err("no targs"))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    let entries = archive.entries().or(Err("no entries"))?;

    for entry in entries {
        let mut entry = entry.or(Err("bad entry"))?;
        let entry_path = entry.path().or(Err("bad entry path"))?;
        if entry_path.ends_with("outbox.json") {
            println!("entry! {:?} {:?}", entry_path, entry.size());

            let mut buffer = String::new();
            entry.read_to_string(&mut buffer);

            println!("json! {:?}", buffer);
        }
    }

    Ok(())
}
