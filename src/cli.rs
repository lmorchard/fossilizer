use clap::{Parser, Subcommand};
use std::convert::From;
use std::error::Error;
use std::path::PathBuf;

use ap_fossilizer::{app, db, mastodon, templates};

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
    /// Initialize teh database
    Init {},
    /// Adds files to myapp
    Import { filenames: Vec<String> },
    /// Build the static site
    Build {},
}

pub fn execute() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    app::init(cli.config.as_deref())?;

    match &cli.command {
        Commands::Init {} => info!("INIT {:?}", command_init()),
        Commands::Import { filenames } => info!("IMPORT {:?}", command_import(filenames)),
        Commands::Build {} => info!("BUILD {:?}", command_build()),
    };

    Ok(())
}

fn command_init() -> Result<(), Box<dyn Error>> {
    db::upgrade()?;
    Ok(())
}

fn command_import(filenames: &Vec<String>) -> Result<(), Box<dyn Error>> {
    for filename in filenames {
        info!("Importing {:?}", filename);

        let mut export = mastodon::Export::from(filename);
        let outbox = export.outbox()?;

        info!("Found {:?} items", outbox.ordered_items.len());

        let conn = db::conn()?;
        let activities = db::activities::Activities::new(conn);
        activities.import_outbox(outbox)?;

        debug!("Imported {:?}", filename);
    }
    info!("Done");

    Ok(())
}

fn command_build() -> Result<(), Box<dyn Error>> {    
    let tera = templates::init()?;
    let mut context = tera::Context::new();

    let conn = db::conn()?;
    let activities = db::activities::Activities::new(conn);

    let years = activities.get_published_years()?;
    info!("YEARS {:?}", years);

    for year in years {
        let months = activities.get_published_months_for_year(year)?;
        info!("MONTHS {:?}", months);
        for month in months {
            let days = activities.get_published_days_for_month(month)?;
            info!("DAYS {:?}", days);

            let day = days.get(0).ok_or("no first day")?;
            let items = activities.get_activities_for_day(day.to_string());
            //info!("ITEMS {:?}", items);
            //break;
        }
    }

    context.insert("number", &1234);

    let result = tera.render("index.html", &context)?;

    info!("RESULT: {:?}", result);

    Ok(())
}
