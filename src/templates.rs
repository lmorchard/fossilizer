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
