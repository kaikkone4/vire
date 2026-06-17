# Vire — Release Notes

## v0.1.0 — Desktop production readiness (TASK-026)

**Branch:** `feat/task-026-desktop-production-readiness`

### What changed

**In-app Langfuse settings with secure secret storage (Workstream A)**

- The Settings view now includes a **Langfuse integration panel** where you can configure base URL, source (local/cloud), environments, public key, secret key, and the enable/disable switch — without editing shell environment variables or restarting into a sourced shell.
- The **secret key is stored in the macOS Keychain** (service `dev.vire.app`), never in SQLite, logs, evidence rows, or exports. The settings form shows only a `set / not set` indicator and never renders the stored value back (SEC-009).
- The **public key is also stored in the Keychain** (same service) for a single clean credential surface.
- Non-secret settings (`base_url`, `source`, `environments`, `langfuse_enabled`) are stored as additive rows in the existing SQLite `settings` key/value table — no schema change to `projects` or `time_entries`.
- **Config precedence:** in-app settings override process environment variables. Env vars (`VIRE_LANGFUSE_*`) are retained as a clearly-marked developer fallback (the TASK-025 `.env.example` template remains valid for dev workflows).
- **Test connection:** a bounded action (20 s ceiling) that reports a coarse verdict (`reachable` / `auth_or_network_error`) without echoing any secret or raw response body. The button is disabled when the integration is turned off.
- **Disabled state:** with the integration toggle off, no import runs and no health probe fires; the source panel shows an explicit `disabled` state — never zero AI usage or cost.
- New Rust `keyring` crate dependency (`apple-native` feature, macOS Keychain Services).

**Mac application icon (Workstream B)**

- The packaged app now shows a **Vire icon** in the Dock and app switcher (generated into `src-tauri/icons/` including `icon.icns`; referenced by `bundle.icon` in `tauri.conf.json`).
- The current mark is a **temporary placeholder**. To replace with a branded asset: drop a PNG (≥ 1024 × 1024) at `src-tauri/icons/source/vire-icon.png`, run `npx tauri icon src-tauri/icons/source/vire-icon.png`, rebuild — no code change required.
- **Safe area (updated in TASK-027 E3):** the mark must occupy ~80% of the canvas (≈10% margin per side) — a full-bleed PNG renders oversized in the Dock. The placeholder was regenerated with this inset in TASK-027. The branded replacement PNG must keep the same safe area.

**Production packaged build, no dev server (Workstream C)**

- `npm run tauri:build` produces a self-contained `Vire.app` (and `.dmg` where the toolchain supports it) that **does not require a Vite dev server or `npm run tauri:dev` at runtime**.
- Artifacts: `src-tauri/target/release/bundle/macos/Vire.app` and (where supported) `src-tauri/target/release/bundle/dmg/Vire_0.1.0_<arch>.dmg`.

### Known limitations

- **Not code-signed or notarized.** On first launch, macOS Gatekeeper may block the app — right-click → Open (or *System Settings → Privacy & Security → Open Anyway*). Signing/notarization is out of scope for v0.1.
- **DMG generation** depends on toolchain support; `.app` is the primary artifact.
- **macOS only.** Cross-platform packaging (Windows/Linux) is out of scope.

### Compatibility and rollback

This release is safe to install alongside or after the prior v0.1 dev build on the same Mac:

- **Database:** uses the same `app_data_dir()/vire.sqlite` as all prior builds. The `init_db` schema init is idempotent (`CREATE TABLE IF NOT EXISTS` + `INSERT OR IGNORE`). The new Langfuse settings are additive key/value rows in the existing `settings` table — no destructive migration, no column changes to `projects` or `time_entries`.
- **Keychain:** entries are app-scoped (`dev.vire.app`) and persist across reinstall. They are not bundled in the artifact.
- **Rollback:** reverting to the immediately prior build opens the same `vire.sqlite` and ignores the unknown additive `settings` rows (key/value table, no schema dependency). No data loss. The prior build falls back to `VIRE_LANGFUSE_*` env vars for Langfuse configuration.

### Tests

**Rust** (`cargo test --manifest-path src-tauri/Cargo.toml`): **94 passed / 0 failed**
(includes 24 new TASK-026 tests covering config resolution precedence, SEC-009 secret non-leak, Test connection coarse verdict + bounded timeout, atomic Keychain pair rollback on failed replacement, and SEC-002 loopback boundary for settings-sourced values)

**Frontend** (`npm run test:frontend`): **39 passed / 2 failed**
(2 pre-existing failures in `tests/pi-observe.security.test.mjs` — unrelated to this task, file unchanged from `main`)

### Manual smoke steps before shipping

