use std::error::Error;
use std::iter::Cloned;
use tera::Tera;

lazy_static! {
    pub static ref TEMPLATES_SOURCE: &'static [(&'static str, &'static str)] = &[
        ("index.html", include_str!("./templates/index.html")),
        ("layout.html", include_str!("./templates/layout.html")),
        ("day.html", include_str!("./templates/day.html")),
        ("activity.html", include_str!("./templates/activity.html")),
    ];
}

pub fn init() -> Result<Tera, Box<dyn Error>> {
    let mut tera = Tera::default();

    tera.add_raw_templates(templates_source())?;

    Ok(tera)
}

// todo: well, that's a gnarly type
pub fn templates_source() -> Cloned<std::slice::Iter<'static, (&'static str, &'static str)>> {
    // todo: allow override of templates from user-supplied source via config
    TEMPLATES_SOURCE.iter().cloned()
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    use crate::activitystreams::{Actor,Activity,IdOrObject};

    const JSON_ACTIVITY_WITH_ATTACHMENT: &str =
        include_str!("./resources/test/activity-with-attachment.json");

    const JSON_ACTOR: &str =
        include_str!("./resources/test/actor.json");

    #[test]
    fn test_activity_template_with_attachment() -> Result<(), Box<dyn Error>> {
        let tera = init()?;

        let mut activity: Activity = serde_json::from_str(JSON_ACTIVITY_WITH_ATTACHMENT)?;
        let actor: Actor = serde_json::from_str(JSON_ACTOR)?;
        activity.actor = IdOrObject::Object(actor);

        let mut context = tera::Context::new();
        context.insert("activity", &activity);

        let rendered_source = tera.render("activity.html", &context)?;
        println!("RENDERED {}", rendered_source);

        Ok(())
    }
}
