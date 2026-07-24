# Media Storage in `data/` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move downloaded media from the regeneratable `build/media` to a durable `data/media`, and have `build` present `build/media` as a symlink so the static site stays self-contained — without duplicating media and without `build --clean` ever destroying it.

**Architecture:** `config.media_path()` becomes the single source of the media location (default `<data_path>/media`, override `APP_MEDIA_PATH`); all three download call sites already funnel through it. A new `fossilizer::media::ensure_build_media` function creates/maintains the `build/media` symlink, auto-migrates a legacy real `build/media` into `data/media`, and falls back to copying where symlinks are unsupported. `build` calls it after `setup_build_path`. The `build --clean` safety property (delete the symlink entry, never follow it into `data/media`) is locked with a regression test.

**Tech Stack:** Rust, `std::fs` (symlinks via `std::os::{unix,windows}::fs`), `fs_extra` (copy fallback), existing `config` crate + `APP_`-prefixed env.

## Global Constraints

- Media location resolves from `config.media_path()`; default `<data_path>/media`; override via `APP_MEDIA_PATH` (the `config` crate maps `APP_MEDIA_PATH` → the `media_path` config key).
- Symlink targets MUST be absolute (a relative target stored at `build/media` would resolve relative to `build/` and point to the wrong place).
- Migration MUST be non-destructive: never merge or clobber. If both a populated legacy `build/media` and a populated `media_path` exist, error out.
- `build --clean` MUST remove the `build/media` symlink entry without deleting the contents of `data/media` through it. (`std::fs::remove_dir_all` already does this; the test guards it.)
- Symlink on Unix; if symlink creation fails (Windows without privilege, unsupported FS), fall back to copying media into `build/media` and log a warning — non-fatal.
- Tests use per-test temp directories under `std::env::temp_dir()` (no new dev-dependency); each test uses a unique dir name and cleans it at start.
- Follow existing patterns: new module declared `pub mod media;` in `src/lib.rs`; `log` macros (`warn!`) are already available crate-wide via `#[macro_use] extern crate log`.

---

## File Structure

- `src/config.rs` (modify) — add `media_path: Option<PathBuf>` field; change the `media_path()` accessor to resolve from it (default `data_path/media`).
- `src/media.rs` (create) — `ensure_build_media` + private helpers (`symlink_dir`, `dir_has_entries`, `absolutize`, `copy_dir_contents`) + all unit tests including the `--clean` safety regression test.
- `src/lib.rs` (modify) — declare `pub mod media;`.
- `src/cli/build.rs` (modify) — call `media::ensure_build_media` after `setup_build_path`.
- `Changes.md` (modify) — note the behavior change + operator migration.
- `docs/dev-sessions/2026-07-23-1439-media-in-data-dir/notes.md` (create) — session summary.

---

## Task 1: `media_path` becomes a configurable, data-relative path

