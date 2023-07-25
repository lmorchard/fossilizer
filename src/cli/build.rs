use anyhow::Result;
use clap::Args;
use fossilizer::{config, db, site_generator, templates};

use std::error::Error;

use std::thread;

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Delete build directory before proceeding
    #[arg(short = 'k', long)]
    clean: bool,
    /// Skip copying over media files
    #[arg(long)]
    skip_media: bool,
    /// Skip building index page
    #[arg(long)]
    skip_index: bool,
    /// Skip building pages for activities
    #[arg(long)]
    skip_activities: bool,
    /// Skip copying over web assets
    #[arg(long)]
    skip_assets: bool,
}

pub async fn command(args: &BuildArgs) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let clean = args.clean;
    let skip_media = args.skip_media;
    let skip_index = args.skip_index;
    let skip_activities = args.skip_activities;
    let skip_assets = args.skip_assets;

    site_generator::setup_build_path(&config.build_path, &clean)?;

    let threads = vec![
        thread::spawn(move || -> Result<()> {
            let config = config::config().unwrap();
            if !skip_assets {
                site_generator::copy_web_assets(&config.build_path).unwrap();
            }
            if !skip_media {
                site_generator::copy_media_files(&[config.media_path()], &config.build_path)
                    .unwrap();
            }
            Ok(())
        }),
        thread::spawn(move || -> Result<()> {
            let config = config::config().unwrap();
            let tera = templates::init().unwrap();
            let db_conn = db::conn().unwrap();
            let db_activities = db::activities::Activities::new(&db_conn);
            let db_actors = db::actors::Actors::new(&db_conn);

            if !skip_activities || !skip_index {
                let actors = db_actors.get_actors_by_id().unwrap();
                let day_entries =
                    site_generator::plan_activities_pages(&config.build_path, &db_activities)
                        .unwrap();
                if !skip_index {
                    site_generator::generate_index_page(&config.build_path, &day_entries, &tera)
                        .unwrap();
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
            Ok(())
        }),
    ];

    for t in threads {
        t.join().unwrap()?
    }

    info!("Build finished");

    Ok(())
}
