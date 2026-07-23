# Docker Homelab Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a `Dockerfile` + `docker-compose.yml` that runs fossilizer on a homelab server, performing scheduled incremental backups of authorized Mastodon accounts and continuously serving the generated static archive.

**Architecture:** One multi-stage image (Rust release build + pinned pagefind binary + a backup-loop shell script) is shared by two long-running compose services: `web` (runs `fossilizer serve`) and `backup` (runs a sleep-loop that fetches every authorized instance, builds the site, and indexes it with pagefind). State lives in bind-mounted `./data` (SQLite DB + instance tokens) and `./build` (generated site + media).

**Tech Stack:** Docker (multi-stage build, buildx `TARGETARCH`), Docker Compose, Rust (`rust:1-bookworm` builder → `debian:bookworm-slim` runtime), POSIX `sh` for the loop, pagefind `v1.5.2`.

## Global Constraints

- Runtime base image: `debian:bookworm-slim` with `ca-certificates` only (binary is self-contained — `openssl` vendored, `rusqlite` bundled).
- Pagefind version pinned via `ARG PAGEFIND_VERSION=v1.5.2`; asset name pattern `pagefind-${PAGEFIND_VERSION}-${target}.tar.gz` from `https://github.com/CloudCannon/pagefind/releases/download/`, where `target` is `x86_64-unknown-linux-musl` (amd64) or `aarch64-unknown-linux-musl` (arm64).
- Config paths are driven by env, not a config file: `APP_DATA_PATH=/data`, `APP_BUILD_PATH=/build` (the `config` crate maps `APP_`-prefixed vars onto config keys; env source wins over any file).
- Fetch command matches the existing `backup-toots.sh`: `mastodon -i <host> fetch --max <FETCH_MAX> --incremental`, then `build`, then `pagefind --keep-index-url --site <build>`.
- Default env values: `WEB_PORT=8881`, `FETCH_MAX=5000`, `BACKUP_INTERVAL=86400`, `RUN_ON_START=true`.
- New files live at repo root (`Dockerfile`, `.dockerignore`, `docker-compose.yml`) except the loop script at `docker/backup-loop.sh`. Do not touch `.devcontainer/Dockerfile`.
- Bind mounts, not named volumes: `./data:/data`, `./build:/build`.
- Per-account fetch failures must log a warning and continue, never abort the run.

---

## File Structure

- `Dockerfile` (create) — three stages: `builder` (cargo release build), `pagefind` (download + verify pinned binary), `runtime` (slim image assembling binary + pagefind + loop script; `ENTRYPOINT ["fossilizer"]`).
- `.dockerignore` (create) — keep the build context small and secret-free.
- `docker/backup-loop.sh` (create) — the scheduled backup loop; the only piece with real logic worth unit-testing.
- `docker-compose.yml` (create) — `web` and `backup` services sharing image `fossilizer:local`.
- `docs/deployment.md` (create) — operator setup + operation notes.
- `README.md` (modify) — add a short pointer to `docs/deployment.md`.

---

## Task 1: Build context hygiene — `.dockerignore`

**Files:**
- Create: `.dockerignore`

**Interfaces:**
- Consumes: nothing.
- Produces: a lean, secret-free build context relied on by every `docker build` in later tasks (notably excludes the ~3 GB `build-20251205.tgz`, `target/`, and the `data/` tokens).

- [ ] **Step 1: Create `.dockerignore`**

```
# Build artifacts
/target
/build

# Local state and secrets (tokens live in data/)
/data
/tmp
tmp

# Large local archives and vendored tooling
*.tgz
/pagefind

# VCS and editor
.git
.gitignore
/.vscode
/.devcontainer

# Docs and CI not needed in the image
/docs
```

- [ ] **Step 2: Verify the heavy paths are ignored**

Run:
```bash
git check-ignore -v --no-index target build data build-20251205.tgz pagefind 2>/dev/null; \
grep -qxF '/target' .dockerignore && grep -qxF '*.tgz' .dockerignore && echo "DOCKERIGNORE_OK"
```
Expected: prints `DOCKERIGNORE_OK` (confirms the two most important excludes are present).

- [ ] **Step 3: Commit**

```bash
git add .dockerignore
git commit -m "build: add .dockerignore to keep image build context lean"
```

---

## Task 2: Backup loop script — `docker/backup-loop.sh`

This is the one component with branching logic (instance discovery, token filtering, graceful per-account failure, run-once vs loop). It has a testable seam via env-overridable binaries and a `BACKUP_INTERVAL<=0` single-run mode, so it is tested with stub binaries before the image ever builds.

**Files:**
- Create: `docker/backup-loop.sh`

