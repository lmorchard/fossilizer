use std::error::Error;
use tera::Tera;

pub fn init() -> Result<Tera, Box<dyn Error>> {
    let mut tera = Tera::default();

    tera.add_raw_template(
        "index.html",
        include_str!("./resources/templates/index.html"),
    )?;

    Ok(tera)
}
