use rust_embed::RustEmbed;
use serde::Serialize;
use serde_json::value::{to_value, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::path::PathBuf;
use tera::Tera;
use url::Url;

use crate::config;

pub mod contexts;

#[derive(RustEmbed)]
#[folder = "src/resources/templates"]
pub struct TemplateAsset;

pub fn init() -> Result<Tera, Box<dyn Error>> {
    let config = config::config()?;

    let mut tera: Tera;
    let templates_path = config.templates_path();
    if templates_path.is_dir() {
        debug!("Using templates from {:?}", templates_path);
        let templates_glob = templates_path.join("**/*.html");
        tera = Tera::new(
            templates_glob
                .to_str()
                .ok_or("failed to construct templates glob")?,
        )?;
    } else {
        debug!("Using embedded templates");
        tera = Tera::default();
        tera.add_raw_templates(templates_source())?;
    }

    tera.register_filter("sha256", filter_sha256);
    tera.register_filter("urlpath", filter_urlpath);

    Ok(tera)
}

/// Produce the sha256 hash of a string
pub fn filter_sha256(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("filter_sha256", "value", String, value);
    Ok(to_value(sha256::digest(s)).unwrap())
}

/// Strip a URL down to just its path
pub fn filter_urlpath(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("filter_sha256", "value", String, value);
    // todo: this is pretty ugly:
    let url = Url::parse("http://example.com")
        .unwrap()
        .join(s.as_str())
        .unwrap();
    Ok(to_value(url.path()).unwrap())
}

pub fn templates_source() -> Vec<(String, String)> {
    // TODO: accept configured switch over to user-supplied templates
    TemplateAsset::iter()
        .map(|filename| {
            let file = TemplateAsset::get(&filename).unwrap();
            (
                filename.to_string(),
                std::str::from_utf8(file.data.as_ref()).unwrap().to_owned(),
            )
        })
        .collect::<Vec<(String, String)>>()
}

pub fn render_to_file(
    tera: &Tera,
    file_path: &PathBuf,
    template_name: &str,
    context: impl Serialize,
) -> Result<(), Box<dyn Error>> {
    let file_parent_path = file_path.parent().ok_or("no parent path")?;
    fs::create_dir_all(file_parent_path)?;
    let context = tera::Context::from_serialize(context)?;
    let output = tera.render(template_name, &context)?;
    let mut file = fs::File::create(file_path)?;
    file.write_all(output.as_bytes())?;
    debug!("Wrote {} to {:?}", template_name, file_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    use crate::activitystreams::{Activity, Actor, IdOrObject};

    const JSON_ACTIVITY_WITH_ATTACHMENT: &str =
        include_str!("./resources/test/activity-with-attachment.json");

    const JSON_ACTOR: &str = include_str!("./resources/test/actor.json");

    #[test]
    fn test_activity_template_with_attachment() -> Result<(), Box<dyn Error>> {
        let tera = init()?;

        let mut activity: Activity = serde_json::from_str(JSON_ACTIVITY_WITH_ATTACHMENT)?;
        let actor: Actor = serde_json::from_str(JSON_ACTOR)?;
        activity.actor = IdOrObject::Object(actor);

        let mut context = tera::Context::new();
        context.insert("site_root", "../..");
        context.insert("activity", &activity);

        let rendered_source = tera.render("activity.html", &context)?;
        println!("RENDERED {rendered_source}");

        Ok(())
    }
}
