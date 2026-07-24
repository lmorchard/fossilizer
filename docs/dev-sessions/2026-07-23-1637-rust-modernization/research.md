# Research — Rust modernization sweep

Baseline: build ✅, 12 tests ✅ (11 unit + `tests/exit_code.rs`), clippy `-D warnings` ✅ (default lints). From two documentarian passes on the worktree.

## Error-type surface (anyhow unification scope)

Many files import **both** `use anyhow::Result;` and `use std::error::Error;`. Written form decides resolution: `Result<T, Box<dyn Error>>` = std; `Result<T>` = anyhow.

**Return `Result<_, Box<dyn Error>>` (→ migrate to `anyhow::Result`):**
- `config.rs`: init:66, config:84, update:88, get:97, try_deserialize:104
- `db.rs`: conn:30, upgrade:47
- `themes.rs`: copy_embedded_themes:11, copy_embedded_web_assets:24, open_outfile_with_parent_dir:75
- `site_generator.rs`: setup_build_path:15, setup_data_path:29, unpack_customizable_resources:47, copy_embedded_assets:59, open_outfile_with_parent_dir:74, copy_web_assets:81, copy_files:99, plan_activities_pages:126, generate_activities_pages:153, generate_activity_page:166, generate_index_page:212, generate_index_json:236
- `app.rs`: init:8, init_logging:14
- `templates.rs`: init:16, render_to_file:58
- `cli.rs`: execute:54
- `mastodon/instance.rs`: build_instance_config_path:35, load_instance_config:42, save_instance_config:57
- `db/actors.rs`: get_actor:28, get_actors_by_id:57
- all `cli/*` `command` fns (build:28, import:19, init:18, serve:21, upgrade:9, fetch:19, mastodon:33, mastodon/code:22, link:12, verify:20, fetch:22)
- test fns in activitystreams.rs (438,455,482,502,530), templates.rs (88, commented-out)

**Already `anyhow::Result` (the target convention):** downloader (execute:25, queue:70, close:77, run:82), fetcher.fetch:48, instance.register_client_app:80, importer (import:46, import_tar:66, import_zip:83, handle_*:101/127/150/158), db/actors (import_actor:18, get_actors:39), db/activities (import*, get_activities*, count:231, query_count:265, query_activities:279, upgrade_status_json:338). `ActivitySchema::FromStr` uses `type Err = anyhow::Error` (activities.rs:25).

**Typed `rusqlite::Error` (internal helpers, keep or convert):** `SingleColumnResult` (activities.rs:246), get_published_* (134/147/160/173/187), query_single_column:252.

**Error-production idioms to convert during migration:**
- `.ok_or("string")?` (String→Box): db.rs:34, themes.rs:13/42/76, site_generator.rs:63/75/248, templates.rs:27/64 → `.ok_or_else(|| anyhow!(...))` or `.context(...)`
- `Box::new(err)`: site_generator.rs:21, 38 → `?`/`.into()`/anyhow
- `format!(...).into()`: cli/mastodon/code.rs:60 → `anyhow!` / `bail!`
- Already-anyhow: `anyhow!` at instance.rs:105, importer.rs:53/55/60, activities.rs:274/329, downloader.rs:34/94/103
- `.context(...)` currently used **nowhere** (opportunity)
- `??` on JoinHandles (fetcher.rs:143, cli/fetch.rs:118-119) relies on anyhow — stays fine

**Cross-style boundaries (anyhow→Box via `?` today; disappear after unification):** actors.rs:60-61, cli/mastodon/link.rs:20, cli/mastodon/fetch.rs:44, cli/import.rs:35, cli/fetch.rs (many).

## Duplicated / shared utilities (extract to `util` module)

- `open_outfile_with_parent_dir` — **byte-identical duplicate**: themes.rs:75 (used 16,45) and site_generator.rs:74 (used 50,66). Body: `parent().ok_or(..)? → create_dir_all? → File::create`.
- Embedded-asset copy: `copy_embedded_themes` (themes.rs:11) and generic `copy_embedded_assets<RustEmbed>` (site_generator.rs:59) are near-identical; **`copy_embedded_assets` has NO call sites (dead)** and carries `// todo: move this to a shared utils module` (site_generator.rs:58). `copy_embedded_web_assets` (themes.rs:24) filters by `<theme>/web`. rust_embed type is `ThemeAsset` (themes.rs:7-9). `templates_source` (themes.rs:53) also iterates ThemeAsset.
- Plan: one generic embedded writer in `util`; themes fns delegate; delete the dead generic.

