use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;
use std::path::PathBuf;
use rusqlite::params;

use ap_fossilizer::{app, db, mastodon};

pub fn execute() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    app::init(cli.config.as_deref())?;

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Init {} => println!("INIT {:?}", command_init()),
        Commands::Import { filename } => println!("IMPORT {:?}", command_import(filename)),
    };

    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    quiet: bool,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

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

fn command_init() -> Result<(), Box<dyn Error>> {
    db::upgrade()?;
    Ok(())
}

fn command_import(filename: &Option<String>) -> Result<(), Box<dyn Error>> {
    let filename = filename.as_ref().ok_or("no filename")?;
    let mut export = mastodon::Export::from(filename);
    let outbox = export.outbox()?;
    let conn = db::conn()?;

    info!("Start transaction");
    conn.execute("BEGIN TRANSACTION", ())?;

    for (count, item) in outbox.ordered_items.into_iter().enumerate() {
        let json_text = serde_json::to_string_pretty(&item)?;

        debug!("Inserting {:?}", count);
        conn.execute(
            "INSERT OR REPLACE INTO activities (json) VALUES (?1)",
            params![json_text],
        )?;
    }

    conn.execute("COMMIT TRANSACTION", ())?;
    info!("Done");

    Ok(())
}
