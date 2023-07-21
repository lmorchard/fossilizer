# The `init` command

The `init` command prepares the current directory with data and configuration
files needed by Fossilzer. It's used like so:

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
    └── media
    └── data.sqlite3
```

- The `build` directory is where your static site will be generated

- The `data/media` directory is where media attachments will be extracted

- The `data/dada.sqlite3` file is a SQLite database into which things like
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

By default, Fossilzer will use templates and assets embedded in the executable
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
    ├── templates
    │   ├── activity.html
    │   ├── day.html
    │   ├── index.html
    │   └── layout.html
    └── web
        ├── index.css
        ├── index.js
        └── vendor
            ├── bootstrap.bundle.min.js
            └── bootstrap.min.css
```

- The `config.toml` file can be used to supply configuration settings

- The `data/templates` directory holds [Tera](https://tera.netlify.app/) templates
  used to generate HTML pages.

- The `data/web` directory holds web assets which will be copied into the root
  directory of your static site when it's generated.

TODO: Need to document configuration settings and templates. For now, just play around with the templates [used by `cli/build.rs`](https://github.com/lmorchard/fossilizer/blob/main/src/cli/build.rs) and see what happens! 😅 Configuration settings can be found in the [`config.rs` module](https://github.com/lmorchard/fossilizer/blob/main/src/config.rs)