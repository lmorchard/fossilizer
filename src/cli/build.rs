use anyhow::Result;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use fossilizer::{activitystreams, config, db, templates};

#[derive(RustEmbed)]
#[folder = "src/resources/web"]
struct WebAsset;

pub fn command_build() -> Result<(), Box<dyn Error>> {
    let config = config::config()?;

    setup_build_path(&config.build_path)?;
    copy_web_assets(&config.build_path)?;
    copy_media_files(&[config.media_path()], &config.build_path)?;

    let db_conn = db::conn()?;
    let db_activities = db::activities::Activities::new(&db_conn);

    let tera = templates::init()?;
    let mut day_entries = plan_activities_pages(&config.build_path, &db_activities)?;
    generate_activities_pages(&config.build_path, &mut day_entries)?;
    generate_index_page(&config.build_path, &day_entries, &tera)?;

    Ok(())
}

fn setup_build_path(build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    /* todo: cli option to clean or not clean
    if let Err(err) = fs::remove_dir_all(build_path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            // todo: improve error handling here
            return Err(Box::new(err));
        }
    }
    */
    fs::create_dir_all(build_path)?;
    Ok(())
}

fn copy_web_assets(build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    for filename in WebAsset::iter() {
        let file = WebAsset::get(&filename).ok_or("no web asset")?;
        let outpath = PathBuf::from(build_path).join(&filename.to_string());

        let outparent = outpath.parent().ok_or("no parent path")?;
        fs::create_dir_all(outparent)?;

        let mut outfile = fs::File::create(&outpath)?;
        outfile.write_all(file.data.as_ref())?;

        debug!("Wrote {} to {:?}", filename, outpath);
    }
    Ok(())
}

fn copy_media_files<P>(media_path: &[P], build_path: &P) -> Result<(), Box<dyn Error>>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    info!("Copying {:?} to {:?}", media_path, build_path);
    // todo: use with progress? https://docs.rs/fs_extra/latest/fs_extra/fn.copy_items_with_progress.html
    fs_extra::copy_items(
        media_path,
        build_path,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: true,
            buffer_size: 64000,
            copy_inside: true,
            content_only: false,
            depth: 0,
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
    for day in all_days {
        let day_path = PathBuf::from(build_path).join(&day).with_extension("html");
        let mut context = IndexDayContext {
            current: IndexDayEntry {
                day: day.clone(),
                day_path: day_path.clone().strip_prefix(build_path)?.to_path_buf(),
                activity_count: 0,
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
    day_entries: &mut Vec<IndexDayContext>,
) -> Result<(), Box<dyn Error>> {
    for day_entry in day_entries {
        generate_activity_page(day_entry, build_path)?;
    }
    Ok(())
}

fn generate_activity_page(
    day_entry: &mut IndexDayContext,
    build_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let tera = templates::init()?;
    let db_conn = db::conn()?;
    let db_activities = db::activities::Activities::new(&db_conn);
    let db_actors = db::actors::Actors::new(&db_conn);

    let day = &day_entry.current.day;
    let day_path = &day_entry.current.day_path;
    let items: Vec<activitystreams::Activity> = db_activities
        .get_activities_for_day(&day)?
        .iter()
        // use faillable iterator here?
        .map(|activity| {
            // Dereference actor ID in activity via DB lookup
            let actor_id = activity.actor.id().unwrap();
            let actor = db_actors.get_actor(actor_id).unwrap();

            let mut activity = activity.clone();
            activity.actor = activitystreams::IdOrObject::Object(actor);
            activity
        })
        .collect();
    day_entry.current.activity_count = items.len();

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
    let index_path = PathBuf::from(&build_path)
        .join("index")
        .with_extension("html");
    let mut context = tera::Context::new();
    context.insert("site_root", ".");
    context.insert("day_entries", &day_entries);
    templates::render_to_file(&tera, &index_path, "index.html", &context)?;
    Ok(())
}
