# Local Langfuse Docker Stack — Setup and Operations

Vire uses a local Docker self-hosted Langfuse instance as its canonical AI trace source (DEC-020). This document covers setup, localhost binding, MinIO/S3 security, stack health, trace content boundaries, and environment/project mapping.

## Stack overview

The Langfuse self-hosted stack is a **stateful multi-component system**, not a single container. All components must be running and healthy for Vire to import AI trace data.

| Component | Image | Role | Default exposed port |
|---|---|---|---|
| `langfuse-web` | `langfuse/langfuse:3` | REST API (Vire import) + web UI | `127.0.0.1:3000` |
| `langfuse-worker` | `langfuse/langfuse-worker:3` | Ingestion, queued jobs, background work | none (internal) |
| PostgreSQL | `postgres` (official) | Application/auth/transactional state | none (internal) |
| ClickHouse | `clickhouse/clickhouse-server` | Trace/event analytics store | none (internal) |
| Redis / Valkey | `redis` or `valkey` | Worker queue and cache | none (internal) |
| MinIO | `minio/minio` | S3-compatible object storage | none (internal); API `minio:9000`, console `:9001` inside the container — not host-published |

Persistent Docker named volumes:

| Volume | Stores |
|---|---|
| `langfuse_postgres_data` | PostgreSQL database files |
| `langfuse_clickhouse_data` | ClickHouse data directory |
| `langfuse_clickhouse_logs` | ClickHouse log directory |
| `langfuse_minio_data` | MinIO object storage |
| `langfuse_redis_data` | Redis/Valkey persistence |

These volumes survive container restarts and upgrades. They are the primary backup scope.

## Prerequisites

