## 0.5.0

- Code cleanup to fix Clippy warnings

## 0.4.0

- Add devcontainer setup for GitHub Codespaces development
- Add 'latest' rolling release for main branch pushes
- Upgrade Ubuntu CI runners from 20.04 to 24.04 LTS
- Improve Rust build caching with target-specific cache keys

## 0.3.1

- Upgrade packages including megalodon library
- Add support for rendering quote reposts in templates
- Add on-the-fly migration from old to new status schema format
- Fix status fetching breakage from library updates

## 0.3.0

- Add Mastodon API support for incremental backup fetch via `mastodon` sub-commands
- Update quick-start documentation to mention `serve` sub-command

## 0.2.1

- Add `serve` command for local web server hosting of built site

## 0.2.0

- Add theme path selection in config and build `--theme` option

## 0.1.3

- Support for importing from multiple archive types (tar.gz and zip)
- Refactor importer to handle actor and outbox JSON separately
- Import media files into temporary directory, move to actor-specific once actor is known
- Quick bugfix to skip moving temporary media files if directory does not exist
- Update to use Pagefind 1.0.4

## 0.1.2

- Rework template contexts into defined structs and add customization documentation
- Refactor some site generator code

## 0.1.1

- Disable some work-in-progress features for the release build (i.e. fetch and fetch-mastodon)
- General code cleanup and documentation work

## 0.1.0

- Attempting to get this thing wired up for release builds on GitHub
