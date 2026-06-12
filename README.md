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

The test suite covers project create/update/archive persistence and active filtering, manual entry create/update/delete and validation, summary totals, CSV filtering/escaping/formula neutralization, text length validation, archived-project historical edits, inverted date-range rejection, SQLite persistence across reopen, and frontend HTML escaping for adversarial payloads.

## Manual verification

1. Launch with `npm run tauri:dev` and confirm the sidebar includes Today, Projects, Manual Entry, Reports, and Settings.
2. Confirm the Today/Settings capture status says `Manual Mode / Capture deferred` and there are no automatic capture controls.
3. Create a project, edit it, then archive it. Confirm archived projects disappear from active entry pickers but remain visible in all-project/report history.
4. Add, edit, and delete a manual entry; deletion requires confirmation.
5. Restart the app and confirm projects and entries persist.
6. In Reports, choose a date range/project filter and export CSV to a selected local destination. Confirm the file contains only matching manual entries.

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

## Privacy status

Vire stores data in a local SQLite database on this Mac. It has no accounts, cloud sync, hosted API, or automatic data upload. It does not capture active windows, idle state, screenshots, keystrokes, browser contents, full URLs, terminal commands, screen pixels, or file contents in the current v0.1 shell.

The AI trace import feature (in development) queries your **local Docker self-hosted Langfuse instance** only. No macOS activity, prompts, command bodies, or raw local evidence is sent to Langfuse Cloud. Local Langfuse traces may contain prompt/session/metadata from existing instrumentation; stricter redaction and retention limits are planned as a follow-up after the local import flow is validated.

Langfuse API keys and local stack credentials are stored in local configuration only and must not be printed, logged, committed, exported, or included in support output.