- Docker Desktop for macOS, installed and running.
- Sufficient disk space for PostgreSQL, ClickHouse, and MinIO volumes (varies with trace volume; plan for several GB for active use).
- The existing loopback-bound stack in [`observability/langfuse/`](../observability/langfuse/) (the local dev/observability Compose stack), which already applies the `127.0.0.1` bindings below. The upstream [Langfuse Docker Compose file](https://langfuse.com/self-hosting/deployment/docker-compose) is the reference it was derived from.

> **Existing stack:** A loopback-bound `docker-compose.yml` with localhost bindings and `.env`-based secret references already exists at [`observability/langfuse/docker-compose.yml`](../observability/langfuse/docker-compose.yml) (the local dev/observability stack). It publishes only `127.0.0.1:${LANGFUSE_PORT:-3000}` for `langfuse-web` and no other host ports. Reuse it rather than authoring a new compose file.
>
> **Implementation follow-up (TASK-007):** Whether to add a separate *Vire-product-bundled* compose file (distinct from the dev/observability stack) is decided by the TASK-007 Langfuse importer spike. Any such file must keep the same localhost-only bindings and environment-variable references.

## Localhost binding

All service ports must be bound to `127.0.0.1` (loopback only) by default. This ensures the Langfuse stack is not reachable from the local network.

In a Docker Compose `ports` entry, use the explicit long-form binding:

```yaml
ports:
  - target: 3000
    published: 3000
    host_ip: "127.0.0.1"
    protocol: tcp
```

Or the short form with the IP prefix:

```yaml
ports:
  - "127.0.0.1:3000:3000"
```

In the existing `observability/langfuse/` stack, `langfuse-web` is the only host-published service (`127.0.0.1:${LANGFUSE_PORT:-3000}:3000`); apply this pattern to it. Internal services (PostgreSQL, ClickHouse, Redis/Valkey, the worker, **and MinIO**) are not host-published at all — do not add `ports:` entries for them. MinIO is reachable only on the internal Compose network (`minio:9000`), which is the correct, stricter posture.

Vire's default import endpoint is `http://127.0.0.1:3000`. Do not change this to a LAN address or Langfuse Cloud URL without an explicit configuration override in Vire settings.

## Credentials and secrets

Local Langfuse credentials include the PostgreSQL password, ClickHouse password, MinIO access key, MinIO secret key, Langfuse `NEXTAUTH_SECRET`, `SALT`, `ENCRYPTION_KEY`, and any Langfuse public/secret API keys used by Vire.

**All credentials must:**
- be stored in a local `.env` file outside version control (add `.env` to `.gitignore`), or in a secrets manager / macOS Keychain;
- never be committed to the repo, included in test fixtures, printed to logs, shown in diagnostic output, included in CSV exports, or exposed in support bundles;
- use randomly generated values for all secrets (not short or guessable values).

Reference credentials in `docker-compose.yml` via environment variable substitution (e.g. `${MINIO_ROOT_PASSWORD}`) so the compose file itself is safe to commit. The committed `observability/langfuse/docker-compose.yml` already follows this pattern — every secret is injected from `.env` (which is gitignored), with no hardcoded values.

## MinIO / S3-compatible object storage — security and operations

### Bucket and prefix layout

The Langfuse upstream defaults use a single bucket with logical prefixes:

| Setting | Default value |
|---|---|
| Bucket name | `langfuse` |
| Events prefix | `events/` |
| Media prefix | `media/` |
| Exports prefix | `exports/` |
| Docker volume | `langfuse_minio_data` |

Verify these values against your actual Langfuse environment variables (`LANGFUSE_S3_*` or equivalent) if you override the defaults.

### Access control

- The `langfuse` bucket **must not be set to public**. MinIO defaults to private access control; do not add bucket policies that grant anonymous read or list access.
- MinIO access key and secret key must never be logged, committed, exported, or printed in diagnostic output. Treat them with the same care as any production password.
- Do not use the same credential for local development and any production or staging Langfuse instance.

### MinIO console access

In the committed `observability/langfuse/` stack the MinIO console runs on `:9001` **inside the container only** (`--console-address ":9001"`) and is **not** published to the host. There is no `127.0.0.1:9001` mapping by default, which is the correct, stricter posture. If you ever need console access for local administration, add a temporary `127.0.0.1`-bound port mapping and remove it afterwards — never expose the console on a LAN interface.

### Data scope

MinIO stores objects that Langfuse web and worker write during trace ingestion, media handling, and batch exports. Loss of MinIO state makes those objects inaccessible even if PostgreSQL and ClickHouse are intact. See [backup-restore.md](backup-restore.md) for consistency requirements.

## Docker/Langfuse availability — Vire behaviour

Vire checks local Langfuse availability before every import run. The following health states are tracked:

| State | Meaning | Vire behaviour |
|---|---|---|
| `healthy` | Stack running; recent import; traces align with sessions | Normal import; AI totals displayed |
| `unavailable` | Docker not running, or one or more stack components unreachable | Vire reports unavailable; offers to open/start Docker where safe; refuses to show AI totals |
| `stale` | Stack reachable but latest trace/import older than expected threshold | Warning displayed; AI totals shown with stale indicator |
| `unknown` | Cannot determine whether local Langfuse state is current or complete | AI totals shown with unknown-source flag; user prompted to check stack |
| `missing` | Runtime session observed but no matching trace arrived | Warning; no cost inferred from runtime alone |
| `wrong_env` | Traces arrived in `default` or an unexpected environment | Warning; traces excluded unless environment is mapped |
| `auth_or_network_error` | 401/403/429 response or network failure (rate limiting folds here) | Vire reports error; AI totals withheld; check credentials and network reachability |
| `schema_changed` | Expected usage/cost fields absent or wrong type in API response | AI totals withheld; likely caused by a Langfuse version change; inspect Langfuse worker logs |
| `delayed` | A late-arriving trace predates the current import checkpoint | Trace reconciled and counted once; informational; no operator action needed |
| `duplicate` | Trace already imported in a previous run (deduplicated by `(environment, trace_id)`) | Informational; trace counted once across re-imports; no operator action needed |

**Vire never interprets Docker down, a missing stack component, or absent traces as zero AI usage or cost.** Absence of traces is an evidence gap that requires explicit review, not a zero-cost signal.

If Docker Desktop is not running, Vire may offer to open it automatically where the macOS API allows this safely. If the Langfuse stack containers are stopped, Vire may show a prompt with the relevant `docker compose up` command. Vire will not silently skip AI totals without surfacing the health state.

## Trace content boundary

Local Langfuse traces from pi and Claude Code may contain prompt text, session metadata, and AI response fragments depending on the instrumentation in place. Vire accepts this within the local Langfuse boundary for the MVP to make the local import flow work.

Stricter limits — redaction rules, prompt-text exclusion, metadata scrubbing, retention windows — are planned as a follow-up once the local trace import flow is validated end-to-end.

Raw macOS activity evidence (app names, window titles, idle/active samples) is stored separately in Vire's local SQLite database and is never sent to or mixed with Langfuse traces.

## Environment and project mapping

Langfuse environments are the primary mechanism for mapping traces to Vire projects. The workspace flow that uses environments is currently working well.

- Configure allowed Langfuse environments per Vire project in Vire settings (e.g. environment `vire` maps to the `vire` project).
- For pi and Claude Code trace emission, set both variables to ensure compatibility:
  ```
  LANGFUSE_TRACING_ENVIRONMENT=<project>
  OTEL_RESOURCE_ATTRIBUTES=langfuse.environment=<project>
  ```
- Traces that arrive in the `default` environment are flagged as `wrong_env` and excluded from AI totals until reviewed or remapped.
- pi-langfuse v1.4.3 requires a local environment propagation patch (including REST fallback) to emit traces with the correct environment. Verify traces are not landing in `default` after each workspace setup change.
- The Claude Code Langfuse observability plugin can silently emit no traces if the Python SDK or plugin is missing or misconfigured. Vire detects missing/stale Claude Code traces and surfaces them as `missing` health state rather than zero cost.

## Runtime reconciliation and import health

The runtime observer (TASK-022) reads a local coarse session log and reconciles observed pi/Claude Code runs against the imported traces. It uses the import health states from the table above to gate its conclusions:

- **Import `healthy`** and no matching trace exists → `observed_no_trace` (a confirmed trace-health gap worth reviewing).
- **Import `unavailable`, `unknown`, or `auth_or_network_error`** → `reconciliation_unknown` — Vire cannot determine whether the gap is real. This is never interpreted as zero AI usage or cost.
- **Runtime log absent, unreadable, or over the size cap** → `reconciliation_unknown` for all sessions. Absence is a state, not a zero-cost conclusion.

The observer is local-only: it reads the session log and the importer's normalized evidence rows from local SQLite, makes no network calls, and stores no token, cost, or duration values. Reconciliation is health/gap detection only; AI time and cost continue to come from the Langfuse importer.

For the runtime log source path, privacy boundary, configuration env vars, and full reconciliation state reference, see [README.md — Runtime reconciliation](../README.md#runtime-reconciliation).

## Vire import endpoint configuration

As of TASK-026 (DEC-026), the importer resolves its configuration **from the in-app Settings panel first**; process environment variables are retained only as a **clearly-marked developer fallback** for local dev / `.env`-sourced setups. The precedence order is:

1. **In-app settings** (Settings → AI evidence import): base URL, source, environments, and `langfuse_enabled` are stored in the local SQLite `settings` table. The public key and secret key are stored in the **macOS Keychain** (service `dev.vire.app`) — never in SQLite, never in plaintext.
2. **Process environment variables** (dev fallback — see table below): used only when the matching in-app setting is absent. Credentials from env are demoted to fallback; clear or replace them in-app once in-app settings are configured.
3. **Code defaults**: loopback `http://127.0.0.1:3000`, `source=local`, `environments=vire`.

The in-app Settings panel is the correct path for all normal usage. Env vars remain valid for local development workflows (e.g. `set -a; . ./.env; set +a` before `npm run tauri:dev`) but are not required once in-app settings are saved.

**Developer fallback env vars** (used only when the in-app setting is absent):

| Setting | Default | Env var | Notes |
|---|---|---|---|
| Base URL | `http://127.0.0.1:3000` | `VIRE_LANGFUSE_BASE_URL` | Local loopback; change only for an explicit Cloud override |
| Source posture | `local` | `VIRE_LANGFUSE_SOURCE` | `local` (default) or `cloud` (explicit override — produces off-host egress) |
| Allowed environments | `vire` | `VIRE_LANGFUSE_ENVIRONMENTS` | Comma-separated list; start with `vire`, add others as needed |
| API public key | — | `VIRE_LANGFUSE_PUBLIC_KEY` (fallback: `LANGFUSE_PUBLIC_KEY`) | Never committed, logged, or exported; overridden by Keychain when set in-app |
| API secret key | — | `VIRE_LANGFUSE_SECRET_KEY` (fallback: `LANGFUSE_SECRET_KEY`) | Never committed, logged, or exported; overridden by Keychain when set in-app |

Langfuse Cloud (`https://cloud.langfuse.com`) is supported as an explicit non-default override only. Set source to `cloud` and base URL to `https://cloud.langfuse.com` in the in-app settings (or via `VIRE_LANGFUSE_SOURCE=cloud` + `VIRE_LANGFUSE_BASE_URL=https://cloud.langfuse.com` as dev fallback); omitting either leaves the importer at the local Docker default.
