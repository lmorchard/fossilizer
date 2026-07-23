# Rust Modernization Sweep Spec

**Goal:** Bring fossilizer's Rust up to current idiom and collapse its duplicated dependency stack — without changing behavior — so the codebase reads consistently and builds leaner.

**Source:** User request 2026-07-23 (Les; fossilizer is a Rust learning project, fussy/pedantic fixes explicitly welcome). Relates to issues #54 (dep consolidation), #58 (cleanup grab-bag), #60 (test coverage).

## Current state

Codebase is idiomatically inconsistent in ways typical of a multi-year learning project (see `research.md`):
- **Error types split** ~evenly between `Result<_, Box<dyn Error>>` (config, db, themes, site_generator, app, templates, cli.rs, instance, actors, all cli commands) and `anyhow::Result` (downloader, fetcher, importer, db/activities, parts of actors). Most files import *both*. `.context()` is used nowhere.
- **Duplicated utils:** `open_outfile_with_parent_dir` byte-identical in `themes.rs:75` and `site_generator.rs:74`; a dead generic `copy_embedded_assets` (`site_generator.rs:59`, no call sites) with a `// todo: move to shared utils` marker.
- **Dated idioms:** `&String`/`&PathBuf`/`&Vec<T>`/`&bool` params, `total_items: i32`, bare `use dotenv;` (unmaintained crate).
- **Duplicated dep stack:** direct `reqwest 0.11` (native-tls→vendored OpenSSL) coexists with `reqwest 0.12` (rustls, via megalodon) → two each of hyper/http/rustls/tungstenite. `openssl = {vendored}` is declared but unreferenced in `src/`. `tokio = ["full", "windows-sys"]` (`"windows-sys"` is not a real tokio feature).

Baseline is green: build ✅, 12 tests ✅, clippy `-D warnings` ✅.

## Desired end state

Delivered as **two independent PRs** off `main`:

**PR 1 — Idiom polish (no behavior change, no dep-graph change):**
- Curated `clippy::pedantic` (+ selected `nursery`) enabled crate-wide via `#![warn(...)]` in `lib.rs`/`main.rs`, with deliberate `#![allow(...)]` for low-value/noisy lints, and clean under `-D warnings`.
- All public/internal error types unified on **`anyhow::Result`**; `Box<dyn Error>` removed from `src/`; string-error `.ok_or("..")?` sites converted to `anyhow!`/`.context()`; redundant `use std::error::Error;` imports removed.
- Single `util` module holding `open_outfile_with_parent_dir` and one generic embedded-asset writer; `themes`/`site_generator` delegate to it; dead `copy_embedded_assets` removed.
- `dotenv` → `dotenvy`.
- Reference-param idioms fixed where clippy flags them (`&str`/`&Path`/`&[T]`/`bool`), plus `total_items` to an unsigned type.
- Tests still green; new unit tests added only where a change alters an error path or the extracted util warrants direct coverage.

**PR 2 — Dependency / TLS consolidation (build change, behavior-preserving):**
- Direct dep bumped to `reqwest = "0.12"` with `rustls-tls` (drop `default-tls`); the ~6 files using reqwest migrated to the 0.12 API.
- `openssl` dependency removed; Dockerfile/CI libssl references cleaned up if newly unnecessary.
- `tokio` features narrowed from `"full"` to the minimal used set; bogus `"windows-sys"` removed.
- Collapses duplicate reqwest/hyper/http/native-tls/openssl trees. Full CI cross-build matrix (all musl + darwin + windows targets) green.

## Design decisions

- **Decision:** Two PRs — polish first, deps/TLS second.
  - **Why:** PR 1 is a large mechanical, non-behavioral diff verifiable on the standard build; PR 2 is behavioral (networking/TLS) and only fully verifiable via the CI cross-build matrix. Mixing them makes review and failure-bisection painful. PR 2 is independently revertible.
  - **Rejected:** one big PR (unreviewable); separate dev-sessions (over-ceremony for a coordinated sweep — sequenced as phases here instead).
