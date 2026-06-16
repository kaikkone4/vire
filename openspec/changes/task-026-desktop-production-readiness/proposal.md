# TASK-026 — Desktop production readiness (in-app Langfuse settings, Mac icon, packaged build)

## Why

Janne ran the local Mac MVP end-to-end (post-TASK-025) and hit three concrete gaps that stop Vire
from being a *real desktop app* rather than a dev-server toy:

1. **There is no way to configure Langfuse from inside the app.** The importer reads its base URL,
   source, environments, and **credentials from process environment variables only**
   (`src-tauri/src/langfuse/config.rs` — `from_env()` / `public_from_env()`, wired through
   `langfuse::run_blocking_import` → `ImporterConfig::from_env()` at `langfuse/mod.rs:39`). A local
   tester must `set -a; . ./.env; set +a` before launching (the TASK-025 onboarding fix). The
   Settings view (`src/main.ts:41` `renderSettings` / `sourcePanel`) **shows** source health and an
   "Import from Langfuse now" button but offers **no fields** to set the URL or keys, no way to turn
   the integration on/off, and no way to verify a connection. This is exactly the gap TASK-025 §Impact
   flagged as out-of-scope ("Any settings UI for entering keys … not introduced here").

2. **The app has no Mac application icon.** `tauri.conf.json` ships `"bundle": { … "icon": [] }` and
   `src-tauri/icons/` holds only a 159-byte placeholder `icon.png`. A running build shows the generic
   default icon in the Dock and app switcher — it does not look like a finished app.

3. **There is no documented, dev-server-free way to install/run the app.** `npm run tauri:build`
   exists (README:59) and `tauri.conf.json` already has `beforeBuildCommand: npm run build` +
   `frontendDist: ../dist`, so a self-contained bundle *should* be producible — but it is unverified,
   undocumented, icon-less, and there is no `.app`/`.dmg` install/run guidance and no release
   compatibility / rollback note for the downstream release artifact (SW-6).

These are **production-readiness** gaps for the private local prototype, not new product behavior. The
AI-evidence model, data boundaries, and local-only posture are unchanged. The change operationalizes
two assumptions the architecture already stated but never realized in the app:

