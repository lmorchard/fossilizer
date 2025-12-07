# For Developers

This section covers information useful for folks aiming to help contribute to or customize this software.

## Getting Started

### Using GitHub Codespaces / Devcontainer

The easiest way to get started with development is to use GitHub Codespaces or a local devcontainer. The repository includes a `.devcontainer` configuration that sets up:

- Rust toolchain with rust-analyzer
- Clippy for linting
- Recommended VS Code extensions
- All necessary dependencies

Simply open the repository in GitHub Codespaces or use VS Code's "Reopen in Container" feature.

### Manual Setup

1. Install Rust via [rustup](https://rustup.rs/)
2. Clone the repository
3. Run `cargo build` to build the project
4. Run `cargo test` to run the test suite

## Crate Documentation

`fossilizer` has not yet been published as a crate, but you can see the module docs here:

- [Crate fossilizer](../doc/fossilizer/index.html)

## Odds & Ends

- For some details on how SQLite is used here as an ad-hoc document database, check out this blog post on [Using SQLite as a document database for Mastodon exports](https://blog.lmorchard.com/2023/05/12/toots-in-sqlite/). TL;DR: JSON is stored as the main column in each row, while `json_extract()` is used mainly to generate virtual columns for lookup indexes.

- When ingesting data, care is taken to attempt to store JSON as close to the original source as possible from APIs and input files. That way, data parsing and models can be incrementally upgraded over time without having lost any information from imported sources.
