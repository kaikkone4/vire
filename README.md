# Vire

Vire is a local-only macOS desktop app for project time tracking, AI usage evidence, and billing review. It imports AI traces (pi, Claude Code) from a **local Docker self-hosted Langfuse stack** as the primary AI time/usage/cost evidence source and requires human approval before any billable or profitability total is computed.

Current version: v0.7.1. Includes manual time entries, projects, reports (with Last 7/14/30/90 day quick-range presets), and CSV export; a local Docker Langfuse AI trace importer with configurable range, backfill, and diagnostics; and an AI time-entry suggestion engine that proposes time blocks from imported Langfuse evidence for human review and explicit acceptance — nothing is auto-posted. Accepted suggestions carry AI cost (where available), visible in Reports summary cards and in the CSV export as `cost_total`/`cost_currency` columns. The Suggestions view provides actionable notices for unmapped environments, untimed entries, and a disabled Langfuse source (TASK-034).

## Run locally

Prerequisites: macOS with Rust, Node.js/npm, and Tauri v2 system dependencies installed.

```sh
npm install
npm run tauri:dev
```

## App runtime configuration (env)

Vire reads its runtime configuration from **process environment variables** using `std::env::var`. It does **not** auto-load a `.env` file — you must export the variables into the shell that launches the app.

### Quick start

```sh
cp .env.example .env
# Edit .env: fill in VIRE_LANGFUSE_PUBLIC_KEY and VIRE_LANGFUSE_SECRET_KEY
# (create keys in the Langfuse UI under Project settings → API keys)
set -a; . ./.env; set +a
npm run tauri:dev
```

### Variables

| Variable | Default | Notes |
|---|---|---|
| `VIRE_LANGFUSE_BASE_URL` | `http://127.0.0.1:3000` | Local Docker loopback (DEC-020). Must be loopback when source is `local`. |
| `VIRE_LANGFUSE_SOURCE` | `local` | `local` (Docker self-hosted) or `cloud` (explicit override only). |
| `VIRE_LANGFUSE_ENVIRONMENTS` | `vire` | CSV list of Langfuse environments to import from. |
| `VIRE_LANGFUSE_PUBLIC_KEY` | — | Required for import. Empty = no credentials. |
| `VIRE_LANGFUSE_SECRET_KEY` | — | Required for import. Never commit. |
| `VIRE_RUNTIME_LOG_PATH` | `~/.local/state/pi-observe/events.jsonl` | Optional. Explicit path to pi-observe session log. |
| `VIRE_RUNTIME_ENV_MAP` | (empty) | Optional. CSV map of project token → Langfuse environment. |
| `VIRE_RUNTIME_MATCH_SLOP_SECS` | `300` | Optional. Time-window slop for session/trace matching. |

The app also accepts the bare `LANGFUSE_PUBLIC_KEY` / `LANGFUSE_SECRET_KEY` names as a fallback when the `VIRE_*` names are not set.

### Two env files — different purposes

| File | Configures | Tracks |
|---|---|---|
| `.env` (root, gitignored) | **Vire desktop app** — Langfuse URL, source, credentials, runtime observer | No (gitignored) |
| `.env.example` (root) | Template for the above — safe defaults, empty secrets | Yes (tracked) |
| `observability/langfuse/.env` | **Langfuse Docker server stack** — Postgres, ClickHouse, Redis, MinIO bootstrap, NextAuth secrets | No (gitignored) |

Do not merge these files. The root `.env` holds settings the Vire app queries Langfuse with; the Docker-stack `.env` holds secrets the Langfuse server containers need to start.

## Build and run the packaged app

Vire ships as a self-contained macOS application bundle that runs **without** a Vite dev server or
`npm run tauri:dev` at runtime. The packaged app loads its frontend from the bundled production assets
(`frontendDist: ../dist`, built by `beforeBuildCommand: npm run build`).

