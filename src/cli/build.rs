use anyhow::Result;
use clap::Args;
use fossilizer::{activitystreams, config, db, templates};
use rayon::prelude::*;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use tera::Tera;

use crate::cli::init::copy_embedded_assets;

// todo: move this to a different package?
#[derive(RustEmbed)]
#[folder = "src/resources/web"]
pub struct WebAsset;

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Delete build directory before proceeding
    #[arg(short = 'k', long)]
    clean: bool,
    /// Skip copying over media files
    #[arg(long)]
    skip_media: bool,
    /// Skip building pages for activities
    #[arg(long)]
    skip_activities: bool,
    /// Skip copying over web assets
    #[arg(long)]
    skip_assets: bool,
}

pub fn command(args: &BuildArgs) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let clean = args.clean;
    let skip_media = args.skip_media;
    let skip_activities = args.skip_activities;
    let skip_assets = args.skip_assets;

    setup_build_path(&config.build_path, &clean)?;

    let threads = vec![
        thread::spawn(move || -> Result<()> {
            let config = config::config().unwrap();
            if !skip_assets {
                copy_web_assets(&config.build_path).unwrap();
            }
            if !skip_media {
                copy_media_files(&[config.media_path()], &config.build_path).unwrap();
            }
            Ok(())
        }),
        thread::spawn(move || -> Result<()> {
            if !skip_activities {
                let config = config::config().unwrap();

                let tera = templates::init().unwrap();

                let db_conn = db::conn().unwrap();
                let db_activities = db::activities::Activities::new(&db_conn);
                let db_actors = db::actors::Actors::new(&db_conn);

                let actors = db_actors.get_actors_by_id().unwrap();

                let day_entries =
                    plan_activities_pages(&config.build_path, &db_activities).unwrap();
                generate_index_page(&config.build_path, &day_entries, &tera).unwrap();
                generate_activities_pages(&config.build_path, &tera, &actors, &day_entries)
                    .unwrap();
            }
            Ok(())
        }),
    ];

    for t in threads {
        t.join().unwrap()?
    }

    Ok(())
}

fn setup_build_path(build_path: &PathBuf, clean: &bool) -> Result<(), Box<dyn Error>> {
    if *clean {
        info!("Cleaning build path");
        if let Err(err) = fs::remove_dir_all(build_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                // todo: improve error handling here
                return Err(Box::new(err));
            }
        }
    }
    fs::create_dir_all(build_path)?;
    Ok(())
}

fn copy_web_assets(build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;

    let web_assets_path = config.web_assets_path();
    if web_assets_path.is_dir() {
        let mut web_assets_contents = Vec::new();
        for entry in web_assets_path.read_dir()? {
            if let Ok(entry) = entry {
                web_assets_contents.push(entry.path());
            }
        }
        copy_files(web_assets_contents.as_slice(), &build_path)?;
    } else {
        info!("Copying embedded static web assets");
        copy_embedded_assets::<WebAsset>(&build_path)?;
    }

    Ok(())
}

fn copy_media_files<P>(media_path: &[P], build_path: &P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    copy_files(media_path, build_path)?;
    Ok(())
}

