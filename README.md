# Vire

Vire is a local-only macOS desktop app for project time tracking, AI usage evidence, and billing review. It imports AI traces (pi, Claude Code) from a **local Docker self-hosted Langfuse stack** as the primary AI time/usage/cost evidence source and requires human approval before any billable or profitability total is computed.

Current state: v0.1 Tauri v2 shell with manual time entries, projects, summaries, and CSV export. The local Docker Langfuse trace importer MVP is available (TASK-019); automatic macOS activity capture is in active development (TASK-005).

## Run locally

Prerequisites: macOS with Rust, Node.js/npm, and Tauri v2 system dependencies installed.

```sh
npm install
npm run tauri:dev
```

## Build

```sh
npm install
npm run tauri:build
```

## Tests

```sh
npm test
npm run test:frontend
```

The test suite covers project create/update/archive persistence and active filtering, manual entry create/update/delete and validation, summary totals, CSV filtering/escaping/formula neutralization/note-text fidelity, text length validation, archived-project historical edits, inverted date-range rejection, SQLite persistence across reopen, and frontend HTML escaping for adversarial payloads.

## Manual verification

1. Launch with `npm run tauri:dev` and confirm the sidebar includes Today, Projects, Manual Entry, Reports, and Settings.
2. Confirm the Today/Settings capture status says `Manual Mode / Capture deferred` and there are no automatic capture controls.
3. Create a project, edit it, then archive it. Confirm archived projects disappear from active entry pickers but remain visible in all-project/report history.
4. Add, edit, and delete a manual entry; deletion requires confirmation.
5. Restart the app and confirm projects and entries persist.
6. In Reports, choose a date range/project filter and export CSV:
   - **Success:** pick a writable `.csv` location and confirm — file is written, `Exported N entries.` alert appears, app stays responsive (no beachball).
   - **Cancel:** open the save dialog and dismiss without choosing — no file is written, app returns to a fully responsive state with no endless loading.
   - **Re-entry:** after a success or cancel, click Export CSV again — dialog opens and resolves normally (no stuck state from the prior run).

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
3. Configure Vire's Langfuse settings to point to `http://127.0.0.1:3000` with your local API key/secret. Do not commit credentials.
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