**Interfaces:**
- Consumes: env vars `APP_DATA_PATH` (default `/data`), `APP_BUILD_PATH` (default `/build`), `FETCH_MAX` (default `5000`), `BACKUP_INTERVAL` (default `86400`), `RUN_ON_START` (default `true`), and test-only overrides `FOSSILIZER` (default `fossilizer`) / `PAGEFIND` (default `pagefind`).
- Produces: an executable `/usr/local/bin/backup-loop.sh` used as the `backup` service entrypoint in Task 4. Behavior relied on downstream: discovers `config-instance-*.toml` files that contain an `access_token`, derives `<host>` from the filename, runs fetch→build→pagefind, and exits 0 after one run when `BACKUP_INTERVAL<=0`.

- [ ] **Step 1: Write the failing behavioral test**

Create a temporary test harness (NOT committed — it exercises the script with stub binaries):

```bash
cat > /tmp/test-backup-loop.sh <<'TEST'
#!/bin/sh
set -eu
work=$(mktemp -d)
data="$work/data"; build="$work/build"; bin="$work/bin"; out="$work/calls.log"
mkdir -p "$data" "$build" "$bin"

# Authorized, live instance
printf 'host = "masto.example.social"\naccess_token = "tok1"\n' > "$data/config-instance-masto.example.social.toml"
# Authorized but dead instance (fetch will fail)
printf 'host = "dead.example.social"\naccess_token = "tok2"\n' > "$data/config-instance-dead.example.social.toml"
# Un-authorized instance (no access_token) -> must be skipped
printf 'host = "noauth.example.social"\n' > "$data/config-instance-noauth.example.social.toml"

# Stub fossilizer: log every call; fail fetch for the dead host
cat > "$bin/fossilizer" <<STUB
#!/bin/sh
echo "fossilizer \$*" >> "$out"
case "\$*" in
  *"-i dead.example.social"*) exit 1 ;;
esac
exit 0
STUB
# Stub pagefind
cat > "$bin/pagefind" <<STUB
#!/bin/sh
echo "pagefind \$*" >> "$out"
exit 0
STUB
chmod +x "$bin/fossilizer" "$bin/pagefind"

# Run one cycle (BACKUP_INTERVAL=0 => single run, no infinite loop)
APP_DATA_PATH="$data" APP_BUILD_PATH="$build" \
  FOSSILIZER="$bin/fossilizer" PAGEFIND="$bin/pagefind" \
  FETCH_MAX=1234 BACKUP_INTERVAL=0 RUN_ON_START=true \
  sh "${SCRIPT:?set SCRIPT to the backup-loop.sh path}"

echo "=== calls ==="; cat "$out"

grep -q '^fossilizer upgrade$' "$out"                                            || { echo "FAIL: no upgrade"; exit 1; }
grep -q 'mastodon -i masto.example.social fetch --max 1234 --incremental' "$out" || { echo "FAIL: live fetch missing"; exit 1; }
grep -q 'mastodon -i dead.example.social fetch' "$out"                           || { echo "FAIL: dead fetch not attempted"; exit 1; }
grep -q 'noauth.example.social' "$out"                                           && { echo "FAIL: unauthorized instance not skipped"; exit 1; }
grep -q '^fossilizer build$' "$out"                                              || { echo "FAIL: build not reached after failing fetch"; exit 1; }
grep -q "pagefind --keep-index-url --site $build" "$out"                         || { echo "FAIL: pagefind not run"; exit 1; }
echo "ALL_ASSERTIONS_PASSED"
rm -rf "$work"
TEST
chmod +x /tmp/test-backup-loop.sh
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `SCRIPT="$PWD/docker/backup-loop.sh" /tmp/test-backup-loop.sh`
Expected: FAIL — `sh: .../docker/backup-loop.sh: No such file or directory` (script not created yet).

- [ ] **Step 3: Write `docker/backup-loop.sh`**

```sh
#!/bin/sh
# Scheduled Mastodon backup loop for fossilizer.
# Discovers every authorized instance in the data dir, fetches incrementally,
# rebuilds the static site, and indexes it with pagefind — then sleeps.
set -eu

DATA_DIR="${APP_DATA_PATH:-/data}"
BUILD_DIR="${APP_BUILD_PATH:-/build}"
FOSSILIZER="${FOSSILIZER:-fossilizer}"
PAGEFIND="${PAGEFIND:-pagefind}"
FETCH_MAX="${FETCH_MAX:-5000}"
BACKUP_INTERVAL="${BACKUP_INTERVAL:-86400}"
RUN_ON_START="${RUN_ON_START:-true}"

log() {
  printf '%s %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$*"
}