- **Decision:** Unify on `anyhow`, not `Box<dyn Error>`.
  - **Why:** anyhow is already the majority convention and a dependency; gives `.context()` and cleaner signatures. Fits the "learning project, modernize" intent.
  - **Rejected:** custom `thiserror` error enum (overkill for a CLI); status quo (the inconsistency is the thing being fixed).
- **Decision:** `rustls-tls`, drop OpenSSL.
  - **Why:** megalodon already pulls reqwest 0.12 + rustls (ring) and it builds on **every active CI musl target today** — so the half-remembered historical musl/ring blocker is already resolved for shipped targets. Removes the C/OpenSSL build dep and simplifies musl static builds.
  - **Rejected:** keeping native-tls/openssl (perpetuates the dual stack and the vendored-C dependency). `i686-unknown-linux-musl` (the one target with a known `ring` failure) is already disabled in CI and stays out of scope.
- **Decision:** Curated pedantic, not blanket.
  - **Why:** `clippy::pedantic` is noisy; lints like `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`, `module_name_repetitions` add churn without value for a CLI. Enable the set, `allow` the noise deliberately, document why in the attribute block.
  - **Rejected:** blanket-accept every pedantic lint (Les was tempted but chose curation).
- **Decision:** TDD deferred; existing suite is the regression net.
  - **Why:** this is refactoring with no intended behavior change. Broad coverage expansion is issue #60. Add tests only where an error path meaningfully changes or the new `util` module deserves direct coverage.

## Patterns to follow

- Target error convention: `use anyhow::{anyhow, Context, Result};` as already in `downloader.rs:1`, `importer.rs`, `db/activities.rs:1`; `anyhow!` usage at `instance.rs:105`, `importer.rs:53-60`.
- Util module: model on the existing byte-identical `open_outfile_with_parent_dir` body (`themes.rs:75`), and the generic `copy_embedded_assets<Assets: RustEmbed>` shape (`site_generator.rs:59`) — promote that generic into `util`, delete the dead copy.
- `?`-propagation and `??`-on-JoinHandle already work under anyhow (`fetcher.rs:143`, `cli/fetch.rs:118-119`) — keep.
- reqwest call sites to migrate enumerated in `research.md` (§reqwest usage). Note `cli/mastodon/verify.rs:34` still has a `.build().unwrap()` — fix to `?` during the migration.

## What we're NOT doing

- **Not** fixing the correctness/security issues already filed: zip-slip #49, XSS #50, downloader failure-surfacing #51, TryFrom-for-untrusted-data #52, DB transactions #53, federated actor URLs #57, API-client timeouts #61. Idiom touches near this code must not silently absorb those fixes.
- **Not** broadly expanding test coverage (#60) — only targeted tests per the TDD decision.
- **Not** upgrading `warp` (its old hyper 0.14 / tungstenite 0.21 duplication remains — separate effort).
- **Not** bumping `megalodon`, or the major-version-behind `config`/`env_logger`/`rusqlite`/`zip` (separate batched bump).
- **Not** re-architecting the global config singleton (#55) or wiring the no-op `-v/-q/-d` flags (#58) — unless a pedantic lint forces a trivial local touch.
- **Not** changing any user-visible CLI behavior, output, or on-disk formats.

## Open questions

- **Exact curated pedantic allow-list.** *Default:* enable `clippy::pedantic` + `clippy::nursery`, then `allow` at minimum `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`, `module_name_repetitions`, `missing_const_for_fn`; refine against actual output during PR 1 and record the final set in `notes.md`. Not blocking.
- **Exact minimal tokio feature set.** *Default:* start from `["rt-multi-thread", "macros", "sync", "time"]` and let the compiler/CI (incl. Windows) dictate additions; record final set. Not blocking.
- **Dockerfile libssl-dev / CI musl OpenSSL.** *Default:* remove `libssl-dev` from Dockerfile and any OpenSSL assumptions once `openssl` is dropped, gated on the full matrix passing; if any target regresses, keep the minimal necessary bits and note it. Not blocking.
