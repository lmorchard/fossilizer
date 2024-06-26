use anyhow::Result;
use clap::Args;
use fossilizer::{config, db, site_generator, templates};
use std::error::Error;

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Delete build directory before proceeding
    #[arg(short = 'k', long)]
    clean: bool,
    /// Skip building index page
    #[arg(long)]
    skip_index: bool,
    /// Skip building index JSON page
    #[arg(long)]
    skip_index_json: bool,
    /// Skip building pages for activities
    #[arg(long)]
    skip_activities: bool,
    /// Skip copying over web assets
    #[arg(long)]
    skip_assets: bool,
    /// Theme to use in building the static site
    #[arg(long)]
    theme: Option<String>,
}

pub async fn command(args: &BuildArgs) -> Result<(), Box<dyn Error>> {
    let clean = args.clean;
    let skip_index = args.skip_index;
    let skip_index_json = args.skip_index_json;
    let skip_activities = args.skip_activities;
    let skip_assets = args.skip_assets;

    config::update(|config| {
        if args.theme.is_some() {
            config.theme = args.theme.as_ref().unwrap().clone();
        }
    })?;

    let config = config::config()?;
    debug!("Using theme {:?}", config.theme);

    site_generator::setup_build_path(&config.build_path, &clean)?;

    if !skip_assets {
        site_generator::copy_web_assets(&config.build_path).unwrap();
    }

    if !skip_activities || !skip_index || !skip_index_json {
        let tera = templates::init().unwrap();
        let db_conn = db::conn().unwrap();
        let db_activities = db::activities::Activities::new(&db_conn);
        let db_actors = db::actors::Actors::new(&db_conn);

        let actors = db_actors.get_actors_by_id().unwrap();
        let day_entries =
            site_generator::plan_activities_pages(&config.build_path, &db_activities).unwrap();
        if !skip_index {
            site_generator::generate_index_page(&config.build_path, &day_entries, &tera).unwrap();
        }
        if !skip_index_json {
            site_generator::generate_index_json(&config.build_path, &day_entries).unwrap();
        }
        if !skip_activities {
            site_generator::generate_activities_pages(
                &config.build_path,
                &tera,
                &actors,
                &day_entries,
            )
            .unwrap();
        }
    }

    info!("Build finished");

    Ok(())
}