```sh
npm install
npm run tauri:build
```

### Artifact location

`npm run tauri:build` writes the bundle under `src-tauri/target/release/bundle/`:

| Artifact | Path |
|---|---|
| App bundle | `src-tauri/target/release/bundle/macos/Vire.app` |
| Disk image (where the toolchain supports it) | `src-tauri/target/release/bundle/dmg/Vire_0.1.0_<arch>.dmg` |

### Install and run

1. Build with `npm run tauri:build`.
2. Open `src-tauri/target/release/bundle/macos/Vire.app` directly, or mount the `.dmg` and drag
   `Vire.app` into `/Applications`, then launch it.
3. **No dev server is required at runtime** — do not run `npm run dev` / `npm run tauri:dev` to use the
   packaged app. The bundled assets are served from inside the `.app`.
4. **Langfuse configuration comes from in-app settings** (Settings → AI evidence import): base URL,
   source (local/cloud), environments, public/secret key, and the enable switch. The secret key is
   stored in the macOS Keychain, never in plaintext. AI trace import additionally requires the local
   Langfuse Docker stack to be running (see below); a down stack is reported as unavailable, never as
   zero AI usage or cost.
5. **Environments are picked, not just typed** (TASK-027 C4): the Settings panel offers a checkbox
   picker seeded from the environments discovered during import; `vire` is the default. An *Advanced*
   field still accepts a comma-separated list for environments discovery has not yet surfaced. Saving
   stores the union of the ticked boxes and any advanced entries.
6. **Each environment maps to a Vire project** (TASK-027 D4, TASK-045): the *Environment → project mapping*
   panel shows every discovered, evidence-backed, or already-mapped environment. Environments with
   existing AI evidence rows appear immediately on Settings open without a re-import. An unmapped environment can be
   mapped to an existing project, or you can type a project name in the inline input and click
   **Create & map** to create a project and map it in one explicit action. Vire never auto-creates a
   project or auto-maps an environment — every
   mapping is a deliberate user action (DEC-006). Clearing a mapping changes only the link; imported
   evidence rows are never rewritten.
7. **Import results are explained, never blank** (TASK-027 A): after an import the source panel shows
   how many traces were imported, duplicated, or skipped, with per-environment health — an empty or
   partial result is explained rather than shown as zero.
8. **Configurable import range** (TASK-029 C): Settings → *Import range* offers `last_7d`, `last_30d`
   (default), `last_90d`, `all`, or `since:<RFC3339>`. A manual or automatic import uses this as the
   range floor; each environment tracks its own cursor and resumes incrementally.
9. **Backfill** (TASK-029 C): **Backfill now** re-scans floor→now regardless of the existing cursor.
   Large backfills run as ordered monthly chunks, each persisted atomically; an interruption loses at
   most the in-flight chunk and re-running continues where it left off (no duplicate rows — durable
   dedupe is idempotent).
10. **Schema diagnostics** (TASK-029 A): when traces are skipped, the import report surfaces grouped
    reason counts and bounded structural shape samples (key names and JSON type names only — no field
    values, no credentials). This replaces the previous repeated free-string warning.
11. **Saturation terminal state** (TASK-029 C): if a single millisecond timestamp holds more traces than
    one run can page through (≥ 50 000), the run surfaces a distinct *capped* terminal diagnostic — never
    an infinite re-run loop. Re-running cannot advance past this point; it is an explicit terminal limit
    of the source API's pagination, surfaced rather than silently truncated.
12. **AI time-entry suggestions** (TASK-032): the *Suggestions* view (sidebar → Suggestions) shows
    AI-evidence time blocks awaiting your review. **Nothing is auto-posted** — every time entry is created
    only when you click **Accept** or **Accept with edits**. Accepted suggestions appear in Today and
    Reports as a separate AI sub-line; they are never folded into your manual (billable) total. The
    exported CSV includes an `origin` column (`manual` or `ai_suggested`) so you can separate AI-suggested
    time in downstream tooling. If Langfuse evidence exists for an environment that has no Vire project
    mapping, a banner in Suggestions names those environments and links to Settings to resolve them — the
    evidence is surfaced, never silently dropped.