fn copy_files<P>(media_path: &[P], build_path: &P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    info!("Copying {:?} to {:?}", media_path, build_path);
    fs_extra::copy_items_with_progress(
        media_path,
        build_path,
        &fs_extra::dir::CopyOptions {
            overwrite: false,
            skip_exist: true,
            buffer_size: 64000,
            copy_inside: true,
            content_only: false,
            depth: 0,
        },
        |process_info| {
            debug!(
                "Copied {} ({} / {})",
                process_info.file_name, process_info.copied_bytes, process_info.total_bytes
            );
            fs_extra::dir::TransitProcessResult::ContinueOrAbort
        },
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct IndexDayEntry {
    pub day: String,
    pub day_path: PathBuf,
    pub activity_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDayContext {
    pub previous: Option<IndexDayEntry>,
    pub current: IndexDayEntry,
    pub next: Option<IndexDayEntry>,
}

fn plan_activities_pages(
    build_path: &PathBuf,
    db_activities: &db::activities::Activities<'_>,
) -> Result<Vec<IndexDayContext>, Box<dyn Error>> {
    let mut day_entries: Vec<IndexDayContext> = Vec::new();
    let all_days = db_activities.get_published_days()?;
    for (day, activity_count) in all_days {
        let day_path = PathBuf::from(build_path).join(&day).with_extension("html");
        let mut context = IndexDayContext {
            current: IndexDayEntry {
                day: day.clone(),
                day_path: day_path.clone().strip_prefix(build_path)?.to_path_buf(),
                activity_count,
            },
            previous: None,
            next: None,
        };
        if let Some(mut previous) = day_entries.pop() {
            previous.next = Some(context.current.clone());
            context.previous = Some(previous.current.clone());
            day_entries.push(previous);
        }
        day_entries.push(context);
    }
    Ok(day_entries)
}

fn generate_activities_pages(
    build_path: &PathBuf,
    tera: &Tera,
    actors: &HashMap<String, activitystreams::Actor>,
    day_entries: &Vec<IndexDayContext>,
) -> Result<(), Box<dyn Error>> {
    info!("Generating {} per-day pages", day_entries.len());
    day_entries.par_iter().for_each(|day_entry| {
        generate_activity_page(&build_path, &tera, &actors, &day_entry).unwrap()
    });
    Ok(())
}

fn generate_activity_page(
    build_path: &PathBuf,
    tera: &Tera,
    actors: &HashMap<String, activitystreams::Actor>,
    day_entry: &IndexDayContext,
) -> Result<(), Box<dyn Error>> {
    // let tera = templates::init()?;
    let db_conn = db::conn()?;
    let db_activities = db::activities::Activities::new(&db_conn);

    let day = &day_entry.current.day;
    let day_path = &day_entry.current.day_path;

    let items: Vec<activitystreams::Activity> = db_activities
        .get_activities_for_day(&day)?
        .iter()
        .map(|activity| {
            let actor_id: &String = activity.actor.id().unwrap();
            let actor: &activitystreams::Actor = actors.get(actor_id).unwrap();
            (activity, actor)
        })
        .filter(|(activity, _actor)| {
            // todo: any actor-related filtering needed here?
            activity.is_public()
        })
        .map(|(activity, actor)| {
            let mut activity = activity.clone();
            activity.actor = activitystreams::IdOrObject::Object(actor.clone());
            activity
        })
        .collect();

    let mut context = tera::Context::new();
    context.insert("site_root", "../..");
    context.insert("day", &day);
    context.insert("current_day", &day_entry.current);
    if let Some(previous) = &day_entry.previous {
        context.insert("previous_day", &previous);
    }
    if let Some(next) = &day_entry.next {
        context.insert("next_day", &next);
    }
    context.insert("activities", &items);

    templates::render_to_file(
        &tera,
        &PathBuf::from(&build_path).join(&day_path),
        "day.html",
        &context,
    )?;

    Ok(())
}

fn generate_index_page(
    build_path: &PathBuf,
    day_entries: &Vec<IndexDayContext>,
    tera: &tera::Tera,
) -> Result<(), Box<dyn Error>> {
    info!("Generating site index page");

    // Index daily entries into outline of years, months, days
    let mut calendar_outline: HashMap<&str, HashMap<&str, HashMap<&str, &IndexDayContext>>> =
        HashMap::new();
    for day_entry in day_entries {
        let parts = day_entry
            .current
            .day
            .split("/")
            .take(3)
            .collect::<Vec<&str>>();
        if let [year, month, day] = parts[..] {
            let year_map = match calendar_outline.entry(year) {
                Vacant(entry) => entry.insert(HashMap::new()),
                Occupied(entry) => entry.into_mut(),
            };
            let month_map = match year_map.entry(month) {
                Vacant(entry) => entry.insert(HashMap::new()),
                Occupied(entry) => entry.into_mut(),
            };
            month_map.insert(day, &day_entry);
        }
    }

    let index_path = PathBuf::from(&build_path)
        .join("index")
        .with_extension("html");

    let mut context = tera::Context::new();
    context.insert("site_root", ".");
    context.insert("day_entries", &day_entries);
    context.insert("calendar_outline", &calendar_outline);

    templates::render_to_file(&tera, &index_path, "index.html", &context)?;

    Ok(())
}
