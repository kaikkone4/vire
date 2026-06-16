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
