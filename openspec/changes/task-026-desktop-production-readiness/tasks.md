# Tasks — TASK-026 Desktop production readiness

**One OpenSpec change, three sequenced workstreams.** A is the security-bearing core (DEC-026 /
SEC-009) and is implemented and reviewed **first**; B and C are low-risk build/packaging slices on the
same release. Each workstream is routable to a distinct developer role — see the per-section role tag.
This is a delivery sequence, **not** a license to skip the security review gate on A.

## Workstream A — In-app Langfuse settings with secure secret storage  *(role: backend-developer Rust/Tauri + frontend)*

### A1. Secure-storage adapter + dependency

- [ ] Add `keyring` to `src-tauri/Cargo.toml`. Flag the new native-binding dependency in the PR.
- [ ] Add a `settings` module (e.g. `src-tauri/src/settings/`) exposing a small trait for secret
      get/set/clear so it is unit-testable with a fake/in-memory keychain (no real Keychain in CI).
      Service = bundle id `dev.vire.app`; separate accounts for public/secret key.
- [ ] Route all settings-sourced credentials through the **existing** `Secret` / `Credentials` types
      in `langfuse/config.rs` — do **not** add a second, un-redacted credential path.

### A2. Persist non-secret settings + re-point config loading

- [ ] Persist `base_url`, `source`, `environments`, `langfuse_enabled` in the existing SQLite
      `settings` table (`lib.rs:53`). Additive rows only — no schema change to `projects`/`time_entries`.
- [ ] Replace `ImporterConfig::from_env()` / `public_from_env()` callers (`langfuse/mod.rs:39,65`) with
      a settings-first resolver (`from_settings_or_env(...)`): **stored settings win; process env is the
      clearly-marked dev fallback; code defaults unchanged** (`http://127.0.0.1:3000`, `source=local`,
      `environments=vire`).
- [ ] Preserve invariants verbatim: `validate_target()` loopback-when-local (SEC-002),
      `build_url()` `/api/public/*` allowlist, `Secret`/`Credentials` redaction — now also covering
      settings-sourced values.
- [ ] `langfuse_enabled == false` short-circuits `import_langfuse_now` and the health probe **before**
      any network/Keychain read; the snapshot reports an explicit **disabled** state (never zero).

### A3. IPC commands

- [ ] Add to `invoke_handler!` (`lib.rs:252`), all returning secret-free `CmdResult`:
  - [ ] `get_langfuse_settings` → non-secret settings **+ `has_public_key` / `has_secret_key`
        booleans only** (never the secret values).
  - [ ] `set_langfuse_settings` → writes non-secret rows.
  - [ ] `set_langfuse_secret` → accepts secret in, writes Keychain, returns no secret.
  - [ ] `clear_langfuse_secret` → removes the Keychain entry.
  - [ ] `test_langfuse_connection` → reuses the health probe; returns a coarse reachable /
        `auth_or_network_error` verdict; **bounded** via the `run_bounded` pattern (`lib.rs:217`);
        no secret, no raw body, no stack-internal detail.

### A4. Settings UI (frontend)

- [ ] Extend `renderSettings` / `sourcePanel` (`src/main.ts:41`) with an editable Langfuse panel: base
      URL, source (local/cloud), environments (CSV), public key, secret key, enable/disable switch, and
      a **Test connection** button wired to `test_langfuse_connection`.
- [ ] Show secret state as "set / not set" (from the boolean flags) with replace/clear actions —
      **never** render a stored secret back into the field. Copy says "Langfuse integration / AI
      evidence import," **not** "tracing" (Vire is a consumer, see `design.md` §2).
- [ ] Minimal styling in `src/style.css`; keep within the existing panel pattern.

### A5. Tests (Rust, no real Keychain / no network in CI)

- [ ] Resolver precedence: settings > env > defaults; `validate_target` still rejects non-loopback when
      `source=local`; `cloud`+off-host allowed only on explicit source.
- [ ] Secret non-leak: `get_langfuse_settings` and all error strings are secret-free;
      `Debug(Credentials)` stays redacted; `langfuse_enabled=false` short-circuits before network/Keychain.
- [ ] `test_langfuse_connection` is bounded, coarse, and secret-free (extend the `run_bounded` tests).

## Workstream B — Mac application icon  *(role: devops / assets)*

- [ ] Provide a source icon PNG (≥1024×1024). If no branded asset exists, ship a **simple temporary
      Vire mark** at a documented source path (e.g. `src-tauri/icons/source/vire-icon.png`).
- [ ] Run `tauri icon <source>.png` to generate `src-tauri/icons/*` (incl. `icon.icns`).
- [ ] Populate `tauri.conf.json` `bundle.icon` (currently `[]`) with the generated paths.
- [ ] Document the **replacement path** in README: drop the branded PNG at the source path, re-run
      `tauri icon`, rebuild — no code change. Note brand owns the final asset (`artifacts/brand/` is
      read-only to SW).

## Workstream C — Production packaged build, no dev server  *(role: devops / release)*

- [ ] Verify `npm run tauri:build` produces `src-tauri/target/release/bundle/macos/Vire.app` (and a
      `.dmg` where supported) and that the launched `.app` runs **without** the Vite dev server.
- [ ] Add a README "Install / run the packaged app" section: build command, artifact location,
      install/run steps, "no dev server / no `tauri:dev` at runtime," and that Langfuse config comes
      from in-app settings (Workstream A) with the local stack required for AI import.
- [ ] **Release compat/rollback note for SW-6** (in README and surfaced for `RELEASE.md`): packaged app
      reuses `app_data_dir/vire.sqlite`; new `settings` rows are additive (idempotent `init_db`);
      Keychain entries are app-scoped (`dev.vire.app`); rollback to a prior build reads the same DB and
      ignores unknown `settings` rows → no data loss, no destructive migration.

## Cross-cutting / verify

- [ ] Confirm `capabilities/default.json` and the CSP are unchanged (no new webview network surface;
      Keychain + HTTP stay in the Rust core). If a change is needed, stop and flag it.
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` and `npm run test:frontend` pass.
- [ ] Re-read every changed file; confirm no secret value is logged, persisted to SQLite/evidence/
      exports, or rendered back into the settings form.

## Acceptance

- Janne can set Langfuse base URL, source, environments, public/secret key, and the enable switch
  **inside the app**, run **Test connection**, and import — **without** editing shell env.
- The secret key is stored in the macOS Keychain (not SQLite/plaintext) and is never displayed back or
  leaked to logs/evidence/exports (SEC-009).
- The built `.app`/`.dmg` launches with **no dev server** and shows the Vire icon in the Dock/app
  switcher; the icon replacement path is documented.
- Release compatibility and rollback are documented for SW-6; DEC-020 loopback default and SEC-002
  boundary are preserved.
