<!-- Compact state; keep <= 2 KB. Reference paths, never paste content. -->

# Handoff — TASK-050 in-app update-available + Open Releases (U-lite)

- **Change dir**: openspec/changes/task-050-in-app-update-available/
- **Branch / PR**: `feat/task-050-in-app-update-available` / PR #38 (draft)
- **Phase / gate**: SW-2 (**COMPLETE**) → SW-3 QA next
- **Tier**: L2

## Last gate result

SW-2 implementation COMPLETE (2026-06-22, Sonnet 4.6).

## What was implemented

| Component | File(s) |
|-----------|---------|
| Rust module | `src-tauri/src/update_check/mod.rs` (check_for_update + open_releases_page) |
| Rust tests | `src-tauri/src/update_check/tests.rs` (8 unit tests, no live network) |
| Cargo deps | `tauri-plugin-opener = "2"`, `semver = "1"` in `src-tauri/Cargo.toml` |
| Capability | `opener:allow-open-url` scoped to `github.com/kaikkone4/vire/releases` |
| lib.rs | opener plugin init + two commands in invoke_handler |
| Frontend builder | `src/update-check-ui.ts` (idle/pending/up_to_date/update_available/unknown states) |
| Frontend wiring | `src/main.ts` — import, state, panel in renderSettings, bindUpdateCheck |
| Frontend tests | `tests/updateCheckUi.test.mjs` (12 tests: all states + XSS escaping) |

## Scope compliance

- Renderer CSP `connect-src` **unchanged** (loopback-only): egress is Rust-side only.
- No `tauri-plugin-updater`, no signing, no binary download/install.
- Auto-check default-OFF (DEC-038 recommended value).
- Capability is scoped `opener:allow-open-url`, never `opener:default`.
- No schema, migration, capture, classifier, or Langfuse changes.

## Test results

- `cargo test`: **241 passed** (8 new update_check tests)
- `node --import tsx --test tests/*.test.mjs`: **115 passed**, 12 new; 2 pre-existing pi-observe.security failures unrelated

## Active blockers

- **None.** FB-1 (SEC-002 boundary amendment) and FB-2 (DEC-038 auto-check default) are parallel
  BA/Janne routing items — non-blocking for QA gate.

## Exact next action

Pi-Assistant: route to **sw-qa-engineer** for SW-3 gate.

## Required files for QA

- `src-tauri/src/update_check/mod.rs` — HTTP client, SemVer compare, commands
- `src-tauri/src/update_check/tests.rs` — unit tests
- `src/update-check-ui.ts` — frontend builder
- `tests/updateCheckUi.test.mjs` — frontend tests
- `src-tauri/capabilities/default.json` — scoped capability
- `src-tauri/src/lib.rs:1173-1237` — plugin + invoke_handler
- `src-tauri/tauri.conf.json:14` — CSP unchanged confirmation