See [README.md — Packaged app and Langfuse settings](README.md#packaged-app-and-langfuse-settings-task-026--required-before-release) for the full checklist.

---

## v0.1 — Langfuse import, environment mapping and desktop polish (TASK-027)

**Branch:** `feat/task-027-langfuse-import-env-mapping-ux-polish`

### What changed

**Import report and schema_changed fix (Workstream A)**

- The **import source panel now shows real results** — counts of traces imported (unique / duplicates / skipped), plus per-environment health. Previously the result was discarded in the Rust core and the UI showed only a coarse health enum; `ImportReport`/`EnvImportLine` now thread counts all the way to the frontend.
- **`schema_changed` fixed for the current Langfuse payload shape.** `Observation` parsing now reads `usageDetails` and `costDetails` (current Langfuse token/cost locations) in addition to the legacy `usage`/`calculatedTotalCost` fields. A generation that arrives in the current shape no longer downgrades to `skipped_schema`; a genuinely unrecognised shape still does. The absence-≠-zero invariant is preserved.
- An empty or partial import is **explained, never blank** — the panel surfaces `missing` health, counts, and warnings for every environment.

**Automatic import (Workstream B)**

- Vire now **imports automatically**: once on startup (off the UI thread, after `init_db`) and periodically thereafter (default every 900 s; configurable via `VIRE_LANGFUSE_AUTO_IMPORT_INTERVAL_SECS`, floored at 30 s). The same `run_blocking_import` path used by the manual button handles both.
- A **shared mutex serialises auto and manual imports** — a periodic tick skips when an import is already in progress, and the manual button queues behind any running import. No concurrent DB writes.
- Auto-import **respects all existing switches**: `langfuse_enabled` and the SEC-002 loopback boundary. A disabled integration runs nothing and reads no credentials. No new capability or CSP change.

**Environment discovery and picker (Workstream C)**

- Vire **discovers environments automatically** by scanning `GET /api/public/traces` without an `environment` filter and collecting distinct non-empty `Trace.environment` values. Discovery runs as part of every import (additive, no extra network round-trip beyond the import page scans). Results persist in the new `langfuse_discovered_environments` table (`environment PK`, `first_seen`, `last_seen`).
- The **Settings environment field is now a checkbox picker** seeded from discovered environments; `vire` is always offered. An *Advanced* CSV field remains as a fallback for environments not yet surfaced by discovery. Saving stores the union of ticked boxes and any advanced entries.

**Environment → project mapping (Workstream D)**

- A new **Environment → project mapping panel** in Settings shows every discovered environment as mapped or unmapped. An unmapped environment can be mapped to an existing project, or you can use **Create project for `<env>`** to create a project and map it in one explicit action.
- Vire **never auto-creates a project or auto-maps an environment** (DEC-006). Every mapping is a deliberate user action.
- Imported evidence is associated with a project **at read time via a join** — evidence rows are never rewritten. Clearing a mapping changes only the link; no data loss.
- New additive table: `langfuse_env_project_map(environment PK, project_id FK→projects, created_at, updated_at)`.

**Desktop UX polish (Workstream E)**

- **Fake macOS traffic-light buttons removed.** The titlebar now shows only the native window controls. The layout is re-balanced with a three-column CSS grid (brand centred, version right-aligned) so the titlebar is coherent without the left dots.
- **Icon safe-area regenerated.** The placeholder mark is now inset to **~80% of the canvas** (`SAFE=0.8` in `src-tauri/icons/source/generate-vire-mark.mjs`) so macOS renders it at Dock parity with other apps. Icon set regenerated: `icon.icns`, `128x128.png`, `32x32.png`, `64x64.png`, and the full `src-tauri/icons/` set.
- **Safe-area requirement for the branded asset:** the final branded PNG (brand-owned, `artifacts/brand/`) **must keep the same ~80% safe area** — a full-bleed 1024×1024 PNG renders oversized in the Dock. The placeholder generator already applies this inset; the branded asset must too.

### Compatibility and rollback

This change is **fully additive** and safe to install on top of any prior Vire build:

- **DB:** two new tables (`langfuse_discovered_environments`, `langfuse_env_project_map`). No changes to `projects`, `time_entries`, or the existing `langfuse_*` tables. All new tables use `CREATE TABLE IF NOT EXISTS` (idempotent init, same posture as prior releases).
- **Rollback:** reverting to the TASK-026 build on the same Mac leaves the importer manual-only and the environment field hand-typed CSV; the two new tables are present in the SQLite file but ignored. No data loss, no destructive migration.
- **Settings / Keychain:** unchanged from TASK-026. The mapping and discovery tables carry no secrets.

### App self-update → TASK-028 (DEC-029)

In-app self-update (Tauri updater plugin, artifact signing, macOS code signing + notarisation, GitHub Releases pipeline) is **split to TASK-028** (DEC-029). It is fully out of scope here — no updater plugin, no minisign key, no new network egress host, no `capabilities/default.json` change. Recommended split: Phase 1 lightweight version check (opens the download page); Phase 2 signed/notarised auto-install.

### Tests

**Rust** (`cargo test --manifest-path src-tauri/Cargo.toml`): **120 unit + 3 adversarial = 123 passed / 0 failed**
(26 new TASK-027-specific tests covering import report counts + health, schema_changed fix, secret-free diagnostics, environment discovery, env→project mapping, auto-import serialisation, and disabled short-circuit)

**Frontend** (`npm run test:frontend`): **51 passed / 2 pre-existing failures (unrelated)**
(12 new TASK-027-specific tests in `tests/envMappingUi.test.mjs` and `tests/shellChrome.test.mjs`; the 2 failures are in `tests/pi-observe.security.test.mjs`, unchanged from `main` since before any TASK-027 commit)

### Manual smoke steps before shipping

See [README.md — Build and run the packaged app](README.md#build-and-run-the-packaged-app) install steps 5–7 for the environment picker, mapping, and import report flows. The full manual macOS smoke checklist (packaged app, icon Dock size, native controls, startup import, env mapping) is in [design.md §9](openspec/changes/task-027-langfuse-import-env-mapping-ux-polish/design.md).
