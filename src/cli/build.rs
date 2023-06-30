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

    let tera = templates::init()?;
    let day_entries = generate_activities_pages(&config.build_path, &tera)?;
    generate_index_page(&config.build_path, day_entries, tera)?;

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

fn generate_activities_pages(
    build_path: &PathBuf,
    tera: &tera::Tera,
) -> Result<Vec<IndexDayEntry>, Box<dyn Error>> {
    let db_conn = db::conn()?;
    let db_activities = db::activities::Activities::new(&db_conn);
    let db_actors = db::actors::Actors::new(&db_conn);

    let mut all_days = db_activities.get_published_days()?;
    all_days.reverse();

    let mut day_entries: Vec<IndexDayEntry> = Vec::new();
    for day in all_days {
        let day_path = PathBuf::from(build_path).join(&day).with_extension("html");

        let month_path = day_path.parent().ok_or("no day path parent")?;
        fs::create_dir_all(month_path)?;

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

        day_entries.push(IndexDayEntry {
            day: day.clone(),
            day_path: day_path.clone().strip_prefix(build_path)?.to_path_buf(),
            activity_count: items.len(),
        });

        let mut context = tera::Context::new();
        context.insert("site_root", "../..");
        context.insert("day", &day);
        context.insert("activities", &items);

        templates::render_to_file(tera, &day_path, "day.html", &context)?;
    }
    Ok(day_entries)
}

fn generate_index_page(
    build_path: &PathBuf,
    day_entries: Vec<IndexDayEntry>,
    tera: tera::Tera,
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
