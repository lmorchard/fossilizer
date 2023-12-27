use crate::{activitystreams, config, db, templates};
use anyhow::Result;
use rayon::prelude::*;
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use tera::Tera;
use crate::config::DEFAULT_CONFIG;
use crate::templates::contexts;

#[derive(RustEmbed)]
#[folder = "src/resources/web"]
pub struct WebAsset;

pub fn setup_build_path(build_path: &PathBuf, clean: &bool) -> Result<(), Box<dyn Error>> {
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

pub fn setup_data_path(clean: &bool) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let data_path = &config.data_path;

    if *clean {
        info!("Cleaning data path");
        if let Err(err) = fs::remove_dir_all(data_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                // todo: improve error handling here
                return Err(Box::new(err));
            }
        }
    }

    fs::create_dir_all(data_path)?;
    Ok(())
}

pub fn unpack_customizable_resources() -> Result<(), Box<dyn Error>> {
    let config = config::config()?;
    let data_path = &config.data_path;

    let config_outpath = data_path.join("config.toml");
    let mut config_outfile = open_outfile_with_parent_dir(&config_outpath)?;
    config_outfile.write_all(DEFAULT_CONFIG.as_bytes())?;

    copy_embedded_assets::<WebAsset>(&config.web_assets_path())?;
    copy_embedded_assets::<templates::TemplateAsset>(&config.templates_path())?;

    Ok(())
}

// todo: move this to a shared utils module? build.rs also uses
pub fn copy_embedded_assets<Assets: RustEmbed>(
    assets_output_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    for filename in Assets::iter() {
        let file = Assets::get(&filename).ok_or("no asset")?;
        let outpath = PathBuf::from(&assets_output_path).join(&filename.to_string());

        let mut outfile = open_outfile_with_parent_dir(&outpath)?;
        outfile.write_all(file.data.as_ref())?;

        debug!("Wrote {} to {:?}", filename, outpath);
    }
    Ok(())
}

pub fn open_outfile_with_parent_dir(outpath: &PathBuf) -> Result<fs::File, Box<dyn Error>> {
    let outparent = outpath.parent().ok_or("no parent path")?;
    fs::create_dir_all(outparent)?;
    let outfile = fs::File::create(outpath)?;
    Ok(outfile)
}

pub fn copy_web_assets(build_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let config = config::config()?;

    let web_assets_path = config.web_assets_path();
    if web_assets_path.is_dir() {
        let mut web_assets_contents = Vec::new();
        for entry in (web_assets_path.read_dir()?).flatten() {
            web_assets_contents.push(entry.path());
        }
        copy_files(web_assets_contents.as_slice(), build_path)?;
    } else {
        info!("Copying embedded static web assets");
        copy_embedded_assets::<WebAsset>(build_path)?;
    }

    Ok(())
}

pub fn copy_files<P>(media_path: &[P], build_path: &P) -> Result<(), Box<dyn Error>>
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

pub fn plan_activities_pages(
    build_path: &PathBuf,
    db_activities: &db::activities::Activities<'_>,
) -> Result<Vec<contexts::IndexDayContext>, Box<dyn Error>> {
    let mut entries: Vec<contexts::IndexDayContext> = Vec::new();
    let all_days = db_activities.get_published_days()?;
    for (date, count) in all_days {
        let day_path = PathBuf::from(build_path).join(&date).with_extension("html");
        let mut context = contexts::IndexDayContext {
            current: contexts::IndexDayEntry {
                date: date.clone(),
                path: day_path.clone().strip_prefix(build_path)?.to_path_buf(),
                count,
            },
            previous: None,
            next: None,
        };
        if let Some(mut previous) = entries.pop() {
            previous.next = Some(context.current.clone());
            context.previous = Some(previous.current.clone());
            entries.push(previous);
        }
        entries.push(context);
    }
    Ok(entries)
}

pub fn generate_activities_pages(
    build_path: &PathBuf,
    tera: &Tera,
    actors: &HashMap<String, activitystreams::Actor>,
    day_entries: &Vec<contexts::IndexDayContext>,
) -> Result<(), Box<dyn Error>> {
    info!("Generating {} per-day pages", day_entries.len());
    day_entries
        .par_iter()
        .for_each(|day_entry| generate_activity_page(build_path, tera, actors, day_entry).unwrap());
    Ok(())
}

pub fn generate_activity_page(
    build_path: &PathBuf,
    tera: &Tera,
    actors: &HashMap<String, activitystreams::Actor>,
    day_entry: &contexts::IndexDayContext,
) -> Result<(), Box<dyn Error>> {
    // let tera = templates::init()?;
    let db_conn = db::conn()?;
    let db_activities = db::activities::Activities::new(&db_conn);

    let day = &day_entry.current.date;
    let day_path = &day_entry.current.path;

    let items: Vec<activitystreams::Activity> = db_activities
        .get_activities_for_day(day)?
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

    templates::render_to_file(
        tera,
        &PathBuf::from(&build_path).join(day_path),
        "day.html",
        contexts::DayTemplateContext {
            site_root: "../..".to_string(),
            activities: items,
            day: day_entry.clone(),
        },
    )?;

    Ok(())
}

pub fn generate_index_page(
    build_path: &PathBuf,
    day_entries: &Vec<contexts::IndexDayContext>,
    tera: &tera::Tera,
) -> Result<(), Box<dyn Error>> {
    info!("Generating site index page");

    let index_path = PathBuf::from(&build_path)
        .join("index")
        .with_extension("html");

    templates::render_to_file(
        tera,
        &index_path,
        "index.html",
        contexts::IndexTemplateContext {
            site_root: ".".to_string(),
            calendar: day_entries.into(),
        },
    )?;

    Ok(())
}

pub fn generate_index_json(
    build_path: &PathBuf,
    day_entries: &Vec<contexts::IndexDayContext>,
) -> Result<(), Box<dyn Error>> {
    info!("Generating site index JSON");

    let file_path = PathBuf::from(&build_path)
        .join("index")
        .with_extension("json");

    let output = serde_json::to_string_pretty(&day_entries)?;

    let file_parent_path = file_path.parent().ok_or("no parent path")?;
    fs::create_dir_all(file_parent_path)?;
    
    let mut file = fs::File::create(file_path)?;
    file.write_all(output.as_bytes())?;

    Ok(())
}
