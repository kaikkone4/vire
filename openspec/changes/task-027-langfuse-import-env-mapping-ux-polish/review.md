# Code Review — TASK-027

**Verdict:** PASS

## Blocking Issues

None.

## Suggestions

- None.

## Prior Blockers Rechecked

- `RELEASE.md` now covers TASK-027 Workstreams A-E, including import diagnostics, auto-import, environment discovery/picker, environment→project mapping, desktop chrome/icon polish, rollback, and the TASK-028 self-update split. Key coverage is at `RELEASE.md:65`, `RELEASE.md:71`, `RELEASE.md:77`, `RELEASE.md:82`, `RELEASE.md:89`, `RELEASE.md:95`, and `RELEASE.md:103`.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` is clean.

## Architecture / Scope Check

- A — import diagnostics and payload tolerance stay in the Langfuse importer boundary. `ImportOutcome` returns the health snapshot plus `ImportReport` at `src-tauri/src/lib.rs:732`, and current `usageDetails`/`costDetails` parsing preserves absence as `None` at `src-tauri/src/langfuse/model.rs:177` and `src-tauri/src/langfuse/model.rs:231`.
- B — auto-import is a trigger for the existing importer, not a second evidence authority. The startup/periodic thread starts after `init_db` at `src-tauri/src/lib.rs:806`, serializes via the import slot at `src-tauri/src/lib.rs:666`, respects the disabled switch before probing at `src-tauri/src/lib.rs:697`, and calls the same `run_blocking_import` path at `src-tauri/src/lib.rs:710`.
- C — discovery uses the existing read-only Langfuse API root and drops only the `environment` filter. The URL builder keeps `TracesAllEnvironments` under `/api/public/traces` at `src-tauri/src/langfuse/config.rs:91`, and discovery persists only environment names via `src-tauri/src/langfuse/store.rs:235`.
- D — mapping is additive and suggestion-first. The table is created via `langfuse_env_project_map` at `src-tauri/src/env_mapping/mod.rs:32`, `set_env_mapping_repo` requires an existing project and never creates one at `src-tauri/src/env_mapping/mod.rs:134`, and evidence association is a read-time LEFT JOIN at `src-tauri/src/env_mapping/mod.rs:208`.
- E — fake traffic-light markup is removed from the shell; the extracted titlebar emits only brand/version at `src/shell-chrome.ts:8`, and the titlebar CSS is rebalanced at `src/style.css:1`.
- No CSP/capabilities/updater drift found: `src-tauri/capabilities/default.json:6` remains core/dialog only, `src-tauri/tauri.conf.json:14` keeps renderer network limited to IPC, and `src-tauri/Cargo.toml:16` has no updater plugin.

## Verification

- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` — PASS.
- `cargo test --manifest-path src-tauri/Cargo.toml` — PASS, 120 unit tests + 3 adversarial tests.
- `node --import tsx --test tests/envMappingUi.test.mjs tests/shellChrome.test.mjs tests/langfuseSettings.test.mjs` — PASS, 15 focused frontend tests.
- `npm run build` — PASS.
- `git diff --check origin/main...HEAD` — PASS.
- `npm run test:frontend` — FAIL only in 4 `tests/pi-observe.security.test.mjs` cases that attempt to bind `127.0.0.1` and hit sandbox `EPERM` at `tests/pi-observe.security.test.mjs:60`, `tests/pi-observe.security.test.mjs:73`, `tests/pi-observe.security.test.mjs:86`, and `tests/pi-observe.security.test.mjs:109`. `git diff --name-only origin/main...HEAD -- tests/pi-observe.security.test.mjs observability/pi-observe/bin/pi-observe.mjs` and `git log --oneline origin/main..HEAD -- tests/pi-observe.security.test.mjs observability/pi-observe/bin/pi-observe.mjs` both returned no output, so these failures are unchanged/unrelated to TASK-027.

## Gate Verdict

PASS — craft/conventions pass, prior blockers are fixed, A-E remain within the approved boundaries, and no CSP/capabilities/updater/new-egress drift was introduced. Proceed to the parallel SW-5/SW-6 flow.
