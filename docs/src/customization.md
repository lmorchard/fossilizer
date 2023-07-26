# Customization

Try `fossilizer init --customize`, which unpacks the following for customization:

- a `data/web` directory with static web assets that will be copied into the `build` directory

- a `data/templates` directory with [Tera templates](https://tera.netlify.app/docs/) used to produce the HTML output

- Note: this will *not* overwrite the database for an existing `data` directory, though it *will* overwrite any existing `templates` or `web` directories.

Check out the templates to see how the pages are built. For a more in-depth reference on what variables are supplied when rendering templates, check out the crate documentation:

- [`index.html` template context](./doc/fossilizer/templates/contexts/struct.IndexTemplateContext.html)
- [`day.html` template context](./doc/fossilizer/templates/contexts/struct.DayTemplateContext.html)
- [All template context structs](./doc/fossilizer/templates/contexts/index.html)
