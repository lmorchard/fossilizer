# Code Review — 2026-07-23

Thorough multi-agent code review of fossilizer (Rust Mastodon-export static-site generator, ~3200 LOC).

**Baseline:** build ✅ · clippy clean ✅ · 11 tests pass ✅ · v0.5.0

Reviewed in 6 parallel agent passes: CLI, Mastodon subsystem, DB, ActivityStreams/templates, site-gen/core, project hygiene/deps/CI.

## Cross-cutting themes

1. **Silent failure / exits-0** — CLI already fixed in unmerged branch `fix-cli-exit-code`. But many inner paths still `return Ok(())` on failure or swallow errors (maps to existing issue **#14 Improve error handling**).
2. **Panics on untrusted data** — infallible `From` impls `.unwrap()` on export/API JSON & URIs; importer/db `.unwrap()`s. A malformed export/response crashes import/fetch/build.
3. **Downloader robustness** — download failures silently discarded, no HTTP status check, no timeouts, new client per task, blocking IO in async.
4. **Security** — zip-slip in tar extraction; `| safe` on remote-authored HTML; `innerHTML` from alt-text.
5. **Dependency bloat** — dual reqwest 0.11+0.12 (→ dual hyper/http/rustls), openssl-vendored + rustls both compiled.
6. **Testability** — global mutable config singleton blocks unit testing of site-gen.
7. **Tooling gaps** — no Makefile, no clippy/fmt in CI, no cargo-audit.

## Low-hanging fruit (fix in this session)
- downloader: `.error_for_status()?` before writing body
- config.rs `init()`: `.unwrap()` → `?` (2 sites)
- db.rs `conn()`: `.unwrap()` → `?` (3 sites)
- cli/build.rs: `.unwrap()` → `?` throughout
- cli/serve.rs: parse/to_str `.unwrap()` → `?`/display
- cli/mastodon/code.rs: remove access-token `println!` (secret leak)
- importer: `import_zip` unwraps → `?`; unsupported extension → `Err`
- instance.rs `register_client_app`: `Ok(())` on failure → `Err`
- db/actors.rs `get_actors`: skip bad rows (warn) to match activities
- templates.rs `filter_urlpath`: fix wrong error label + unwraps
- activitystreams: `.or_else(|| Some()).unwrap()` → `unwrap_or_else`
- db count `i16` → `i64`; drop meaningless `ORDER BY` in count
- media-lightbox.js: `innerHTML` → `textContent`
- .gitignore: add archive patterns (`*.tgz` etc.) as guardrail for stray 2.9GB tarball
- Add Makefile (check/test/build/serve/docs)
- Add CI fmt+clippy lint job

## Larger concerns → GitHub issues (see issues.md for filed list)
- Panics on untrusted data (TryFrom refactor)
- Downloader robustness (errors/timeouts/streaming/shared client)
- Zip-slip path traversal in tar extraction (security)
- XSS: `| safe` on remote HTML + review escaping (security)
- DB transaction leak (`import_many`/`import_collection` need RAII txn)
- Dependency consolidation (reqwest 0.12, drop openssl)
- Global config testability
- Migration validation test + SQL double-quote-string convention
- Temp media dir leak on failed import
- Orphaned `src/cli/fetch.rs` (WIP for existing #12)

## GitHub issues filed
- #49 Security: zip-slip path traversal in tar/gz import
- #50 Security: sanitize/escape remote-authored HTML rendered with `| safe`
- #51 Downloader: surface failures, timeouts, shared client, stream to disk
- #52 Replace panicking `From` impls with `TryFrom` for untrusted data
- #53 DB: RAII transactions in import paths
- #54 Consolidate HTTP/TLS deps (reqwest 0.12, drop vendored openssl)
- #55 Reduce reliance on global mutable config (testability)
- #56 Migration validation test + single-quote SQL literals
- #57 Fix remote-actor URL derivation for federated accounts
- #58 Code-quality cleanup grab-bag (orphaned fetch.rs, no-op flags, temp-dir leak, docs, mime mapping, disabled template tests, cargo-audit)

Existing issues cross-referenced: #14 (error handling), #12 (outbox fetch → orphaned cli/fetch.rs), #35 (fetch-before-link panic).

## Fixes committed (this branch, worktree-code-review)
1. `850cee9` Replace panics with error propagation; stop swallowing failures (12 files)
2. `321d105` Render media descriptions as text (XSS)
3. `6b8c106` Add Makefile, CI lint gate, archive gitignore guardrail
4. `4b2313a` Session notes

Not committed / flagged for Les: the stray ~2.9 GB `build-20251205.tgz` and `backup-toots.sh` at the repo root (untracked, live in the main checkout, not the worktree). `.gitignore` now guards against committing root archives; the tarball itself should be deleted/moved manually.

## Notes / non-issues confirmed by agents
- rarray() queries are injection-safe (bound params)
- per-page db::conn() in rayon loop is intentional/correct
- downloader Notify shutdown protocol is correct
- pagefind/build/ are gitignored (not tracked) — only build-20251205.tgz is a risk
- Cargo.toml versions are caret minimums; Cargo.lock resolves upward
