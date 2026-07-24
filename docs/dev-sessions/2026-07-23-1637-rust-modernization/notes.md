# Notes — Rust modernization sweep

## PR 1 (phases 1–4) — idiom polish, complete

Baseline preserved throughout: build ✅, 12 unit + `exit_code` integration test ✅, clippy `-D warnings` ✅, fmt ✅. No behavior change.

### Curated `clippy::pedantic` allow-list (Phase 4)
Enabled `#![warn(clippy::pedantic)]` in `lib.rs` + `main.rs`. Did **not** add `nursery` (kept scope to pedantic; nursery can be a later pass). Deliberate `#![allow(...)]` (documented in the attribute block), each because it's noise for a CLI app rather than a library:
- `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`, `module_name_repetitions` — doc/API-surface noise
- `unused_async` — CLI `command` fns share a uniform `async fn(...) -> Result<()>` dispatch shape; some don't await
- `implicit_hasher` — generalizing app fns over `BuildHasher` is library-only noise
- `struct_excessive_bools` — clap `Args` structs are legitimately several bool flags
- `non_std_lazy_statics` — the `lazy_static!` → `std::sync::LazyLock` swap is a **dep-graph change**, deferred to PR 2 (PR 1 is dep-graph-neutral)

Plus one **module-scoped** allow: `#![allow(clippy::manual_string_new)]` in `config.rs` — `DEFAULT_CONFIG = include_str!("./resources/default_config.toml").to_string()` binds to a template file that is currently 0 bytes, so clippy wants `String::new()`; keeping the `include_str!` binding preserves the link for when the template gains content.

### Everything else in Phase 4 was fixed, not allowed
Auto-fixed (clippy --fix): uninlined format args, needless raw-string hashes, redundant semicolons, `if !x {..} else {..}` inversions, ignored-unit patterns, redundant closure. Manual: merged identical match arms (`Video | Gifv`), `match_wildcard_for_single_variants` (`_ =>` → `ActivitySchema::Unknown(name) =>`), `unnecessary_wraps` on a test fn, `needless_pass_by_value` (`Importer::import(PathBuf)` → `&Path`), `assigning_clones` (`clone_from`). Plus contained idioms: `&bool` → `bool` (setup_build/data_path + callers), `total_items: i32` → `u64`.

### ⚠️ Destructive-autofix caught
`clippy --fix` rewrote `DEFAULT_CONFIG`'s `include_str!(..).to_string()` to `String::new()` (manual_string_new, because the template is empty). Reverted — it severs the file binding. Resolved with the scoped allow above instead.

### Deferred (deliberately NOT done in PR 1)
- **Broad `&String`/`&PathBuf`/`&Vec<T>` → `&str`/`&Path`/`&[T]` sweep.** These are *not* flagged even under pedantic — they're shielded (each `&PathBuf`/`&String` is forwarded to another fn that itself wants the owned-ref type, e.g. rusqlite params, `fs_extra`, generic `&P`). Forcing them risks an unshielding cascade with no clippy backing. Left as a focused follow-up if wanted. (Phase 1 did convert the two `themes` fns the util extraction *unshielded*.)
- `lazy_static!` → `LazyLock` (see `non_std_lazy_statics` above) → PR 2 or a follow-up.

## PR 2 (phases 5–6) — deps/TLS — NOT STARTED
Pending: reqwest 0.11→0.12 + rustls-tls, drop `openssl`, narrow tokio features, Docker/CI cleanup. Full CI cross-build matrix required.