> The build is a local prototype: it is **not** code-signed or notarized. On first launch macOS
> Gatekeeper may require right-click → Open (or *System Settings → Privacy & Security → Open Anyway*).
> Signing/notarization is out of scope for v0.1.

### Application icon

The app ships a Vire icon (Dock and app switcher) generated into `src-tauri/icons/` and referenced by
`bundle.icon` in `src-tauri/tauri.conf.json`. The current mark is a **temporary placeholder** — brand
owns the final asset (`artifacts/brand/` is read-only to engineering).

To replace it with a branded asset — **no code change required**:

1. Drop a branded PNG (≥1024×1024, square) at `src-tauri/icons/source/vire-icon.png`.
2. Regenerate the icon set: `npx tauri icon src-tauri/icons/source/vire-icon.png`.
3. Rebuild: `npm run tauri:build`.

> **Safe area (TASK-027 E3):** the mark must occupy ~80% of a transparent 1024×1024 canvas (≈10%
> margin per side) so macOS renders it at Dock parity with other apps. The branded replacement asset
> **must keep this same ~80% safe area** — a full-bleed PNG renders oversized in the Dock. The
> placeholder generator already applies this inset.

The placeholder source PNG is produced by a dependency-free generator
(`src-tauri/icons/source/generate-vire-mark.mjs`, run with `node`) so the temporary mark is
reproducible until the branded asset lands.

### Release compatibility and rollback (see also [RELEASE.md](RELEASE.md))

This build is forward/backward compatible with prior Vire builds on the same Mac:

- **Data:** the packaged app uses the same local database, `app_data_dir()/vire.sqlite`, as the dev and
  prior builds. `init_db` is idempotent (`CREATE TABLE IF NOT EXISTS` + `INSERT OR IGNORE`), and the new
  Langfuse configuration persists as **additive rows in the existing key/value `settings` table** — no
  destructive migration. TASK-029 adds one new table (`langfuse_backfill_progress`, a single-row resume
  cursor) and one new settings row (`langfuse_import_range`). TASK-032 adds one new table
  (`time_entry_suggestions`) and one new additive column (`time_entries.origin TEXT NOT NULL DEFAULT
  'manual'`); existing rows are backfilled to `'manual'` — no destructive migration, no existing data
  altered. TASK-034 adds two additive nullable columns (`cost_total REAL`, `cost_currency TEXT`) on
  `time_entries` via `add_column_if_absent` — older builds ignore them silently; no data loss.
  TASK-044 adds a `langfuse_public_key` row to the `settings` table (no DDL change; the table
  pre-existed); a pre-v0.6.2 rollback ignores the row silently — no data loss.
- **Secrets (TASK-044):** the Langfuse **secret** key lives in an app-scoped macOS Keychain entry
  (service `dev.vire.app`, account `langfuse_secret_key`). The **public** key is stored in the
  local SQLite `settings` table alongside other non-secret configuration (base URL, environments)
  since v0.6.2 — it is not a secret and no longer requires a Keychain entry. Both persist across
  reinstall and are **not** bundled in the artifact. **Existing-install note:** after upgrading
  from a pre-v0.6.2 build the app shows "no credentials" on first launch — the public key is not
  auto-migrated from the legacy Keychain item. Open Settings → Langfuse and re-enter both keys
  once to restore the integration.
