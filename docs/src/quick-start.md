# Quick Start

These are rough instructions for a rough command-line tool. There is no GUI, yet.

1. Request and download [an export from your Mastodon instance](https://docs.joinmastodon.org/user/moving/#export) (e.g. `archive-20230720182703-36f08a7ce74bbf59f141b496b2b7f457.tar.gz`)
1. Download [a release of pagefind](https://github.com/CloudCannon/pagefind/releases) and [install it](https://pagefind.app/docs/installation/) or use [a precompiled binary](https://pagefind.app/docs/installation/#downloading-a-precompiled-binary)
1. Download [a release of Fossilizer](https://github.com/lmorchard/fossilizer/releases) - there is no installation, just a standalone command.
    - Note: on macOS, you'll need to make an exception to run `fossilizer` in Security & Privacy settings
1. Make a working directory somewhere
1. Initialize the `data` directory:
    ```
    fossilizer init
    ```
1. Ingest your Mastodon export and extract media attachments:
    ```
    fossilizer import archive-20230720182703-36f08a7ce74bbf59f141b496b2b7f457.tar.gz
    ```
1. Build your static website in the `build` directory:
    ```
    fossilizer build
    ```
1. Build pagefind assets for search:
    ```
    pagefind --keep-index-url --site build
    ```
1. Serve the `build` directory up with a local web server - the `--open` option will attempt to open a browser:
    ```
    fossilzer serve --open
    ```
1. Enjoy a static web site of your Mastodon toots.

## Tips

- Try `fossilizer` by itself for a list of subcommands, try `--help` as an option to get more details on any command.

- Try `fossilizer upgrade` to upgrade the SQLite database and other assets when you download a new version. This is not (yet) automatic.

- `data/config.toml` can be used to set many as-yet undocumented configuration options.

- `data/data.sqlite3` is a a persistent SQLite database that accumulates all imported data.

- `data/media` is where media attachments are unpacked.

- You can repeatedly import data and import from multiple Mastodon instances. Everything will be merged.

- Try `fossilizer init --customize`, which unpacks the following for customization:

  - a `data/web` directory with static web assets that will be copied into the `build` directory

  - a `data/templates` directory with [Tera](https://tera.netlify.app/docs/) templates used to produce the HTML output

  - Note: this will *not* overwrite the database for an existing `data` directory, though it *will* overwrite any existing `templates` or `web` directories.
