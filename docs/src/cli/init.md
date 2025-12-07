# The `init` command

The `init` command prepares the current directory with data and configuration
files needed by Fossilizer. It's used like so:

```bash
mkdir my-mastodon-site
cd my-mastodon-site
fossilizer init
```

When using the `init` command for the first time, some files and directories
will be set up for you:

```bash
my-mastodon-site/
└── build
└── data
    └── data.sqlite3
```

- The `build` directory is where your static site will be generated

- The `data/data.sqlite3` file is a SQLite database into which things like
  posts and user account data will be stored.

After you've run this command, you can try [the `import` command](./build.md) to
ingest data from one or more Mastodon exports.

## Options

### `--clean`

The `--clean` flag will delete existing `build` and `data` directories before
setting things up. Be careful with this, because it will wipe out any existing
data!

```bash
fossilizer init --clean
```

### `--customize`

By default, Fossilizer will use templates and assets embedded in the executable
to generate a static web site. However, if you'd like to customize how your
site is generated, you can extract these into external files to edit:

```bash
fossilizer init --customize
```

This will result in a file structure something like this:

```bash
my-mastodon-site/
└── build
└── data
    └── media
    ├── config.toml
    ├── data.sqlite3
    └── themes
        └── default
            ├── templates
            │   ├── activity.html
            │   ├── day.html
            │   ├── index.html
            │   └── layout.html
            └── web
                ├── index.css
                └── index.js
```

- The `config.toml` file can be used to supply configuration settings

- The `data/themes` directory holds themes that can be used to customize the appearance of the site. The `default` theme is provided by default. If you want to use a different theme, you can copy the `default` directory and modify it under a directory with a different name. This name, then, can be supplied to the `build` command with the `--theme` option.

- The `data/themes/default/templates` directory holds [Tera](https://tera.netlify.app/) templates used to generate HTML pages.

- The `data/themes/default/web` directory holds web assets which will be copied into the root directory of your static site when it's generated.

TODO: Need to document configuration settings and templates. For now, just play around with the templates [used by `cli/build.rs`](https://github.com/lmorchard/fossilizer/blob/main/src/cli/build.rs) and see what happens! 😅 Configuration settings can be found in the [`config.rs` module](https://github.com/lmorchard/fossilizer/blob/main/src/config.rs)
