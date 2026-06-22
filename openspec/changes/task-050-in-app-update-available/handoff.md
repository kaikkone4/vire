<!-- Compact state; keep <= 2 KB. Reference paths, never paste content. -->

# Handoff — TASK-050 in-app update-available

- **Branch / PR:** `feat/task-050-in-app-update-available` / #38 (draft)
- **Phase:** SW-2 review fixes complete; ready for SW-4 re-review
- **Tier:** L2

## SW-4 blocker fixes

1. `check_for_update` is now async and moves the bounded blocking reqwest call to `tauri::async_runtime::spawn_blocking`; join failures remain fail-soft `Unknown`.
2. Rust tests call production payload parsing/version comparison helpers. Malformed JSON payload coverage verifies `Unknown`.
3. Settings privacy copy now distinguishes local records from explicit user-initiated network operations, including the GitHub update check.
4. Rust files are rustfmt-clean.

The non-blocking duplicate “Open release page” control and renderer data attribute were removed. `open_releases_page` still opens only the compile-time releases-index URL.

## Changed paths

- `src-tauri/src/update_check/mod.rs`
- `src-tauri/src/update_check/tests.rs`
- `src/main.ts`
- `src/update-check-ui.ts`
- `tests/updateCheckUi.test.mjs`

## Checks

- `cargo test update_check`: pass (9/9)
- `node --import tsx --test tests/updateCheckUi.test.mjs`: pass (12/12)
- `npm run build`: pass
- `cargo fmt --all -- --check`: pass
- `openspec validate task-050-in-app-update-available --strict`: pass
- scoped `git diff --check`: pass

## Next action

Route to SW-4 for re-review. Scope remains U-lite only: no updater/download/install/signing, launch auto-check, CSP change, or broad opener permission.
