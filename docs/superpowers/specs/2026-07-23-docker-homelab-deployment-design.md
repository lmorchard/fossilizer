# Fossilizer Homelab Deployment — Design

**Date:** 2026-07-23
**Goal:** Run fossilizer on a homelab server to perform scheduled backups of Mastodon
accounts and continuously serve the generated static archive over HTTP, via a
`Dockerfile` and `docker-compose.yml`.

## Background

Fossilizer is a Rust CLI that archives Mastodon accounts into a SQLite database
(`data/data.sqlite3`) plus downloaded media, and generates a static HTML site into
`build/`. The relevant facts driving this design:

- **Auth** is per-instance, stored in `data/config-instance-<host>.toml` (contains the
  access token). Three instances are already authorized: `hackers.town`,
  `masto.hackers.town`, `mastodon.social`.
- **Backup pipeline** (from the existing ad-hoc `backup-toots.sh`):
  `mastodon -i <host> fetch --max 5000 --incremental` → `build` → `pagefind`.
- **Config** is read from `data/config.toml` (optional) plus `APP_`-prefixed environment
  variables — the `config` crate maps e.g. `APP_DATA_PATH` → `data_path`,
  `APP_BUILD_PATH` → `build_path`.
- **Themes/templates fall back to embedded resources** when no on-disk theme directory
  exists (`templates.rs`, `site_generator.rs`), so `init` is **not** required to build.
- **`db::upgrade()` runs migrations** and is idempotent; a fresh DB needs it before the
  first fetch, an existing DB is unaffected.
- **`serve` only reads static files** from `build/` — it never opens the SQLite DB, so
  there is **no DB contention** between the serving and backup containers.
- The binary is self-contained: `openssl` is `vendored`, `rusqlite` is `bundled`. The
  only runtime dependency is `ca-certificates` for TLS trust.

## Architecture

A two-service Docker Compose stack sharing a single image. The services differ only by
the command they run.

```
                 ┌─────────────────────────────────────┐
                 │  fossilizer image (multi-stage)      │
                 │  /usr/local/bin/fossilizer           │
                 │  /usr/local/bin/pagefind             │
                 │  /usr/local/bin/backup-loop.sh       │
                 └─────────────────────────────────────┘
                        │                       │
         ┌──────────────┘                       └──────────────┐
         ▼                                                      ▼
  ┌──────────────┐                                      ┌──────────────┐
  │  web         │  serve --host 0.0.0.0 --port 8881    │  backup      │  backup-loop.sh
  │  (long-run)  │  mounts data:ro, build:ro            │  (long-run)  │  mounts data:rw, build:rw
  │  publishes   │                                      │              │
  │  :8881       │                                      │              │
  └──────────────┘                                      └──────────────┘
         │                                                      │
         └───────────────────┬──────────────────────────────────┘
                             ▼
              bind mounts: ./data:/data   ./build:/build
```

## Components

### Image — `Dockerfile` (multi-stage)

1. **Builder** — `rust:1-bookworm`; `cargo build --release`. Produces a self-contained
   binary (vendored openssl, bundled sqlite).
2. **pagefind fetch** — download the pinned `pagefind_extended` release tarball matching
   `$TARGETARCH` (map `amd64`→`x86_64-unknown-linux-musl`,
   `arm64`→`aarch64-unknown-linux-musl`) so the image builds on both x86 and ARM homelab
   hardware.
3. **Runtime** — `debian:bookworm-slim` + `ca-certificates`. Copies in the `fossilizer`
   binary, the `pagefind` binary, and `backup-loop.sh`. Sets
   `ENV APP_DATA_PATH=/data APP_BUILD_PATH=/build`.

### `.dockerignore`

Excludes the build context bloat and state that must not leak into the image:
`target/`, `data/`, `build/`, `tmp/`, `*.tgz` (there is a ~3 GB `build-20251205.tgz` in
the repo root), `.git/`, `pagefind` (the vendored binary in the repo root).

### `docker-compose.yml`

- **`web`**
  - command: `serve --host 0.0.0.0 --port 8881`
  - volumes: `./data:/data:ro`, `./build:/build:ro`
  - ports: `${WEB_PORT:-8881}:8881`
  - `restart: unless-stopped`
- **`backup`**
  - command: `/usr/local/bin/backup-loop.sh`
  - volumes: `./data:/data`, `./build:/build`
  - environment: `FETCH_MAX` (default 5000), `BACKUP_INTERVAL` (default 86400),
    `RUN_ON_START` (default true)
  - `restart: unless-stopped`

Bind mounts (not named volumes) are used so the operator can drop the existing `data/`
(DB + instance tokens) directly in place, and so homelab file-level backups see the data
transparently.

### `backup-loop.sh`

```sh
fossilizer upgrade                 # idempotent migration bootstrap

run_backup() {
  for cfg in /data/config-instance-*.toml; do
    [ -e "$cfg" ] || continue
    grep -q '^access_token' "$cfg" || continue          # skip un-authorized configs
    host=<strip prefix "config-instance-" and suffix ".toml" from basename>
    if fossilizer mastodon -i "$host" fetch --max "$FETCH_MAX" --incremental; then
      log "fetched $host"
    else
      log "WARN: fetch failed for $host, continuing"      # graceful failure
    fi
  done
  fossilizer build
  pagefind --keep-index-url --site /build
}

[ "$RUN_ON_START" = "true" ] && run_backup
while true; do
  sleep "$BACKUP_INTERVAL"
  run_backup
done
```

- **Account discovery:** iterate every `config-instance-*.toml` that contains an
  `access_token`. Adding an account = authorize it once; removing = delete its config
  file. No hand-maintained list.
- **Graceful failure:** a failed fetch (dead instance, auth error, network) logs a warning
  and moves on; it does not abort the run or crash the loop.
- **Scheduling:** plain sleep-loop (interval seconds), not a crontab. Logs go to
  `docker logs backup`. Wall-clock scheduling ("run at 4am") is explicitly out of scope;
  if wanted later, swap the loop for busybox `crond`.

## Data flow

1. `backup` container starts → `fossilizer upgrade` migrates the DB (no-op if current).
2. On start (if `RUN_ON_START`) and every `BACKUP_INTERVAL` thereafter:
   fetch each authorized account incrementally into `/data/data.sqlite3` and download
   media into `/build/media` → `fossilizer build` regenerates `/build` → `pagefind`
   indexes `/build`.
3. `web` container continuously serves `/build` over HTTP on port 8881.

## Error handling

- Per-account fetch failures are caught and logged; the loop continues.
- `restart: unless-stopped` recovers both containers from crashes / host reboots.
- **Accepted caveat:** `build` overwrites `/build` in place with no atomic swap, so `web`
  may briefly serve a half-regenerated page during a build. Acceptable for a
  non-mission-critical personal archive; not engineered around.

## Testing / verification

- `docker compose build` succeeds (image builds, pagefind arch resolves).
- `docker compose up web` serves the existing `build/` at `http://<host>:8881`.
- `backup` container with a short `BACKUP_INTERVAL` runs one full cycle: logs show a fetch
  per authorized instance (with the dead one logging a warning, not crashing), a build,
  and a pagefind index; `data.sqlite3` and `build/` are updated.

## Out of scope

- Wall-clock cron scheduling.
- Atomic build/publish swap.
- Reverse proxy / TLS config (operator's existing homelab proxy fronts port 8881).
- Multi-arch image publishing to a registry (built locally on the homelab host).
