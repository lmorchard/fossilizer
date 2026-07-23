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
