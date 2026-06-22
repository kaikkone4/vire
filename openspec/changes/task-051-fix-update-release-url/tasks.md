# Tasks — TASK-051 hotfix: correct update-check repo owner

Single component (the **Vire desktop app**). String-correction hotfix + regression guard.
**No** feature change, version bump, schema/migration/CSP/capture/classifier/Langfuse change, and
**no** `tauri-plugin-updater`. Generated files (`src-tauri/gen/schemas/capabilities.json`,
`src-tauri/target/**`) are gitignored — do **not** hand-edit or commit them; they regenerate at build.

## 1. Correct the two Rust-core constants

- [x] `src-tauri/src/update_check/mod.rs:23` — set `RELEASES_URL` to
      `https://github.com/kaikkone4/vire/releases`.
- [x] `src-tauri/src/update_check/mod.rs:26` — set `GITHUB_API_LATEST` to
      `https://api.github.com/repos/kaikkone4/vire/releases/latest`.

## 2. Correct the scoped opener capability allowlist

- [x] `src-tauri/capabilities/default.json:12` — set the `opener:allow-open-url` `allow[].url` to
      `https://github.com/kaikkone4/vire/releases`. Keep it host/path-scoped; do **not** add
      `opener:default` / `opener:allow-default-urls`. The allowlist URL MUST equal `RELEASES_URL`.

## 3. Align test fixtures to the canonical owner

- [x] `src-tauri/src/update_check/tests.rs:9,27` — replace `kaikkonen4` with `kaikkone4` in the
      `html_url` fixtures.
- [x] `tests/updateCheckUi.test.mjs:41,70` — replace `kaikkonen4` with `kaikkone4` in the
      `release_url` fixtures.

## 4. Add a regression guard (the new invariant)

- [x] Add a Rust unit test (in `update_check/tests.rs`) asserting both `RELEASES_URL` and
      `GITHUB_API_LATEST` contain `"kaikkone4/vire"` and contain **no** `"kaikkonen4"` substring.
      A `const`/compile-time assertion is acceptable if preferred over a `#[test]`.
- [x] (Optional, cheap) Assert the `opener:allow-open-url` allow URL in `capabilities/default.json`
      equals `RELEASES_URL` — guards the capability/constant from drifting apart again.

## 5. Verify (proportional to scope)

- [x] `cargo test update_check` passes (existing 9 + new guard).
- [x] `node --import tsx --test tests/updateCheckUi.test.mjs` passes.
- [x] `npm run build` passes; `cargo fmt --all -- --check` passes.
- [x] `grep -rn "kaikkonen4" src-tauri/src src-tauri/capabilities src tests` returns **no** matches
      (build artifacts under `target/`/`gen/` may still show stale copies until rebuilt — ignore them).
- [x] `openspec validate task-051-fix-update-release-url --strict` passes.
- [x] **Local smoke before release:** corrected-owner releases page and repo API both resolve
      (`https://github.com/kaikkone4/vire/releases` → 200, `api.github.com/repos/kaikkone4/vire`
      → 200) — the typo previously 404'd. `…/releases/latest` is 404 *only* because no release is
      published yet (legitimate "no release" → fail-soft `Unknown`, unrelated to the owner). Once a
      release is published, Check-for-updates will resolve to UpToDate / UpdateAvailable.