- **Rollback:** reverting to any prior build opens the same `vire.sqlite` and ignores unknown additive
  `settings` rows, the new `langfuse_backfill_progress` and `time_entry_suggestions` tables, the
  `time_entries.origin` column, and the `cost_total`/`cost_currency` columns → **no data loss, no
  destructive migration**. AI-accepted entries created before rollback persist as plain entries in older
  builds (cost not displayed). A prior build simply falls back to environment variables
  (`VIRE_LANGFUSE_*`) for Langfuse config, which remain a marked dev fallback. The default import window
  changes from 7 days to 30 days (TASK-029); re-importing a trace already stored is a durable-dedupe
  no-op.

## Dependency advisory gate

`src-tauri/deny.toml` defines a **target-scoped** Rust dependency advisory check run by
`.github/workflows/dependency-advisories.yml` on every PR, push to `main`, and
`workflow_dispatch`. Pinned scanner: **cargo-deny 0.19.9**. The check evaluates only the shipped
Apple targets (`aarch64-apple-darwin`, `x86_64-apple-darwin`); the runner is ubuntu-latest purely
for speed — the gate's correctness comes from `[graph].targets`, not the runner platform.

Run locally with the same pinned version CI uses:

```sh
cargo install cargo-deny --version 0.19.9 --locked
cd src-tauri && cargo deny check advisories
```

**Advisory posture — 17 RustSec advisories in `Cargo.lock` (as of v0.7.1):**

| Group | Count | Crates | Handling |
|---|---|---|---|
| Linux-only (deferred) | 12 | GTK3/glib/proc-macro-error — `cfg`-gated to Linux; absent from macOS graph | Target-scoped out via `[graph].targets`; **not** in `ignore` |
| Apple-present (accepted) | 5 | `unic-*` unmaintained (RUSTSEC-2025-0075/0080/0081/0098/0100) via `urlpattern → tauri-utils` | In `[advisories].ignore` with per-ID rationale |

**Tripwire:** adding a Linux build triple to `[graph].targets` re-surfaces the 12 Linux-only
advisories and fails the gate — by design. Evaluate and accept or fix each advisory before adding
a Linux target; do not add the gtk3-rs/glib IDs to `ignore`.

Full advisory inventory, reachability proof, and risk-acceptance rationale:
`src-tauri/deny.toml` (DEFERRED/ACCEPTED comment blocks) and [RELEASE.md](RELEASE.md) v0.7.1.

## Tests

```sh
npm test
npm run test:frontend
```

The test suite covers project create/update/archive persistence and active filtering, manual entry create/update/delete and validation, summary totals, CSV filtering/escaping/formula neutralization/note-text fidelity, text length validation, archived-project historical edits, inverted date-range rejection, SQLite persistence across reopen, and frontend HTML escaping for adversarial payloads.

## Manual verification

### Dev mode (quick sanity)

1. Launch with `npm run tauri:dev` and confirm the sidebar includes Today, Projects, Manual Entry, Reports, Suggestions, and Settings.
2. Confirm the Today/Settings capture status says `Manual Mode / Capture deferred` and there are no automatic capture controls.
3. Create a project, edit it, then archive it. Confirm archived projects disappear from active entry pickers but remain visible in all-project/report history.
4. Add, edit, and delete a manual entry; deletion requires confirmation.
5. Restart the app and confirm projects and entries persist.
6. In Reports, confirm the four quick-range preset buttons (**Last 7 days**, **Last 14 days**, **Last 30 days**, **Last 90 days**) appear above the date inputs (TASK-033). Click **Last 7 days** — confirm the start/end date inputs populate and the report re-renders. Select a project filter, then click **Last 30 days** — confirm the project filter is preserved. Manually edit either date field — confirm no preset button stays highlighted.
7. In Reports, choose a date range/project filter and export CSV:
   - **Success:** pick a writable `.csv` location and confirm — file is written, `Exported N entries.` alert appears, app stays responsive (no beachball).
   - **Cancel:** open the save dialog and dismiss without choosing — no file is written, app returns to a fully responsive state with no endless loading.
   - **Re-entry:** after a success or cancel, click Export CSV again — dialog opens and resolves normally (no stuck state from the prior run).
