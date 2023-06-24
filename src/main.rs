use ap_fossilizer::mastodon_export::MastodonExport;
use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;

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

fn command_import(filename: &Option<String>) -> Result<(), Box<dyn Error>> {
    let filename = filename.as_ref().ok_or("no filename")?;

    let mut export = MastodonExport::from(filename);
    let outbox = export.outbox()?;

    println!("outbox {:?}", outbox.ordered_items.len());

    Ok(())
}