**Files:**
- Modify: `src/config.rs`
- Test: `src/config.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: existing `AppConfig` (`build_path`, `data_path` fields; `media_path()` accessor).
- Produces: `AppConfig.media_path: Option<PathBuf>` field and `AppConfig::media_path(&self) -> PathBuf` returning `self.media_path.clone().unwrap_or_else(|| self.data_path.join("media"))`. All later tasks rely on `media_path()` returning `data/media` by default.

- [ ] **Step 1: Write the failing tests**

Add to the bottom of `src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_path_defaults_to_data_path_media() {
        let config = AppConfig {
            data_path: PathBuf::from("/tmp/somedata"),
            ..Default::default()
        };
        assert_eq!(config.media_path(), PathBuf::from("/tmp/somedata/media"));
    }

    #[test]
    fn media_path_uses_explicit_override() {
        let config = AppConfig {
            data_path: PathBuf::from("/tmp/somedata"),
            media_path: Some(PathBuf::from("/mnt/bigdisk/media")),
            ..Default::default()
        };
        assert_eq!(config.media_path(), PathBuf::from("/mnt/bigdisk/media"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib config::tests 2>&1 | tail -20`
Expected: compile error — `AppConfig` has no field `media_path` (and/or the accessor returns `build_path/media`). This is the expected RED (fails to compile because the field doesn't exist yet).

- [ ] **Step 3: Add the field and update the accessor**

In `src/config.rs`, add the field to `AppConfig` (after `data_path`):

```rust
    #[serde(default = "default_data_path")]
    pub data_path: PathBuf,

    /// Location for downloaded media. Defaults to `<data_path>/media` when
    /// unset. Override with `APP_MEDIA_PATH`. Kept separate from `build_path`
    /// so media is durable state, not regeneratable build output.
    pub media_path: Option<PathBuf>,

    #[serde(default = "default_theme")]
    pub theme: String,
```

Replace the existing `media_path()` accessor:

```rust
    pub fn media_path(&self) -> PathBuf {
        self.media_path
            .clone()
            .unwrap_or_else(|| self.data_path.join("media"))
    }
```

(The `Option<PathBuf>` field and the `media_path()` method share a name; this is legal in Rust — fields and methods occupy separate namespaces. `self.media_path` is the field, `self.media_path()` is the accessor.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib config::tests 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: make media_path a configurable data-relative path"
```

---

## Task 2: `src/media.rs` — maintain the `build/media` symlink + safe migration

**Files:**
- Create: `src/media.rs`
- Modify: `src/lib.rs`
- Test: `src/media.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `crate::site_generator::setup_build_path` (for the safety regression test); `fs_extra` (already a dependency).
- Produces: `pub fn ensure_build_media(build_path: &Path, media_path: &Path) -> Result<(), Box<dyn Error>>`. Later tasks (build wiring) rely on this signature. Behavior: ensures `build_path/media` resolves to an absolute symlink to `media_path`; migrates a legacy real dir; errors on ambiguity; copies as a fallback.

- [ ] **Step 1: Declare the module**

In `src/lib.rs`, add (keeping alphabetical order, after `pub mod mastodon;`):

```rust
pub mod mastodon;
pub mod media;
pub mod site_generator;
```

- [ ] **Step 2: Write the failing tests**

Create `src/media.rs` with ONLY the test module first (so it fails to compile against the not-yet-written function), plus the imports:

```rust
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

// (implementation added in Step 4)

#[cfg(test)]
mod tests {
    use super::*;

    /// Fresh, unique temp dir per test; removed first so reruns are clean.
    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fossilizer-media-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_symlink_when_build_media_absent() {
        let root = test_dir("create");
        let build = root.join("build");
        let media = root.join("media");
        fs::create_dir_all(&media).unwrap();
        fs::write(media.join("photo.jpg"), b"bytes").unwrap();

        ensure_build_media(&build, &media).unwrap();

        let link = build.join("media");
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        // Readable through the link.
        assert_eq!(fs::read(link.join("photo.jpg")).unwrap(), b"bytes");
    }

    #[test]
    fn is_idempotent_on_existing_symlink() {
        let root = test_dir("idempotent");
        let build = root.join("build");
        let media = root.join("media");

        ensure_build_media(&build, &media).unwrap();
        // Second call must not error or replace the media store.
        fs::write(media.join("keep.txt"), b"x").unwrap();
        ensure_build_media(&build, &media).unwrap();

        assert!(build.join("media").join("keep.txt").exists());
    }

    #[test]
    fn migrates_legacy_real_dir_when_media_absent() {
        let root = test_dir("migrate");
        let build = root.join("build");
        let media = root.join("media"); // does NOT exist yet
        // Legacy layout: real build/media dir with a file.
        fs::create_dir_all(build.join("media")).unwrap();
        fs::write(build.join("media").join("old.png"), b"legacy").unwrap();

        ensure_build_media(&build, &media).unwrap();

        // File now lives in the media store...
        assert_eq!(fs::read(media.join("old.png")).unwrap(), b"legacy");
        // ...and build/media is now a symlink to it.
        assert!(fs::symlink_metadata(build.join("media")).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(build.join("media").join("old.png")).unwrap(), b"legacy");
    }

    #[test]
    fn errors_when_both_legacy_dir_and_media_are_populated() {
        let root = test_dir("ambiguous");
        let build = root.join("build");
        let media = root.join("media");
        fs::create_dir_all(build.join("media")).unwrap();
        fs::write(build.join("media").join("a.png"), b"a").unwrap();
        fs::create_dir_all(&media).unwrap();
        fs::write(media.join("b.png"), b"b").unwrap();

        let result = ensure_build_media(&build, &media);
        assert!(result.is_err(), "must refuse to migrate ambiguous state");
        // Nothing destroyed.
        assert!(build.join("media").join("a.png").exists());
        assert!(media.join("b.png").exists());
    }

    #[test]
    fn clean_build_does_not_delete_media_through_symlink() {
        let root = test_dir("clean-safety");
        let build = root.join("build");
        let media = root.join("media");
        fs::create_dir_all(&media).unwrap();
        fs::write(media.join("important.bin"), b"do-not-lose").unwrap();

        // Establish the symlink as build would.
        ensure_build_media(&build, &media).unwrap();
        assert!(build.join("media").join("important.bin").exists());

        // Run the real clean routine used by `build --clean`.
        crate::site_generator::setup_build_path(&build, &true).unwrap();

        // The build dir was wiped, but the media store MUST survive.
        assert!(
            media.join("important.bin").exists(),
            "clean must remove the symlink entry, never follow it into the media store"
        );
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --lib media:: 2>&1 | tail -20`
Expected: compile error — `ensure_build_media` not found. Expected RED.

- [ ] **Step 4: Write the implementation**

Add the implementation to `src/media.rs` (above the `#[cfg(test)]` module):

```rust
/// Ensure `build_path/media` resolves to the durable media store at
/// `media_path`, so the generated static site is self-contained while the
/// media itself lives outside the regeneratable `build/` directory.
///
/// - Creates `media_path` (and `build_path`) if missing.
/// - Absent `build_path/media` → creates an absolute symlink to `media_path`.
/// - Already a symlink → re-points it only if it targets the wrong place.
/// - Legacy real directory → migrates its contents into `media_path` (only when
///   `media_path` did not already exist) and replaces it with the symlink.
///   If BOTH the legacy dir and `media_path` hold media, returns an error
///   rather than risk clobbering.
/// - If symlink creation is unsupported/fails, copies `media_path` into
///   `build_path/media` and warns (non-fatal).
pub fn ensure_build_media(build_path: &Path, media_path: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(build_path)?;
    let link = build_path.join("media");
    let media_target = absolutize(media_path)?;
    let media_existed = media_path.exists();

    match fs::symlink_metadata(&link) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fs::create_dir_all(media_path)?;
            // Re-point only if it targets somewhere other than our media store.
            if fs::read_link(&link).ok().as_deref() != Some(media_target.as_path()) {
                fs::remove_file(&link)?; // removes the symlink entry, not the target
                symlink_dir(&media_target, &link)?;
            }
            return Ok(());
        }
        Ok(meta) if meta.file_type().is_dir() => {
            let legacy_nonempty = dir_has_entries(&link)?;
            let media_nonempty = media_existed && dir_has_entries(media_path)?;
            if legacy_nonempty && media_nonempty {
                return Err(format!(
                    "both {link:?} (legacy media directory) and {media_path:?} contain \
                     media; refusing to migrate automatically. Merge them into \
                     {media_path:?}, remove {link:?}, then re-run."
                )
                .into());
            }
            if media_existed {
                // media_path already present, legacy dir empty: drop the legacy dir.
                fs::remove_dir_all(&link)?;
            } else {
                // Promote the legacy dir to become the media store.
                if let Some(parent) = media_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&link, media_path)?;
            }
        }
        Ok(_) => {
            return Err(format!(
                "{link:?} exists but is neither a directory nor a symlink; \
                 remove it and re-run."
            )
            .into());
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Box::new(e)),
    }

    fs::create_dir_all(media_path)?;
    if let Err(e) = symlink_dir(&media_target, &link) {
        warn!("could not symlink {link:?} -> {media_target:?} ({e}); copying media instead");
        fs::create_dir_all(&link)?;
        copy_dir_contents(media_path, &link)?;
    }
    Ok(())
}

fn absolutize(p: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if p.is_absolute() {
        Ok(p.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(p))
    }
}

fn dir_has_entries(p: &Path) -> Result<bool, Box<dyn Error>> {
    Ok(fs::read_dir(p)?.next().is_some())
}

fn copy_dir_contents(from: &Path, to: &Path) -> Result<(), Box<dyn Error>> {
    let opts = fs_extra::dir::CopyOptions {
        overwrite: true,
        content_only: true,
        ..Default::default()
    };
    fs_extra::dir::copy(from, to, &opts)?;
    Ok(())
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib media:: 2>&1 | tail -20`
Expected: `test result: ok. 5 passed`.

- [ ] **Step 6: Confirm the whole suite and lints are clean**

Run: `cargo test 2>&1 | grep "test result" && cargo clippy --all-targets 2>&1 | tail -5`
Expected: all `test result: ok`; clippy prints no warnings.

- [ ] **Step 7: Commit**

```bash
git add src/media.rs src/lib.rs
git commit -m "feat: manage build/media symlink with safe migration to data/media"
```

---

## Task 3: Wire into `build` + document the change

**Files:**
- Modify: `src/cli/build.rs`
- Modify: `Changes.md`
- Create: `docs/dev-sessions/2026-07-23-1439-media-in-data-dir/notes.md`

**Interfaces:**
- Consumes: `fossilizer::media::ensure_build_media` (Task 2); `config.media_path()` (Task 1).
- Produces: `build` creates/maintains `build/media` → `data/media` on every run.

- [ ] **Step 1: Add `media` to the build imports**

In `src/cli/build.rs`, change:

```rust
use fossilizer::{config, db, site_generator, templates};
```

to:

```rust
use fossilizer::{config, db, media, site_generator, templates};
```

- [ ] **Step 2: Call `ensure_build_media` after `setup_build_path`**

In `src/cli/build.rs`, immediately after this existing line:

```rust
    site_generator::setup_build_path(&config.build_path, &clean)?;
```

add:

```rust
    media::ensure_build_media(&config.build_path, &config.media_path())?;
```

- [ ] **Step 3: Verify it compiles and the suite stays green**

Run: `cargo build 2>&1 | tail -5 && cargo test 2>&1 | grep "test result"`
Expected: build succeeds; all `test result: ok`. (This is the wiring's verification — the behavior itself is covered by Task 2's unit tests; `build`'s full path needs a populated DB and is not unit-tested here.)

- [ ] **Step 4: Add a Changes.md entry**

Read `Changes.md` first to match its existing format, then add a new top entry describing this change. Content to convey (match the file's style/heading conventions):

```
- Media now downloads to `data/media` (durable) instead of `build/media`.
  `build` creates `build/media` as a symlink to it, so the generated site is
  unchanged and `build --clean` no longer risks deleting media. Existing media
  in `build/media` is auto-migrated to `data/media` on the next `build`.
  Override the location with `APP_MEDIA_PATH`. Operators: back up `data/`
  (which now includes media); `build/` is fully regeneratable.
```

- [ ] **Step 5: Write the session notes**

Create `docs/dev-sessions/2026-07-23-1439-media-in-data-dir/notes.md`:

```markdown
# Media in data/ — Session Notes

## What changed
- `config.media_path()` now resolves to `<data_path>/media` by default
  (override `APP_MEDIA_PATH`), decoupled from `build_path`.
- New `fossilizer::media::ensure_build_media` maintains `build/media` as an
  absolute symlink to the media store, auto-migrates a legacy real
  `build/media` (non-destructively), and copies as a fallback where symlinks
  are unsupported.
- `build` calls it after `setup_build_path`.

## Safety
- Regression test `clean_build_does_not_delete_media_through_symlink` proves
  `build --clean` removes the symlink entry without following it into the
  media store.

## Operator migration
- First `build` after upgrade moves `build/media` → `data/media` automatically.
- For an in-flight transfer that already copied `build/media` to a server:
  `mv build/media data/media` before the first `build`.
- Backup surface is now just `data/`.

## Follow-ups / notes
- `serve` (warp) follows the symlink; no change needed. Confirm when validating.
- External publish (issue #15) will assemble/upload media from `data/media`.
```

- [ ] **Step 6: Commit**

```bash
git add src/cli/build.rs Changes.md docs/dev-sessions/2026-07-23-1439-media-in-data-dir/notes.md
git commit -m "feat: build maintains data/media symlink; document media relocation"
```

---

## Self-Review Notes

- **Spec coverage:** Part 1 (config `media_path`) → Task 1; Part 2 (`build` symlink + non-destructive migration + copy fallback) → Task 2 + wiring in Task 3; Part 3 (`--clean` safety invariant + regression test) → Task 2 Step 2 test `clean_build_does_not_delete_media_through_symlink`; operator migration notes → Task 3 (`Changes.md`, `notes.md`). All spec sections map to a task.
- **Absolute-target constraint:** enforced via `absolutize()` before every `symlink_dir` call and used in the re-point comparison — a spec/global constraint that would otherwise silently break relative `data_path` setups.
- **Type/name consistency:** `ensure_build_media(build_path: &Path, media_path: &Path) -> Result<(), Box<dyn Error>>` is defined in Task 2 and called with `(&config.build_path, &config.media_path())` in Task 3 (`&PathBuf` coerces to `&Path`). `media_path()` accessor signature from Task 1 is used unchanged.
- **No placeholders:** every code step contains complete code; tests contain real assertions.
- **Non-destructiveness:** the ambiguity branch errors before any move/remove; `remove_dir_all` in the migration only runs on an empty legacy dir (guarded by `media_existed` + the earlier both-populated check).