7. Open the Suggestions view (sidebar → Suggestions). Confirm it renders without error — an empty state
   ("No pending suggestions") is expected if no Langfuse evidence has been imported. Confirm that no
   time entries appear in Today until you explicitly click Accept on a suggestion.

### Packaged app and Langfuse settings (TASK-026 — required before release)

These steps require a macOS build; Keychain-backed paths cannot be verified in CI.

1. **Build the packaged app:** `npm run tauri:build` — confirm it completes without error.
2. **Launch the `.app` directly** from `src-tauri/target/release/bundle/macos/Vire.app` (no `npm run tauri:dev`). Confirm the Vire icon appears in the Dock and app switcher — not the generic default.
3. **Gatekeeper:** on first launch macOS may block the unsigned app — right-click → Open (or *System Settings → Privacy & Security → Open Anyway*).
4. **Settings → Langfuse integration panel:** confirm the panel is visible with base URL, source, environments, and the enable toggle.
5. **Save non-secret settings:** change the base URL / environments and click Save. Quit and relaunch — confirm the values persisted (SQLite round-trip).
6. **Credentials (no read-back):** enter a public key and secret key, click Save credentials. Confirm the form shows `set` flags — it must never display the stored values back.
7. **Keychain verify (TASK-044):** open macOS **Keychain Access.app** and confirm **one** entry exists under service `dev.vire.app` (account `langfuse_secret_key`). The public key is now stored in SQLite — no `langfuse_public_key` entry should appear in Keychain. A second Keychain dialog for the public key on fresh launch is a regression.
8. **Test connection:** with integration enabled and credentials set, click **Test connection**. Confirm a coarse verdict appears (`reachable` / `auth_or_network_error`) — no secret value or raw error body in the result. If the local Langfuse stack is not running, expect `auth_or_network_error` or `unavailable` — never an empty/frozen UI.
9. **Test connection disabled guard:** turn the integration toggle off and save. Confirm the Test connection button is disabled (with tooltip). Confirm "Import from Langfuse now" is also disabled.
10. **Clear credentials:** click Clear credentials → confirm both keys show `not set`. A subsequent import attempt reports `auth_or_network_error`, never zero AI usage or cost.
11. **Rollback smoke (if a prior build is available):** open a prior Vire build on the same Mac — confirm the DB loads, the unknown additive `settings` rows are silently ignored, and no crash or data loss occurs.

### Import range, backfill, and schema diagnostics (TASK-029 — required before release)

These steps require an active local Langfuse stack.

12. **Import range:** Settings → *Import range* — confirm options `last_7d`, `last_30d`, `last_90d`, `all`, `since:<RFC3339>` are present and `last_30d` is the default. Change to `last_90d`, click Save. Quit and relaunch — confirm `last_90d` persisted (SQLite round-trip).
13. **Backfill now:** with integration enabled, confirm **Backfill now** is visible. Click it; confirm the import report appears on completion and shows incremental progress (not a blank or frozen UI).
14. **Schema diagnostics:** if any traces were skipped, confirm the import report groups skip reasons (e.g. `N skipped: N observations-not-embedded`) rather than repeating the same free-string warning N times. Structural samples (if present) must show JSON key names and type names only — no field values, no credentials.
15. **Rollback smoke (TASK-029 additions):** open a prior Vire build on the same Mac — confirm the `langfuse_import_range` settings row and the `langfuse_backfill_progress` table are silently ignored (no crash, no import failure, no data loss).

### AI time-entry suggestions (TASK-032 — required before release)

Substituting your own imported Langfuse evidence where indicated. Steps 18–20 require at least one pending suggestion; steps 16–17 and 21 are verifiable without evidence.

16. **Open Suggestions view:** click *Suggestions* in the sidebar. Confirm the view renders — an empty
    state ("No pending suggestions") is expected with no imported evidence. Confirm no time entries
    appeared in Today as a result of opening this view.
