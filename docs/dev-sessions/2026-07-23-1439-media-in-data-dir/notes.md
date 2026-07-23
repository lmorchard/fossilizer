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

## Known limitations
- On platforms without symlink support (e.g. Windows without the symlink
  privilege), `ensure_build_media` falls back to copying media into
  `build/media` instead of symlinking it. That copy is not idempotent across
  repeated builds: the next build sees `build/media` as a populated legacy
  directory alongside a populated `data/media` and hits the "refusing to
  migrate" ambiguity error, requiring `build/media` to be cleared manually
  first. The media relocation is designed with Unix/self-hosted deployments
  in mind.