- `03_architecture_plan.md:147` — "Langfuse API credentials and local stack secrets are provided by
  local configuration/**secret storage** and must never be printed, exported, committed, logged…";
- `03_architecture_plan.md:169` — "Setup should show the active base URL, environment(s), last import,
  local stack health, and credential/trace-health status **without displaying secrets**."

The TASK-019 design already anticipated this storage path: `task-019-local-langfuse-importer-mvp/design.md`
§3 lists credentials as coming "from **Keychain** or chmod-600 gitignored `.env`." TASK-026 realizes
the Keychain half for the in-app settings flow.

## What Changes

One OpenSpec change, **three clearly separated workstreams** (A → B → C). They are sequenced, not
independent PRs forced together: A is the security-bearing core and is implemented/reviewed first;
B and C are low-risk build/packaging slices that ride the same "production readiness" release. See
`tasks.md` for the routable per-workstream breakdown and `arch-review.md` for the split rationale.

### Workstream A — In-app Langfuse settings with secure secret storage  *(backend Rust + frontend)*

- Add an **editable Langfuse settings panel** to the existing Settings view (`src/main.ts`
  `renderSettings`/`sourcePanel`): base URL, source (local/cloud), environments (CSV), public key,
  secret key, and an **enable/disable** switch for the Langfuse integration. A **Test connection**
  action calls the existing read-only health probe path and reports reachable / auth-or-network-error
  **without echoing any secret**.
- **Persist non-secret settings** (`base_url`, `source`, `environments`, `langfuse_enabled`) in the
  existing SQLite `settings` table (`lib.rs:53`). **Persist the secret key in the macOS Keychain**
  via a secure-storage abstraction — **never** in SQLite, the `settings` table, logs, evidence, or
  exports (**SEC-009**). The public key is non-secret but, for a single clean credential surface, is
  stored alongside the secret in Keychain as well (it already round-trips through the redacting
  `Secret`/`Credentials` types in `config.rs`).
- **Re-point config loading** so the importer resolves config from the **settings store first, with
  process env as an explicit dev fallback** (clearly marked). `ImporterConfig::from_env()` /
  `public_from_env()` become `from_settings_or_env(...)` (exact shape is a developer decision; the
  contract is: stored settings win, env is the marked dev fallback, defaults unchanged — loopback
  `http://127.0.0.1:3000`, `source=local`, `environments=vire`). The existing Rust **URL allowlist /
  loopback invariant** (`config.rs::validate_target`, SEC-002) and **secret redaction**
  (`Secret`/`Credentials` Debug) are preserved unchanged and now also guard settings-sourced values.
- When the integration is **disabled**, no import runs and no health probe fires; the source panel
  shows a "disabled" state (never "zero usage/cost").
- **Architecture decision DEC-026** (proposed): Vire stores the Langfuse **secret key in the macOS
  Keychain** (via the `keyring` crate) and non-secret Langfuse settings in the local SQLite `settings`
  table; in-app settings take precedence over process-env, which is retained only as a marked dev
  fallback. See `design.md` §3 and the ADR below.

### Workstream B — Mac application icon  *(devops / assets)*

- Add a **source app icon** and generate the full Tauri icon set (`tauri icon <source.png>` →
  `src-tauri/icons/*` incl. `icon.icns` for macOS) and populate `tauri.conf.json` `bundle.icon` so the
  packaged app shows a real icon in the Dock and app switcher.
- If no branded asset exists, ship a **simple temporary Vire mark** and **document the replacement
  path** (drop the branded source PNG at the documented path, re-run `tauri icon`, rebuild) so brand
  can swap it later without code changes.

### Workstream C — Production packaged build, no dev server  *(devops / release)*

- **Verify and document** that `npm run tauri:build` produces a self-contained macOS **`.app`** (and
  **`.dmg`** where the toolchain supports it) that launches **without** the Vite dev server
  (`frontendDist: ../dist`, `beforeBuildCommand: npm run build`).
- Add a **README "Install / run the packaged app"** section: how to build, where the artifact lands
  (`src-tauri/target/release/bundle/{macos,dmg}/`), how to install/run, and that no dev server or
  `npm run tauri:dev` is required at runtime. Note the app still reads its Langfuse config from the
  in-app settings (Workstream A) and that AI import additionally needs the local Langfuse stack up.
- **Represent release compatibility & rollback for SW-6**: the packaged app reuses the same local
  SQLite DB (`app_data_dir/vire.sqlite`) and additive `settings`/Keychain entries — document the
  forward/backward compatibility (older build still reads the same DB; the new `settings` rows are
  additive; Keychain entries are app-scoped) and the rollback path (revert to the prior build; the
  DB/Keychain remain readable; no destructive migration).

## Impact

- **Affected code (Workstream A):** `src-tauri/Cargo.toml` (+`keyring`), a new secure-storage module
  (e.g. `src-tauri/src/settings/`), `src-tauri/src/langfuse/config.rs` + `mod.rs` (config source
  re-point), `src-tauri/src/lib.rs` (new IPC commands: get/set Langfuse settings, test connection,
  store/clear secret), `src/main.ts` + `src/style.css` (settings form + Test connection). The
  `capabilities/default.json` and CSP are reviewed but expected unchanged (no new webview network
  surface; Keychain and HTTP stay in the Rust core).
- **Affected config/assets (Workstreams B, C):** `tauri.conf.json` (`bundle.icon`), `src-tauri/icons/*`
  (generated), `README.md` (install/run + icon-replacement docs). No data-model or schema change
  beyond additive `settings` rows.
- **Affected specs:** **MODIFY/extend `app-configuration`** (in-app settings + secure secret storage +
  settings-over-env precedence + Test connection); **ADD `desktop-packaging`** (production packaged
  build without dev server, app icon, install/run docs, release compat/rollback). `csv-export`,
  `langfuse-importer`, and `runtime-reconciliation` capabilities are **not** modified.
- **Security (SEC-009):** strictly improves posture — the secret key moves from a sourced shell env /
  plaintext-prone `.env` into the **macOS Keychain**; the in-app form must never render stored secret
  values back, and Test connection must not echo credentials. SEC-002 loopback boundary and SEC-003
  credential-non-leak are preserved and now also cover settings-sourced values.
- **DEC-020 / NFR-001 preserved:** local Docker Langfuse on loopback stays the default; `cloud` stays
  an explicit, deliberately-selected non-default override (now selectable in-app, still loopback-gated
  by `validate_target`); no localhost binding is loosened; no raw activity egress.
- **Out of scope (clean boundaries):**
  - Any change to the **AI-evidence model**, capture, classification, review UI, or CSV export.
  - **OTEL trace *emission*** from Vire — Vire is a trace *consumer* (importer); the enable/disable
    switch governs the **import integration**, not agent-side OTEL emission (terminology clarified in
    `design.md` §2).
  - **Auto-DMG signing / notarization / Gatekeeper distribution** — out of scope for the private local
    prototype; `tauri build` unsigned/local is sufficient. Recorded as a future option, not built here.
  - **Cross-platform** (Windows/Linux) packaging — macOS-only per the Mac-first posture.
  - Migrating the **existing env-based dev flow** away — env stays as a marked dev fallback (TASK-025
    `.env.example` remains valid for developers).

## ADR — DEC-026 (proposed)

**Decision.** Vire's in-app Langfuse configuration is editable in the Settings view and persisted
locally: **non-secret settings** (`base_url`, `source`, `environments`, `langfuse_enabled`) in the
SQLite `settings` table; the **Langfuse secret key (and public key) in the macOS Keychain** via the
`keyring` crate. The importer resolves configuration from this **settings store first**, with process
environment variables retained as an **explicit, clearly-marked dev fallback**. Secrets are never
stored in SQLite, the settings table, logs, evidence rows, or exports, and are never rendered back into
the settings form. The DEC-020 defaults (loopback `http://127.0.0.1:3000`, `source=local`,
`environments=vire`) and the SEC-002 loopback URL allowlist are unchanged.

**Status.** Proposed (this change). Routed to BA-flow Architect for the canonical decision log
(`artifacts/ba/07_decision_log.md`) via `feedback_to_ba[]`; SW `code/` write-scope cannot edit it.

**Alternatives considered.** (1) **Encrypt secrets in SQLite** (app-managed key) — rejected: a
locally-stored encryption key next to the ciphertext is not meaningfully secure, and macOS already
provides Keychain. (2) **`tauri-plugin-stronghold`** (encrypted vault) — viable, but it requires a
user-supplied vault password and adds an interaction surface heavier than this single-user Mac-first
prototype needs; Keychain is the OS-native, password-free fit the task names first. Kept as a future
option if cross-platform parity is later required. (3) **Keep env-only config** — rejected: the whole
point of the task is in-app configuration without editing shell env.