17. **No auto-posting:** confirm that entries appear in Today only after you explicitly click **Accept**
    or **Accept with edits**. Generating or refreshing suggestions must not post any entry.
18. **Accept a suggestion** (requires evidence): if suggestions are listed, click **Accept** on one.
    Confirm it disappears from Suggestions, then appears in Today as an `AI-suggested Xh Ym` sub-line.
    Confirm the manual (billable) total in Today is unchanged — AI time is reported separately, never
    folded into the human total.
19. **Dismiss a suggestion** (requires evidence): click **Dismiss** on a pending suggestion and confirm
    the dismissal dialog appears; after confirming, the suggestion disappears and does not reappear after
    clicking **Refresh suggestions**.
20. **CSV origin column** (requires accepted entry): export CSV from Reports. Confirm the file includes
    an `origin` column; the accepted entry's row reads `ai_suggested`; manually-added entries read
    `manual`.
21. **Unmapped env guidance** (if applicable): if any Langfuse environment has no Vire project mapping,
    confirm a banner appears at the top of Suggestions naming those environments and their trace counts,
    with a link to Settings to map them. Confirm no evidence from unmapped environments is silently
    dropped — it appears in the banner, not in a suggestion.

### AI suggestions UAT polish (TASK-034 — required before release)

Steps 22–24 require at least one accepted suggestion with a known AI cost; step 24 also exercises
context-dependent Suggestions notices. Step 25 is optional (requires a same-minute block).

22. **AI cost in Reports** (requires accepted suggestion with cost): open Reports, choose a date range
    that includes the accepted entry's project. Confirm the project card shows an
    `AI-suggested Xh · $Y.YY` (or equivalent currency) sub-line. In the lead "Total tracked" card,
    confirm AI cost is shown separately. If entries span mixed currencies, confirm "—" appears rather
    than a summed total.
23. **CSV cost columns** (requires accepted entry with cost): export CSV from Reports. Confirm two new
    columns `cost_total` and `cost_currency` are present; the accepted AI-suggested row carries the
    cost values; manually-added rows have empty strings in those columns.
24. **Trackability notices in Suggestions** (context-dependent):
    - *Unmapped environment:* if a Langfuse environment has no project mapping, open Suggestions —
      confirm a "not trackable until mapped" notice with a "Map in Settings" link appears for that
      environment's suggestions (not a bare empty table).
    - *Untimed suggestion:* if a suggestion block carries no time span, confirm a
      "not auto-trackable — add time manually" badge appears on that row.
    - *Disabled source / empty state:* with no pending suggestions and the Langfuse integration
      disabled (Settings → disable), open Suggestions — confirm the empty state lists "source down"
      as a cause with an actionable link, not a blank table or bare "0".
25. **Same-minute normalization** (optional — requires a same-minute block): if a suggestion with
    identical `HH:MM` start and end is available, accept it — confirm it stores as a non-zero span
    (at least 1 min) with no error. For the `23:59` edge case, confirm the stored entry spans
    `23:58 → 23:59`, not midnight.

## Local Langfuse Docker stack

Vire's canonical AI trace source is a **local, self-hosted Langfuse instance running via Docker Compose**. This is not a single-container setup — it is a stateful multi-component stack that requires explicit startup, backup, and recovery procedures.

### Components

| Service | Role | Default local port |
|---|---|---|
| `langfuse-web` (`langfuse/langfuse:3`) | UI and REST API (Vire import endpoint) | `127.0.0.1:3000` |
| `langfuse-worker` (`langfuse/langfuse-worker:3`) | Ingestion and background processing | internal |
| PostgreSQL | Application/transactional state | internal |
| ClickHouse | Trace and event analytics store | internal |
| Redis / Valkey | Worker queue and cache | internal |
| MinIO (S3-compatible) | Event/media/export object storage | internal (not host-published) |

