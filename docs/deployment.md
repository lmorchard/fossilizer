# Homelab Deployment

Run fossilizer on a server to back up your Mastodon accounts on a schedule and
serve the generated archive.

## What it runs

- **`web`** — serves the static archive from `./build` on `WEB_PORT` (default 8881).
  Put your own reverse proxy / TLS in front of it.
- **`backup`** — a loop that, on start and every `BACKUP_INTERVAL` seconds:
  fetches each authorized instance incrementally, rebuilds the site, and updates
  the pagefind search index.

## First-time setup

1. Authorize each account once (from a checkout, or `docker compose run --rm
   --entrypoint fossilizer backup mastodon -i <host> link`, then `... code <code>`).
   Tokens are written to `./data/config-instance-<host>.toml`.
2. Copy your existing `./data` (SQLite DB + instance tokens) into place if you have
   one; otherwise the first run bootstraps a fresh DB.
3. `docker compose up -d`

## Configuration (env / `.env`)

| Variable          | Default | Meaning                                             |
|-------------------|---------|-----------------------------------------------------|
| `WEB_PORT`        | `8881`  | Host port the archive is served on.                 |
| `FETCH_MAX`       | `5000`  | Max statuses fetched per account per run.           |
| `BACKUP_INTERVAL` | `86400` | Seconds between runs. `0` or less = one run, then exit. |
| `RUN_ON_START`    | `true`  | Run a backup immediately on container start.        |

## Accounts

The backup loop auto-discovers every `./data/config-instance-*.toml` that contains
an `access_token`. Add an account by authorizing it; remove one by deleting its
config file. A dead or unauthorized instance logs a warning and is skipped — it
does not stop the run.

## Operating

- Logs: `docker compose logs -f backup` / `docker compose logs -f web`
- One-off backup now: `docker compose run --rm -e BACKUP_INTERVAL=0 backup`
- Data to back up externally: the `./data` directory (DB + tokens). `./build` is
  regeneratable output.

## Caveat

`build` overwrites `./build` in place with no atomic swap, so `web` may briefly serve
a half-regenerated page during a build. Harmless for a personal archive.
