# Rust Modernization Sweep — Implementation Plan

**Goal:** Modernize idiom and collapse the duplicated dependency stack with zero behavior change, delivered as two independent PRs.

**Approach:** PR 1 (phases 1–4) is non-behavioral polish verifiable on the standard build: extract shared utils, unify on `anyhow`, swap dotenvy, then enable curated `clippy::pedantic` and fix idioms. PR 2 (phases 5–6) is the behavioral-but-preserving dep/TLS migration verified by the full CI matrix. One commit per phase (`Phase N: <name>`).

**Tech stack:** Rust 2021, anyhow, rust_embed, reqwest, rustls, tokio, clippy.

**TDD note:** This is a refactoring sweep with no intended behavior change, so per SKILL.md the existing suite (12 tests) is the regression net — phases are **opt-out of test-first** except Phase 1, which adds direct unit tests for the newly extracted `util` module. Every phase must keep `make test` green.

---

## PR 1 — Idiom polish

### Phase 1: Extract shared `util` module

Dedupe `open_outfile_with_parent_dir` and consolidate embedded-asset copying into one generic helper; delete the dead generic.

**Files:**
- Create: `src/util.rs`
- Modify: `src/lib.rs` — add `pub mod util;`
- Modify: `src/themes.rs` — delete local `open_outfile_with_parent_dir` (75-80); reimplement `copy_embedded_themes` (11) and `copy_embedded_web_assets` (24) as thin wrappers over the generic; keep `templates_source` and `ThemeAsset` as-is
- Modify: `src/site_generator.rs` — delete local `open_outfile_with_parent_dir` (74-79) and the **dead** generic `copy_embedded_assets` (59-72, no call sites); import from `util`; update `unpack_customizable_resources`/`copy_web_assets` call sites
- Test: unit tests in `src/util.rs`

**Key changes** — `src/util.rs` returns the target `anyhow::Result` convention from the start:
```rust
use anyhow::{anyhow, Result};
use rust_embed::RustEmbed;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Create the parent directory (if needed) and open `outpath` for writing.
pub fn open_outfile_with_parent_dir(outpath: &Path) -> Result<fs::File> {
    let outparent = outpath
        .parent()
        .ok_or_else(|| anyhow!("no parent path for {}", outpath.display()))?;
    fs::create_dir_all(outparent)?;
    Ok(fs::File::create(outpath)?)
}

/// Copy every embedded asset from a `RustEmbed` folder into `output_path`.
/// If `strip_prefix` is set, only assets under that prefix are copied and the
/// prefix is removed from their output path.
pub fn copy_embedded_assets<Assets: RustEmbed>(
    output_path: &Path,
    strip_prefix: Option<&str>,
) -> Result<()> {
    for filename in Assets::iter() {
        let name = filename.as_ref();
        let rel = match strip_prefix {
            Some(prefix) => match Path::new(name).strip_prefix(prefix) {
                Ok(r) => r.to_path_buf(),
                Err(_) => continue,
            },
            None => Path::new(name).to_path_buf(),
        };
        let asset = Assets::get(name).ok_or_else(|| anyhow!("missing embedded asset {name}"))?;
        let outpath = output_path.join(rel);
        let mut outfile = open_outfile_with_parent_dir(&outpath)?;
        outfile.write_all(asset.data.as_ref())?;
        debug!("Wrote {name} to {}", outpath.display());
    }
    Ok(())
}
```
- `themes::copy_embedded_themes(out)` → `util::copy_embedded_assets::<ThemeAsset>(out, None)`
- `themes::copy_embedded_web_assets(theme, out)` → `util::copy_embedded_assets::<ThemeAsset>(out, Some(&format!("{theme}/web")))`  (preserves the current `<theme>/web` strip; the old code strips a `PathBuf(theme).join("web")` prefix — same thing)
- Wrappers keep existing signatures so callers in `cli/build.rs`/`site_generator.rs` are untouched this phase.

**Tests (`src/util.rs`):**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_outfile_creates_missing_parent_dirs() {
        let dir = std::env::temp_dir().join(format!("fossilizer-util-{}", std::process::id()));
        let target = dir.join("a/b/c.txt");
        let _ = std::fs::remove_dir_all(&dir);
        let mut f = open_outfile_with_parent_dir(&target).unwrap();
        use std::io::Write;
        f.write_all(b"ok").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "ok");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