run_backup() {
  log "backup run: starting"
  for cfg in "$DATA_DIR"/config-instance-*.toml; do
    [ -e "$cfg" ] || continue   # no matches -> glob stays literal; skip it
    if ! grep -q '^access_token' "$cfg"; then
      log "skipping $(basename "$cfg"): no access_token"
      continue
    fi
    base=$(basename "$cfg")
    host=${base#config-instance-}
    host=${host%.toml}
    log "fetching $host (max $FETCH_MAX, incremental)"
    if "$FOSSILIZER" mastodon -i "$host" fetch --max "$FETCH_MAX" --incremental; then
      log "fetched $host"
    else
      log "WARN: fetch failed for $host, continuing"
    fi
  done
  log "building static site"
  "$FOSSILIZER" build
  log "indexing site with pagefind"
  "$PAGEFIND" --keep-index-url --site "$BUILD_DIR"
  log "backup run: complete"
}

log "running database migrations"
"$FOSSILIZER" upgrade

if [ "$RUN_ON_START" = "true" ]; then
  run_backup
fi

if [ "$BACKUP_INTERVAL" -le 0 ]; then
  log "BACKUP_INTERVAL<=0: single-run mode, exiting"
  exit 0
fi

while true; do
  log "sleeping ${BACKUP_INTERVAL}s until next run"
  sleep "$BACKUP_INTERVAL"
  run_backup
done
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `sh -n docker/backup-loop.sh && SCRIPT="$PWD/docker/backup-loop.sh" /tmp/test-backup-loop.sh`
Expected: `sh -n` prints nothing (valid syntax); the harness prints the call log and ends with `ALL_ASSERTIONS_PASSED`.

- [ ] **Step 5: Commit**

```bash
git add docker/backup-loop.sh
git commit -m "feat: add scheduled Mastodon backup loop script"
```

---

## Task 3: Container image — `Dockerfile`

**Files:**
- Create: `Dockerfile`

**Interfaces:**
- Consumes: the repo source (Rust crate) and `docker/backup-loop.sh` from Task 2; `.dockerignore` from Task 1.
- Produces: image with `ENTRYPOINT ["fossilizer"]`, binaries at `/usr/local/bin/fossilizer` and `/usr/local/bin/pagefind`, script at `/usr/local/bin/backup-loop.sh`, and env `APP_DATA_PATH=/data` / `APP_BUILD_PATH=/build`. Consumed by both services in Task 4.

- [ ] **Step 1: Create `Dockerfile`**

```dockerfile
# syntax=docker/dockerfile:1

# ---- Builder: compile the release binary ----
FROM rust:1-bookworm AS builder
WORKDIR /usr/src/fossilizer
# buildpack-deps (rust image base) already provides gcc, make, perl, pkg-config,
# and libssl-dev needed for the vendored-openssl / bundled-sqlite build.
COPY . .
RUN cargo build --release --locked

# ---- Pagefind: fetch the pinned search-index binary ----
FROM debian:bookworm-slim AS pagefind
ARG PAGEFIND_VERSION=v1.5.2
ARG TARGETARCH
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN set -eux; \
    case "$TARGETARCH" in \
      amd64) target=x86_64-unknown-linux-musl ;; \
      arm64) target=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported TARGETARCH: ${TARGETARCH:-unset}" >&2; exit 1 ;; \
    esac; \
    url="https://github.com/CloudCannon/pagefind/releases/download/${PAGEFIND_VERSION}/pagefind-${PAGEFIND_VERSION}-${target}.tar.gz"; \
    curl -fsSL "$url" -o /tmp/pagefind.tar.gz; \
    tar -xzf /tmp/pagefind.tar.gz -C /usr/local/bin; \
    chmod +x /usr/local/bin/pagefind; \
    /usr/local/bin/pagefind --version

# ---- Runtime: assemble the slim final image ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/fossilizer/target/release/fossilizer /usr/local/bin/fossilizer
COPY --from=pagefind /usr/local/bin/pagefind /usr/local/bin/pagefind
COPY docker/backup-loop.sh /usr/local/bin/backup-loop.sh
RUN chmod +x /usr/local/bin/backup-loop.sh
ENV APP_DATA_PATH=/data \
    APP_BUILD_PATH=/build
ENTRYPOINT ["fossilizer"]
```

- [ ] **Step 2: Build the image**

Run: `docker build -t fossilizer:local .`
Expected: build succeeds through all three stages; the pagefind stage prints a version line like `Pagefind 1.5.2 ...`. (First build compiles the Rust crate — several minutes.)

- [ ] **Step 3: Verify the runtime image contents**

Run:
```bash
docker run --rm fossilizer:local --version && \
docker run --rm --entrypoint pagefind fossilizer:local --version && \
docker run --rm --entrypoint sh fossilizer:local -c 'test -x /usr/local/bin/backup-loop.sh && echo SCRIPT_OK'
```
Expected: prints the fossilizer version, the pagefind version, and `SCRIPT_OK`.

- [ ] **Step 4: Commit**

```bash
git add Dockerfile
git commit -m "feat: add multi-stage Dockerfile with fossilizer + pagefind"
```

---

## Task 4: Compose stack — `docker-compose.yml`

**Files:**
- Create: `docker-compose.yml`

**Interfaces:**
- Consumes: image built from Task 3 (`build: context: .`, tagged `fossilizer:local`); the loop entrypoint from Task 2; bind mounts `./data`, `./build`.
- Produces: `web` service (published on `WEB_PORT`) and `backup` service (scheduled loop). Terminal deliverable of the deployment.

- [ ] **Step 1: Create `docker-compose.yml`**

```yaml
services:
  web:
    build:
      context: .
    image: fossilizer:local
    command: ["serve", "--host", "0.0.0.0", "--port", "8881"]
    volumes:
      - ./build:/build:ro
    ports:
      - "${WEB_PORT:-8881}:8881"
    restart: unless-stopped

  backup:
    build:
      context: .
    image: fossilizer:local
    entrypoint: ["/usr/local/bin/backup-loop.sh"]
    environment:
      FETCH_MAX: "${FETCH_MAX:-5000}"
      BACKUP_INTERVAL: "${BACKUP_INTERVAL:-86400}"
      RUN_ON_START: "${RUN_ON_START:-true}"
    volumes:
      - ./data:/data
      - ./build:/build
    restart: unless-stopped
```

Note: `web` mounts only `./build:ro` — `serve` reads the build dir via `APP_BUILD_PATH` and never touches the DB or tokens, so `data` is intentionally not mounted there.

- [ ] **Step 2: Validate compose config**

Run: `docker compose config >/dev/null && echo COMPOSE_OK`
Expected: prints `COMPOSE_OK` (YAML parses, image/build/volumes resolve). Warnings about the default `WEB_PORT` env being unset are fine.

- [ ] **Step 3: End-to-end — serve the existing build**

Run:
```bash
docker compose up -d web
sleep 3
curl -sSf -o /dev/null -w '%{http_code}\n' http://localhost:8881/
docker compose logs web | tail -n 5
docker compose down
```
Expected: `curl` prints `200` (the existing `./build/` index is served). Logs show `Serving up ... at http://0.0.0.0:8881`.

- [ ] **Step 4: End-to-end — one backup cycle**

Run (single-run mode so it exits after one pass; uses the real authorized tokens already in `./data`):
```bash
BACKUP_INTERVAL=0 FETCH_MAX=50 docker compose run --rm \
  -e BACKUP_INTERVAL=0 -e FETCH_MAX=50 backup
```
Expected: logs show `running database migrations`, a `fetching <host>` line per authorized instance in `./data` (a dead/unauthorized instance logs `WARN: fetch failed ... continuing` rather than aborting), `building static site`, `indexing site with pagefind`, `backup run: complete`, then `single-run mode, exiting` with exit code 0. `./data/data.sqlite3` and `./build/` are updated.

- [ ] **Step 5: Commit**

```bash
git add docker-compose.yml
git commit -m "feat: add docker-compose stack for serve + scheduled backup"
```

---

## Task 5: Operator documentation — `docs/deployment.md`

**Files:**
- Create: `docs/deployment.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: the behavior and env knobs defined in Tasks 2–4.
- Produces: setup/operation docs. No code depends on this.

- [ ] **Step 1: Create `docs/deployment.md`**

````markdown
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
````

- [ ] **Step 2: Add a pointer in `README.md`**

Append this section to the end of `README.md`:

```markdown
## Homelab deployment

To run scheduled backups and serve the archive with Docker, see
[docs/deployment.md](docs/deployment.md).
```

- [ ] **Step 3: Verify links resolve**

Run: `test -f docs/deployment.md && grep -q 'docs/deployment.md' README.md && echo DOCS_OK`
Expected: prints `DOCS_OK`.

- [ ] **Step 4: Commit**

```bash
git add docs/deployment.md README.md
git commit -m "docs: add homelab deployment guide"
```

---

## Self-Review Notes

- **Spec coverage:** image/multi-stage → Task 3; `.dockerignore` (incl. 3 GB tgz) → Task 1; two-service compose w/ bind mounts + `WEB_PORT` → Task 4; backup loop w/ auto-discovery, graceful failure, sleep-loop, env knobs → Task 2; pagefind pinned + arch mapping → Tasks 1/3; `db::upgrade` bootstrap → Task 2 script; no-DB-contention serve (build:ro only) → Task 4; accepted in-place-build caveat → documented in Task 5. All spec sections map to a task.
- **Env-driven config:** `APP_DATA_PATH`/`APP_BUILD_PATH` set in the image (Task 3) and honored by the script/services; no config file required.
- **Type/name consistency:** env var names, file paths, and command signatures are identical across the script (Task 2), Dockerfile (Task 3), compose (Task 4), and docs (Task 5).
