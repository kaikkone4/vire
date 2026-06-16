# Design — TASK-026 Desktop production readiness

- **Author:** SW Architect (SW-1) scaffold for developer roles
- **Change:** `task-026-desktop-production-readiness`
- **Tier:** L2 (secure secret storage + documentation + release compat/rollback thresholds apply)
- **Decisions honored:** DEC-020 / DEC-022 (local Docker Langfuse default; Cloud explicit override),
  DEC-025 (`VIRE_*` env config — retained as a marked dev fallback), SEC-002 (loopback network
  boundary), SEC-003 (credential non-leak). **New:** DEC-026 (Keychain secret storage), SEC-009
  (secure secret storage control).
- **Source of truth:** `artifacts/ba/03_architecture_plan.md` §4.4 (settings show base URL / env /
  health **without secrets**) and §7 (secrets are local config, never displayed/logged/exported);
  `:147` (credentials from local **secret storage**); TASK-019 `design.md` §3 (credentials "from
  Keychain or chmod-600 `.env`"); the live code in `src-tauri/src/langfuse/config.rs`, `langfuse/mod.rs`,
  `lib.rs`, and `src/main.ts`.

## 1. Component boundaries

The change touches three areas; none crosses a **data-model** component boundary
(`03_architecture_plan.md` §3 — capture / runtime observer / importer / store / classifier / review /
exporter). The importer's *evidence* logic is untouched; only its **config source** changes.

| Workstream | Component(s) touched | Boundary crossed? |
| --- | --- | --- |
| A — in-app settings + secret storage | Settings component + Langfuse importer **config** (not evidence) + a new secure-storage adapter | No data boundary; extends `app-configuration` |
| B — app icon | Build/bundle config + assets | None (build-time) |
| C — production packaging | Build/bundle + docs + release artifact | None (build-time) |

```
 webview (renderer)                 Rust core (server-side)                    OS / external
 ─────────────────                  ───────────────────────                   ─────────────
 Settings form  ──IPC──▶  get_langfuse_settings (no secret echoed)
  base url / src /        set_langfuse_settings ──┐
  envs / enabled /        set_langfuse_secret ────┼─▶ settings module
  [public key] /          clear_langfuse_secret   │     ├─ non-secret ─▶ SQLite `settings` table
  [secret key] /          test_langfuse_connection │     └─ secret ─────▶ macOS Keychain (keyring)
  Test connection         get_langfuse_source_health (existing, read-only)
                          import_langfuse_now ─────────▶ ImporterConfig::from_settings_or_env
                                                          (settings first, env = dev fallback)
                                                          → reqwest → 127.0.0.1:3000 (loopback)
```

**Why secret storage lives in the Rust core, not the webview:** the same reason the importer does
(TASK-019 `design.md` §1) — the CSP is `connect-src ipc: http://ipc.localhost`, so the renderer cannot
touch the network or OS Keychain. Keychain access and credential handling stay server-side; the
renderer only sends/receives non-secret settings and a boolean test-connection verdict.

## 2. Terminology — "tracing" means the Langfuse **import** integration

Janne's scope says "enable/disable tracing." In Vire's role this is the **Langfuse trace-import
integration**, not OTEL trace *emission* — **Vire is a consumer of Langfuse traces, never a producer**
(`03_architecture_plan.md` §4.2/§4.3; the importer is read-only, `langfuse/mod.rs`). The setting is
therefore modeled as `langfuse_enabled` (master switch for import + health probing). When `false`: no
import runs, no health probe fires, and the source panel shows an explicit **disabled** state — never a
zero AI usage/cost (absence-≠-zero invariant, `langfuse-importer` spec). The settings UI copy should
say "Langfuse integration" / "AI evidence import," not "tracing," to avoid implying Vire emits traces.

## 3. Secret storage model (DEC-026 / SEC-009)

| Setting | Secret? | Store | Notes |
| --- | --- | --- | --- |
| `base_url` | no | SQLite `settings` | default `http://127.0.0.1:3000`; loopback-gated when `source=local` |
| `source` | no | SQLite `settings` | `local` (default) \| `cloud` (explicit override; only off-host egress) |
| `environments` (CSV) | no | SQLite `settings` | default `vire`; environment-first mapping |
| `langfuse_enabled` | no | SQLite `settings` | master switch for import + health probe |
| `public_key` | no (but credential-adjacent) | macOS **Keychain** | kept with the secret for one clean credential surface; already redacted by `Credentials` Debug |
| `secret_key` | **yes** | macOS **Keychain** | **never** SQLite/settings/logs/evidence/exports; **never** rendered back to the form |

- **Keychain access** via the `keyring` crate (macOS Keychain Services / Security framework). Service
  name e.g. `dev.vire.app` (the bundle identifier), distinct account entries for public/secret key.
  Adding the dependency is part of DEC-026; flag it explicitly in the PR (it is a new native-binding
  dependency).
- **Reuse the existing redaction types.** `config.rs` already defines `Secret` (redacts in
  `Debug`/`Display`, exposed only at the `Authorization` header) and `Credentials` (redacts even the
  public key). Settings-sourced credentials MUST flow through these same types — do not introduce a
  second, un-redacted credential path.
- **No read-back.** `get_langfuse_settings` returns non-secret fields plus **boolean presence flags**
  (`has_public_key`, `has_secret_key`) — never the secret values. The form shows "•••• set / not set,"
  with actions to replace or clear. This realizes `03_architecture_plan.md:169` ("without displaying
  secrets").
- **Clearing** a secret removes the Keychain entry; the importer then resolves to the env dev fallback
  if present, else surfaces `auth_or_network_error` (never zero), per the existing
  `Credentials: Option<...>` contract (`config.rs:121`).

## 4. Config resolution precedence (settings-first, env as marked dev fallback)

Today: `ImporterConfig::from_env()` (full, incl. creds) and `public_from_env()` (no creds) are the only
loaders (`langfuse/mod.rs:39,65`). Re-point to a single resolver, e.g.
`ImporterConfig::from_settings_or_env(conn, keychain)`:

1. **Non-secret:** read `base_url` / `source` / `environments` / `langfuse_enabled` from the `settings`
   table; if a key is unset, fall back to the env var; if that is unset, the existing **code default**
   (`DEFAULT_BASE_URL`, `Source::Local`, `DEFAULT_ENVIRONMENT`). Defaults are unchanged.
2. **Credentials:** read public/secret from Keychain; if absent, fall back to
   `VIRE_LANGFUSE_PUBLIC_KEY` / `VIRE_LANGFUSE_SECRET_KEY` (then bare `LANGFUSE_*`) — the existing env
   path, now demoted to a **dev fallback** that should be clearly labeled in code comments and docs.
3. **Invariants preserved:** `validate_target()` (loopback-when-local, SEC-002) runs unchanged on the
   resolved `base_url`; `build_url()`'s `/api/public/*` allowlist is unchanged; secrets keep flowing
   through `Secret`/`Credentials`.
4. **`langfuse_enabled == false`** short-circuits `import_langfuse_now` and the health probe before any
   network/Keychain read; the snapshot reports a disabled state.

Keep the resolver a pure function of `(settings rows, keychain, env)` so it stays unit-testable with a
fake settings source and a fake/in-memory keychain (no real Keychain in CI).

## 5. New IPC surface (Workstream A)

Add to `invoke_handler!` (`lib.rs:252`), all returning the existing `CmdResult<T>` secret-free error
strings:

| Command | Returns | Secret exposure |
| --- | --- | --- |
| `get_langfuse_settings` | non-secret settings + `has_public_key` / `has_secret_key` booleans | **none** |
| `set_langfuse_settings` | updated non-secret settings | none (writes `settings` table) |
| `set_langfuse_secret` | `()` / ok | accepts secret **in**, never returns it; writes Keychain |
| `clear_langfuse_secret` | `()` / ok | removes Keychain entry |
| `test_langfuse_connection` | reachable / `auth_or_network_error` verdict (no secret, no body) | **none** |

`test_langfuse_connection` reuses the importer's availability/health probe against the *currently
entered or stored* config and reports only a coarse verdict — never a secret, raw response body, or
stack-internal detail. It must be **bounded** like `import_langfuse_now` (reuse the `run_bounded`
pattern, `lib.rs:217`) so a hung probe cannot freeze the UI.

## 6. App icon (Workstream B)

- Generate the icon set with `tauri icon <source>.png` (Tauri CLI is already a devDependency,
  `@tauri-apps/cli ^2.2.0`). It writes `src-tauri/icons/{icon.icns, icon.png, *.png, Square*.png}` and
  the macOS `.icns` is what the Dock/app switcher use.
- Populate `tauri.conf.json` `bundle.icon` with the generated paths (currently `[]`).
- **Replacement path (documented):** drop the branded source PNG at a documented location (e.g.
  `src-tauri/icons/source/vire-icon.png`, ≥1024×1024), re-run `tauri icon`, rebuild. No code change for
  a brand swap. The temporary mark is explicitly a placeholder; brand owns the final asset
  (`artifacts/brand/` is read-only to SW — do not write there).

## 7. Production packaging & release compat/rollback (Workstream C)

- `npm run tauri:build` → `beforeBuildCommand: npm run build` (tsc + vite → `dist/`) → Tauri bundles
  `src-tauri/target/release/bundle/macos/Vire.app` and (where supported) `…/dmg/Vire_0.1.0_*.dmg`. The
  app loads `frontendDist: ../dist` — **no dev server at runtime**. Verify by launching the built
  `.app` directly (manual macOS smoke; document it as the acceptance check).
- **Release compatibility (for SW-6 `RELEASE.md`):**
  - **Data:** the packaged app uses the same `app_data_dir()/vire.sqlite` (`lib.rs:249`) as the dev
    build; `init_db` is idempotent (`CREATE TABLE IF NOT EXISTS` + `INSERT OR IGNORE`), and the new
    Langfuse settings are **additive `settings` rows** — no destructive migration, no column changes
    to `projects`/`time_entries`.
  - **Secrets:** Keychain entries are app-scoped (`dev.vire.app`); they persist across reinstall and
    are not bundled in the artifact.
  - **Rollback:** revert to the prior build → it reads the same DB and ignores unknown `settings` rows
    (key/value table, no schema dependency) → no data loss. A prior build simply falls back to env for
    Langfuse config. Document this forward/back compatibility explicitly.
- **Versioning:** `tauri.conf.json` `version` / `Cargo.toml` `version` stay `0.1.0` unless Janne bumps;
  SW-6 owns the release-tag/version decision. Signing/notarization remain out of scope (local prototype).

## 8. Test direction (handoff to SW-3 / SW-4)

- **Config resolver (unit, Rust):** settings-row present → wins over env; env present + no settings →
  env used; neither → code defaults; `validate_target` still rejects non-loopback when `source=local`;
  `cloud` + off-host allowed only on explicit source. Fake settings + fake keychain; **no real
  Keychain, no network** in CI.
- **Secret non-leak (unit, Rust):** `get_langfuse_settings` output and every error string contain no
  secret material (extend the existing redaction assertions, e.g. `lib.rs` `run_bounded` secret-free
  test); `Debug` of settings-sourced `Credentials` stays redacted; `langfuse_enabled=false`
  short-circuits before any network/Keychain read.
- **Test connection (unit, Rust):** bounded like `import_langfuse_now`; returns a coarse verdict;
  never includes a secret or raw body; a hung probe times out with a secret-free message.
- **Packaging/icon (manual macOS smoke, documented):** built `.app` launches with no dev server and
  shows the Vire icon in the Dock/app switcher; in-app settings persist across relaunch; secret set →
  import works; secret cleared → falls back/`auth_or_network_error`, never zero. Keychain-backed paths
  are validated manually (CI cannot assert real Keychain).

## 9. Out-of-scope boundaries (do not build here)

- OTEL trace **emission** from Vire (it is a consumer); cross-platform packaging; DMG
  signing/notarization/Gatekeeper distribution; any AI-evidence/model/capture/classification/review/CSV
  change; encrypting secrets *inside* SQLite (rejected — see DEC-026 alternatives). The `app-configuration`
  env template (TASK-025) is **not** removed — env remains the marked dev fallback.