```
(A `copy_embedded_assets` test would need a fixture `RustEmbed` type; the existing `templates.rs`/build integration already exercises `ThemeAsset` copying, so `open_outfile` direct coverage is the targeted addition here.)

**Verification — automated:**
- [x] `make test` passes (incl. new `util` test) — 12 unit + exit_code integration
- [x] `make lint` passes (clippy `-D warnings`)
- [x] `cargo build --locked` succeeds

**Verification — manual:**
- [x] `grep -rn "open_outfile_with_parent_dir" src/` shows the definition only in `util.rs`
- [x] `grep -rn "fn copy_embedded_assets" src/` shows it only in `util.rs`

**Adaptation note:** Extracting `util` unshielded clippy `ptr_arg` on `themes::copy_embedded_themes`/`copy_embedded_web_assets` (they used to forward `&PathBuf` to a `&PathBuf`-taking local fn; now they forward to `util`'s `&Path` fn). Pulled those two `&PathBuf`→`&Path` fixes forward from Phase 4 to keep this commit lint-green. No cascade beyond the two (their caller `copy_web_assets` still forwards to a generic `&P` fn, so it stays shielded until Phase 4).

---

### Phase 2: Unify error handling on `anyhow::Result`

Replace every `Result<_, Box<dyn Error>>` in `src/` with `anyhow::Result<_>`; convert string/boxed error production to anyhow idioms.

**Files (all `Box<dyn Error>` sites from `research.md`):** `src/config.rs`, `src/db.rs`, `src/themes.rs`, `src/site_generator.rs`, `src/app.rs`, `src/templates.rs`, `src/cli.rs`, `src/mastodon/instance.rs`, `src/db/actors.rs`, and every `src/cli/*.rs` + `src/cli/mastodon/*.rs` `command` fn. Also the `Result<_, Box<dyn Error>>` test fns in `src/activitystreams.rs` and the commented-out block in `src/templates.rs`.

**Mechanical recipe (apply per file):**
1. Signature: `Result<T, Box<dyn Error>>` → `anyhow::Result<T>` (written as `Result<T>` with `use anyhow::Result;`).
2. Imports: remove `use std::error::Error;` where it becomes unused; ensure `use anyhow::{anyhow, Context, Result};` (add only the parts used).
3. Error production conversions:
   - `.ok_or("msg")?` → `.context("msg")?` (for `Option`) — e.g. `db.rs:34`, `site_generator.rs:63/248`, `templates.rs:27/64`
   - `return Err(Box::new(err));` → `return Err(err.into());` or `return Err(anyhow!(...))` — `site_generator.rs:21,38`
   - `format!(...).into()` in `Err` → `anyhow!(...)` / `bail!(...)` — `cli/mastodon/code.rs:60`
   - `.ok_or_else(|| String)` → `.ok_or_else(|| anyhow!(...))` — `cli/mastodon/code.rs:30-31,35,39`
   - `.map_err(|e| format!(...))?` → `.map_err(|e| anyhow!(...))?` or `.with_context(|| ...)?` — `cli/serve.rs:30`
4. Leave existing `anyhow!`, `??`-on-JoinHandle, and `db/activities.rs` internal `rusqlite::Error`-typed private helpers (`SingleColumnResult`, `query_single_column`, `get_published_*`) **as-is** — they already convert cleanly via `?` and are not `Box<dyn Error>`.

**Key changes (illustrative):**
```rust
// config.rs — before / after
pub fn init(config_path: &Path) -> Result<(), Box<dyn Error>> { ... }
pub fn init(config_path: &Path) -> anyhow::Result<()> { ... }   // use anyhow::Result

// db.rs:34
let database_parent_path = Path::new(&database_path).parent().ok_or("no parent path")?;
let database_parent_path = Path::new(&database_path)
    .parent()
    .context("database path has no parent directory")?;
```

**Verification — automated:**
- [ ] `grep -rn "Box<dyn Error>" src/` returns nothing (or only intentional, documented exceptions)
- [ ] `grep -rn "use std::error::Error" src/` returns nothing
- [ ] `make test` passes
- [ ] `make lint` passes
- [ ] `cargo build --locked` succeeds

**Verification — manual:**
- [ ] Error messages on a forced failure (e.g. `fossilizer build` with no DB) still read sensibly, now with anyhow context

---

### Phase 3: `dotenv` → `dotenvy`

Swap the unmaintained crate for its maintained fork.

**Files:**
- Modify: `Cargo.toml` — replace `dotenv = "0.15.0"` with `dotenvy = "0.15"`
- Modify: `src/config.rs` — remove bare `use dotenv;` (line 2); change `dotenv::dotenv().ok();` (line 67) → `dotenvy::dotenv().ok();`

**Verification — automated:**
- [ ] `cargo build --locked` succeeds
- [ ] `make test` passes
- [ ] `grep -rn "dotenv::" src/` returns nothing (only `dotenvy::`)

**Verification — manual:**
- [ ] A `.env` with `APP_LOG_LEVEL=debug` (or similar) is still honored by `fossilizer`

---

### Phase 4: Curated `clippy::pedantic` + idiom fixes

Turn on the stricter lint set, curate the allow-list, and fix flagged idioms.

**Files:**
- Modify: `src/lib.rs` and `src/main.rs` — add the lint attribute block (below)
- Modify: idiom sites from `research.md` — `&String`→`&str` (`instance.rs`, `db/actors.rs:30`, `db/activities.rs:147/160/205`), `&PathBuf`→`&Path` (throughout), `&Vec<T>`→`&[T]` (`site_generator.rs`, `contexts.rs:92`), `&bool`→`bool` (`site_generator.rs:15/29`, deref `if *clean`→`if clean`), `total_items: i32`→`u64` (`activitystreams.rs:97/113/119`), manual push-loops → iterator chains where clippy flags (`activitystreams.rs:352-364`)
- No change needed to `.github/workflows/ci.yml` / `Makefile` (already `clippy --all-targets -- -D warnings`; the crate-level attrs drive pedantic)

**Key changes** — lint block (starting point; refine allow-list against real output, record final set in `notes.md`):
```rust
#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::missing_const_for_fn
)]
```
- Callers of signature-changed fns update accordingly (e.g. `&String`→`&str` and `&PathBuf`→`&Path` callers usually need no change — the references coerce).
- If a pedantic lint demands a change that risks behavior or is pure noise, add it to the `allow` block instead and note why.

**Verification — automated:**
- [ ] `cargo clippy --all-targets --locked -- -D warnings` passes with pedantic+nursery enabled
- [ ] `cargo fmt --all -- --check` passes
- [ ] `make test` passes

**Verification — manual:**
- [ ] Final `#![allow(...)]` set and rationale recorded in `notes.md`

**→ PR 1 boundary:** run `/dev-session pr` here (self-review, push, open PR, Copilot review). PR 1 is complete and mergeable before PR 2 begins.

---

## PR 2 — Dependency / TLS consolidation

*(Branch PR 2 from `main` after PR 1 merges, or stack on PR 1 if it hasn't merged yet — note the base in the PR.)*

### Phase 5: Migrate `reqwest` 0.11 → 0.12 with rustls

Bump the direct dependency and migrate call sites; converge on rustls.

**Files:**
- Modify: `Cargo.toml:38` — `reqwest = { version = "0.12", default-features = false, features = ["gzip", "json", "rustls-tls"] }`
- Modify (per `research.md` §reqwest): `src/downloader.rs`, `src/mastodon/instance.rs`, `src/cli/fetch.rs`, `src/cli/mastodon/code.rs`, `src/cli/mastodon/link.rs`, `src/cli/mastodon/verify.rs`
- Modify: `Cargo.lock` (via `cargo build`)

**Key changes:**
- The used API surface (`Client`, `ClientBuilder`, `get`/`post`, `json`, `send`, `bytes`, `text`, `error_for_status`, `StatusCode`, `header::*`, `Url::parse_with_params`) is unchanged across 0.11→0.12 — expect a near-drop-in bump. Fix `cli/mastodon/verify.rs:34` `.build().unwrap()` → `.build()?` while touching it.
- Verify no code depended on native-tls-specific behavior (none found in `research.md`).

**Verification — automated:**
- [ ] `cargo build --locked` succeeds
- [ ] `make test` passes
- [ ] `cargo tree -i reqwest` shows a **single** version (0.12.x)
- [ ] `cargo tree -i native-tls` returns nothing

**Verification — manual:**
- [ ] `fossilizer mastodon` auth flow and a real `fetch`/media download still work against a live instance (or note as CI-only if no instance handy)

---

### Phase 6: Drop OpenSSL, narrow tokio features, clean Docker/CI

Remove the now-unused vendored OpenSSL and trim tokio; let the full CI matrix prove the musl/darwin/windows builds.

**Files:**
- Modify: `Cargo.toml:39` — remove the `openssl = { version = "0.10.55", features = ["vendored"] }` line
- Modify: `Cargo.toml:36` — `tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }` (drop `"full"` and bogus `"windows-sys"`; add back any feature the compiler/CI demands and record it)
- Modify: `Dockerfile` — remove `libssl-dev` from the apt install (~line 7) if the build no longer needs it (keep only if bundled-sqlite still requires it — verify)
- Modify: `.github/workflows/ci.yml` — no OpenSSL apt step exists to remove; confirm musl-tools step still sufficient

**Verification — automated:**
- [ ] `cargo build --locked` succeeds locally
- [ ] `make test` passes
- [ ] `cargo tree -i openssl` returns nothing
- [ ] `cargo tree -i openssl-sys` returns nothing

**Verification — manual (CI-gated):**
- [ ] Full CI matrix green on the PR: musl x86_64 / aarch64 / arm, darwin x86_64 / aarch64, windows-msvc
- [ ] Docker image still builds (`docker build .`) without libssl-dev
- [ ] Final tokio feature set recorded in `notes.md`

**→ PR 2 boundary:** run `/dev-session pr`; requires the full matrix (not just PR quick-test) to be green before merge.

---

## Plan self-review

- **Spec coverage:** util extraction → P1; anyhow unification → P2; dotenvy → P3; curated pedantic + idioms → P4; reqwest 0.12/rustls → P5; drop openssl + tokio narrowing + Docker → P6. All spec requirements mapped.
- **Placeholder scan:** no TBD/TODO; the two "record final set in notes.md" items are deliberate outputs of curation, each with a concrete starting set given.
- **Type consistency:** `util::open_outfile_with_parent_dir(&Path) -> Result<fs::File>` and `util::copy_embedded_assets::<A: RustEmbed>(&Path, Option<&str>) -> Result<()>` are referenced consistently in P1 and unaffected by later phases; `anyhow::Result` naming consistent P2 onward.
- **Scope discipline:** filed issues #49–#53/#57/#61/#60/#55 explicitly excluded per spec; no drive-by fixes.
