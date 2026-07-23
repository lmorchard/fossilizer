# Media Storage in `data/` — Design

**Date:** 2026-07-23
**Goal:** Move downloaded Mastodon media out of the regeneratable `build/` directory and make it durable state in `data/media`, with `build/media` provided as a symlink so the generated static site stays self-contained.

## Problem

Fossilizer downloads attachment media directly into `build/media` (because
`config.media_path()` is derived as `build_path/media`). This couples
irreplaceable state to a directory that is otherwise regeneratable output:

- **`build --clean` deletes all media.** `site_generator::setup_build_path`
  runs `fs::remove_dir_all(build_path)` when `--clean` is passed, wiping
  `build/media` along with the generated HTML. Today this is only survivable
  because the scheduled backup loop calls plain `build`; any manual `build -k`
  destroys the archive's media.
- **Media is effectively irreplaceable.** Media is downloaded only during
  `fetch`, per new status. Because fetches run `--incremental` (stopping once
  they reach already-imported statuses), media for already-archived posts is
  never re-downloaded, and `build` only references it — it never fetches.
- **The mental model is a trap.** A user who reasonably assumes `build/` is
  ephemeral and deletes it loses 3+ GB of media.

## Constraints and context

- **Static-site-generator identity must be preserved.** Templates reference
  media as `<site_root>/media/<actor_hash>/…` (`activity.html`), so the served
  and published site must resolve media under `build/media`.
- **No byte-duplication on the normal (self-hosted) path.** The original
  design avoided a `data/media → build/media` copy; that concern stands.
- **All downloads already funnel through `config.media_path()`** — call sites
  in `src/cli/fetch.rs`, `src/cli/mastodon/fetch.rs`, and `src/cli/import.rs`.
  Redefining that one function redirects every download.
- **`config.rs` already flags this**: `// todo: allow each of these to be
  individually overriden` sits directly above `media_path()`.
- **Deferred:** external publishing (issue #15). When a publish command lands,
  it can assemble/upload media from `data/media` itself. Portable-to-static-host
  output is therefore out of scope here.

## Design

### Part 1 — `media_path` as a first-class configurable path

In `src/config.rs`:

- Add a `media_path: PathBuf` field to `AppConfig` with a serde default of
  `data_path/media` (a `default_media_path`-style helper, matching the existing
  `default_build_path` / `default_data_path` pattern). Overridable via
  `APP_MEDIA_PATH`, consistent with the other `APP_`-prefixed paths.
- Change the `media_path()` accessor to return this field instead of
  `build_path.join("media")`.

Because the three download call sites already call `config.media_path()`, no
other call-site changes are needed for downloads to land in `data/media`.

### Part 2 — `build` provides `build/media` as a symlink

`build` gains one idempotent step, run on **every** build (after
`setup_build_path`, regardless of whether `--clean` was passed), that ensures
`build/media` resolves to the media source:

1. If `build/media` is **missing** → create a symlink `build/media → <absolute media_path>`.
2. If `build/media` is **already the correct symlink** → leave it.
3. If `build/media` is a **legacy real directory** (pre-migration state):
   - If `data/media` (the configured `media_path`) does **not** exist → move
     `build/media`'s contents to `media_path`, then replace `build/media` with
     the symlink. (This is the auto-migration.)
   - If `media_path` **already exists** and `build/media` is a populated real
     directory → **error out** with a clear, actionable message rather than
     merge or clobber. This ambiguous state must never destroy data.
4. **Cross-platform fallback:** attempt the symlink on all platforms; if symlink
   creation fails (e.g. Windows without the privilege, or a filesystem that
   cannot symlink) → fall back to **copying** `media_path` into `build/media`
   and log a warning. This keeps the tool functional everywhere while giving the
   zero-duplication symlink on Unix/self-hosted deployments.

`media_path` itself is created (empty) if absent so the symlink always has a
valid target.

### Part 3 — Safety invariant: `--clean` must not follow the symlink

**`build --clean` must delete `build/` without deleting the contents of
`data/media` through the `build/media` symlink.**

Rust's `std::fs::remove_dir_all` removes a symlink entry without recursing into
its target, so `setup_build_path`'s existing `remove_dir_all(build_path)` is
correct. This behavior is load-bearing, so it is protected by an explicit
regression test (see Testing). No code change to `setup_build_path` is required;
the test guards against a future well-meaning change that follows the link.

## Data flow (after change)

1. `fetch` / `import` download media to `config.media_path()` = `data/media`.
2. `build` regenerates `build/` and ensures `build/media` symlinks to
   `data/media` (migrating a legacy real `build/media` on first run).
3. `serve` (warp `fs::dir(build)`) and external servers (nginx/Caddy) follow the
   symlink to serve media. (Warp's symlink-following to be confirmed in
   implementation; expected to work.)
4. `build --clean` wipes `build/` (including the symlink entry) and leaves
   `data/media` intact; the next `build` re-creates the symlink.

## Error handling

- **Ambiguous migration** (both a populated real `build/media` and an existing
  `media_path`): hard error with a message telling the user to reconcile
  manually. No silent merge, no clobber.
- **Symlink unsupported/failed:** warn and fall back to copy; not fatal.
- **Missing `media_path`:** created empty; not an error.

## Testing

- **Config:** `media_path()` defaults to `data_path/media`; `APP_MEDIA_PATH`
  override is honored.
- **Symlink creation:** after `build`, `build/media` is a symlink whose target
  is `media_path`, and a file placed in `media_path` is readable via
  `build/media/<file>`.
- **Auto-migration:** given a legacy real `build/media` with a file and no
  `media_path`, `build` moves the file to `media_path` and replaces
  `build/media` with the symlink.
- **Ambiguity guard:** given both a populated real `build/media` and an existing
  `media_path`, `build` errors and destroys nothing.
- **Safety invariant (Part 3):** create `media_path` containing a file, symlink
  `build/media` to it, run the clean routine, assert the file in `media_path`
  still exists.
- Existing test suite continues to pass.

## Migration notes (operators)

- Existing users' media lives in `build/media`; the first `build` after this
  change auto-migrates it to `data/media` (the migration is part of `build`'s
  symlink step, not the DB `upgrade`).
- For an in-flight homelab transfer that copied `build/media` to the server, the
  simplest reconciliation is `mv build/media data/media` on the server before
  the first `build`; the symlink is then created automatically.
- After this change, the durable backup surface is **just `data/`** (`data/media`
  plus the DB and instance tokens); `build/` is fully disposable.

## Out of scope

- External publishing / upload (issue #15).
- Producing a symlink-free portable `build/` tree for static hosts (a future
  publish command's concern).
- Changing `serve` to serve media from a separate route (the symlink keeps the
  single-root model intact).