All service ports must be bound to `127.0.0.1` by default. Do not expose services on LAN interfaces unless you have explicitly changed the compose port bindings and understand the security implications.

### Setup

> **Existing local stack:** A loopback-bound, secret-safe Langfuse Compose stack already exists at [`observability/langfuse/`](observability/langfuse/) (the local Pi-Team/development observability stack). It binds Langfuse to `127.0.0.1:${LANGFUSE_PORT:-3000}`, publishes no other host ports, and injects every secret via required `.env` variables (no committed credentials). Reuse this vetted stack rather than hand-rolling a compose file and risking a misconfigured `127.0.0.1` binding.
>
> **Implementation follow-up (TASK-007):** Whether to add a separate *Vire-product-bundled* `docker-compose.yml` (distinct from the dev/observability stack above) is decided by the TASK-007 Langfuse importer spike. Any such file must keep the same localhost-only bindings and environment-variable references (no committed secrets). Until then, the `observability/langfuse/` stack is the reference; see [docs/langfuse-local-setup.md](docs/langfuse-local-setup.md) for binding requirements.

1. Install Docker Desktop for macOS and ensure it is running.
2. Bring up the existing loopback-bound stack in [`observability/langfuse/`](observability/langfuse/) (see its README and [docs/langfuse-local-setup.md](docs/langfuse-local-setup.md)); it already restricts bindings to `127.0.0.1`.
3. Copy `.env.example` → `.env`, fill in `VIRE_LANGFUSE_PUBLIC_KEY` and `VIRE_LANGFUSE_SECRET_KEY` with keys from the Langfuse UI (Project settings → API keys), then source into the shell: `set -a; . ./.env; set +a`. See [App runtime configuration (env)](#app-runtime-configuration-env). Do not commit `.env` or credentials.
4. In Vire, confirm the Langfuse health status shows `healthy` before relying on AI trace totals.

### Availability and UX

If Docker or the local Langfuse stack is not running, Vire reports AI trace health as `unavailable`, `stale`, or `unknown`. Vire will not interpret missing or unavailable traces as zero AI usage or cost. Depending on context, Vire may offer to open/start Docker where this is safe to do, or refuse to display or import AI totals until the stack is running.

### Object storage (MinIO/S3) — security cautions

The local Langfuse stack stores event, media, and batch-export objects in MinIO. Key requirements:

- **Default bucket:** `langfuse` — prefixes `events/`, `media/`, `exports/`
- **Default volume:** `langfuse_minio_data` (Docker named volume)
- The bucket **must not be set to public**. Keep the default private/non-public bucket policy.
- MinIO access and secret keys **must not be committed to the repo, logged, printed in diagnostics, exported in CSV, or shown in support output**.
- Store MinIO and Langfuse credentials only in local environment variables or a secrets manager; never in the codebase or evidence rows.
- Back up MinIO object storage **in sync with PostgreSQL and ClickHouse**. Restoring from mismatched backups causes divergence failures (see [docs/backup-restore.md](docs/backup-restore.md)).

For detailed MinIO setup, volume management, and backup/restore procedures, see [docs/langfuse-local-setup.md](docs/langfuse-local-setup.md) and [docs/backup-restore.md](docs/backup-restore.md).

## AI trace import

Vire imports AI traces from local Langfuse by Langfuse environment. Environments are the primary project mapping mechanism and are currently working well in the workspace flow.

- Default import endpoint: `http://127.0.0.1:3000`
- Langfuse Cloud is supported only as an explicit non-default override — it is not the default source
- Vire does not upload macOS activity, window titles, prompts, command bodies, or raw local evidence to Langfuse Cloud
- For emitting traces from pi and Claude Code into local Langfuse, set both `LANGFUSE_TRACING_ENVIRONMENT=<project>` and `OTEL_RESOURCE_ATTRIBUTES=langfuse.environment=<project>`

For full import configuration, health states, and operational guidance, see [docs/langfuse-local-setup.md](docs/langfuse-local-setup.md).

## Runtime reconciliation

Vire cross-checks locally observed pi/Claude Code agent runs against imported Langfuse traces. This is **reconciliation and health-gap detection only** — it is never a duplicate AI time or cost ledger, and absence of the runtime log or a down import is always `unknown`, never zero AI usage or cost.

### Source

The observer reads a local coarse session log produced by `pi-observe`. No processes are scanned, no command-lines are read, and no network calls are made.

| Config | Default | Override env var |
|---|---|---|
| Runtime log path | `~/.local/state/pi-observe/events.jsonl` | `VIRE_RUNTIME_LOG_PATH` (explicit file path) |
| State directory | `$HOME/.local/state/pi-observe/` | `PI_OBSERVE_STATE_DIR` (directory; `events.jsonl` appended) |

If the log is absent, empty, a symlink, or larger than 8 MB the observer has no runtime evidence. All otherwise-importable traces reconcile to `reconciliation_unknown`. Absence is a state — never a zero-cost conclusion.

### Privacy boundary

Each log record is filtered through a **strict ingest allowlist** before anything is stored. Only coarse metadata is kept: event type, project token, tool label, opaque run/session id (a hash), timestamps, and terminal status. The following are **always dropped and never persisted or logged**, even if a hostile log injects them:

- Prompt or response text
- Terminal command bodies or shell history
- Environment variables or secret-shaped strings
- Free-text summaries and path/repository identifiers beyond the safe project token

The allowlist is enforced at the type level — serde discards all non-listed keys — not as a runtime filter that could be bypassed. The observer makes no network calls and no new macOS permission is required.

### Reconciliation states

| State | Meaning |
|---|---|
| `matched` | Runtime session aligns with an imported trace (session id match, or same environment + overlapping time window) |
| `observed_no_trace` | Session observed **and** the Langfuse import for its window+environment was `healthy`, but no trace arrived — a confirmed trace-health gap |
| `reconciliation_unknown` | Import was `unavailable`/`unknown`/`auth_or_network_error` for the session's window, or no runtime log exists — absence is never zero |
| `unmatched_runtime` | Session cannot be mapped to any Langfuse environment (no project→environment mapping); needs manual review |
| `unmatched_trace` | An imported trace has no corresponding runtime session |

`observed_no_trace` is only asserted when the Langfuse import for the session's window and environment was `healthy`. A missing trace under a down or uncertain import resolves to `reconciliation_unknown`.

### Settings panel

The Settings AI evidence panel shows a live reconciliation summary beneath the Langfuse source status:

> Observed agent runs: **N** · without a matching trace: **M** · unknown: **K**. A down or absent import is reported as unknown, never zero.

If no runtime log is found: "Runtime reconciliation: **unknown** — no runtime session log found."

The full per-session review and approval UI is out of scope for this release (TASK-009).

## Privacy status

Vire stores data in a local SQLite database on this Mac. It has no accounts, cloud sync, hosted API, or automatic data upload. It does not capture active windows, idle state, screenshots, keystrokes, browser contents, full URLs, terminal commands, screen pixels, or file contents in the current v0.1 shell.

The AI trace import feature (in development) queries your **local Docker self-hosted Langfuse instance** only. No macOS activity, prompts, command bodies, or raw local evidence is sent to Langfuse Cloud. Local Langfuse traces may contain prompt/session/metadata from existing instrumentation; stricter redaction and retention limits are planned as a follow-up after the local import flow is validated.

The runtime reconciliation observer reads a local coarse session log (metadata only) through a strict ingest allowlist. No prompts, command bodies, shell history, environment variables, secrets, or free-text summaries are stored. It makes no network calls and is entirely local.

Langfuse API keys and local stack credentials are stored in local configuration only and must not be printed, logged, committed, exported, or included in support output.
