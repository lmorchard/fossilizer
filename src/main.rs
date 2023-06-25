use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;
use std::path::Path;

use ap_fossilizer::mastodon_export::MastodonExport;

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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();

    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Init {} => println!("INIT {:?}", command_init()),
        Commands::Import { filename } => println!("IMPORT {:?}", command_import(filename)),
    }
}

fn command_import(filename: &Option<String>) -> Result<(), Box<dyn Error>> {
    let filename = filename.as_ref().ok_or("no filename")?;

    let mut export = MastodonExport::from(filename);
    let outbox = export.outbox()?;

    println!("outbox {:?}", outbox.ordered_items.len());

    Ok(())
}

fn command_init() -> Result<(), Box<dyn Error>> {
    let export_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/resources/test")
        .join("mastodon-export.tar.gz");

    let mut export = MastodonExport::from(export_path);
    let outbox = export.outbox()?;

    let conn = ap_fossilizer::db::init()?;

    conn.execute("BEGIN TRANSACTION", ())?;

    for item in outbox.ordered_items {
        let json_text = serde_json::to_string_pretty(&item)?;

        println!(".");
        conn.execute(
            "INSERT INTO activities (json) VALUES (?1)",
            (json_text,),
        )?;
    }

    conn.execute("COMMIT TRANSACTION", ())?;

    Ok(())
}