## dotenv → dotenvy
- `config.rs:2` `use dotenv;` (bare import), `config.rs:67` `dotenv::dotenv().ok();`, `Cargo.toml:14` `dotenv = "0.15.0"`. Only references.

## Idiom / clippy-pedantic candidates (ptr_arg / trivially_copy)
- `&String`: instance.rs 35/42/58/81, actors.rs:30, activities.rs 147/160/205
- `&PathBuf` (→`&Path`): themes 11/26/75, site_generator 15/60/74/81/127/154/167/213/237, importer 101/127, activitystreams:31, templates:60
- `&Vec<T>` (→`&[T]`): site_generator 157/214/238, contexts.rs:92 (`From<&Vec<IndexDayContext>>`)
- `&bool` (→`bool`): site_generator setup_build_path:15, setup_data_path:29 (deref `if *clean`)
- Other agent-noted nits: `total_items: i32` for counts (activitystreams 97/113/119), manual push-loops in `Attachments for Object` (activitystreams 352-364)
- NOTE: default clippy passes clean today — pedantic pass will reveal the true set; curate allows.

## reqwest / TLS / deps (PR 2)

**Direct reqwest 0.11 usage (all in `src/`, migrate to 0.12 API):**
- downloader.rs:27 ClientBuilder::new().build()?, :28-32 get().send().error_for_status()?, :38 .bytes()
- instance.rs:91 Client::new(), :94 post().json().send(), :96 StatusCode::OK, :97 .json(), :104 .text()
- cli/fetch.rs:2 `use reqwest::header`, :34-41 HeaderMap + ClientBuilder.default_headers, :45/73/78-83 get().send().json()
- cli/mastodon/code.rs:50 Client::new(), :51 post().json().send(), :53 StatusCode::OK
- cli/mastodon/link.rs:31 reqwest::Url::parse_with_params
- cli/mastodon/verify.rs:34 ClientBuilder::new().build().unwrap() (still has an unwrap!), :35-39 get().header().send(), :41 StatusCode::OK
- No `reqwest::Error` by name, no `.query()`. API surface is 0.12-compatible.

**Dependency graph today (two of everything):**
- reqwest **0.11.23** (fossilizer direct, default-tls→native-tls→openssl) AND **0.12.3** (via megalodon + oauth2, `rustls-tls` + webpki-roots)
- hyper 0.14 (reqwest 0.11, warp, mockito) AND 1.2 (reqwest 0.12)
- rustls 0.22 & 0.23; native-tls 0.2 (only 0.11 side)
- tokio-tungstenite 0.21 (**warp**) AND 0.27 (megalodon)
- **`openssl = {vendored}` (Cargo.toml:39) is NOT referenced in `src/`** — present only to force vendored OpenSSL for musl (Dockerfile:7 comment). Droppable with the 0.11→0.12+rustls switch.

**De-risks the rustls call:** megalodon already builds reqwest 0.12 + rustls (ring) on **every active CI musl target today** → the historical "ring won't cross-compile for musl" blocker is already resolved for shipped targets. Only `i686-unknown-linux-musl` is commented out in CI (`ci.yml:116` "fails on building 'ring'") — already disabled, stays out of scope.

**Residual duplication after PR 2:** `warp 0.3.7` still pulls hyper 0.14 + tokio-tungstenite 0.21. Out of scope (warp upgrade is separate).

**CI (`.github/workflows/ci.yml`):** lint job (fmt+clippy -D warnings), test-pr (musl x86_64), full matrix (musl x86_64/aarch64/arm, darwin x86_64/aarch64, windows-msvc). musl-tools installed; no explicit libssl apt step (vendored). All via `houseabsolute/actions-rust-cross@v0`, `--locked --release`.

**tokio:** `Cargo.toml:36` `features = ["full", "windows-sys"]`. `"windows-sys"` is **not a real tokio feature name**. Actual APIs used: `#[tokio::main]` (main.rs:6), spawn (downloader 88/179, cli/fetch:29), JoinSet (downloader), select! (downloader:119), Notify (downloader), time (downloader tests), join! (test). No `tokio::fs`, no channels. → narrow to a minimal feature set; drop bogus `"windows-sys"`. Verify Windows build.
