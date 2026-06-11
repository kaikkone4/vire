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
| MinIO | `minio/minio` | S3-compatible object storage | `127.0.0.1:9090` (API) |

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
- The upstream [Langfuse Docker Compose file](https://langfuse.com/self-hosting/deployment/docker-compose) as a starting point.

> **Implementation follow-up (TASK-007):** A project-local `docker-compose.yml` with Vire-specific localhost bindings and environment variable references does not exist in this repo yet. It will be added after the TASK-007 Langfuse importer spike confirms the required stack configuration.

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

Apply this pattern to every port that the `langfuse-web` and MinIO services expose. Internal service ports (PostgreSQL, ClickHouse, Redis/Valkey, the worker) should not be published at all in a local development compose — remove or comment out any `ports:` entries for those services.

Vire's default import endpoint is `http://127.0.0.1:3000`. Do not change this to a LAN address or Langfuse Cloud URL without an explicit configuration override in Vire settings.

## Credentials and secrets

Local Langfuse credentials include the PostgreSQL password, ClickHouse password, MinIO access key, MinIO secret key, Langfuse `NEXTAUTH_SECRET`, `SALT`, `ENCRYPTION_KEY`, and any Langfuse public/secret API keys used by Vire.

**All credentials must:**
- be stored in a local `.env` file outside version control (add `.env` to `.gitignore`), or in a secrets manager / macOS Keychain;
- never be committed to the repo, included in test fixtures, printed to logs, shown in diagnostic output, included in CSV exports, or exposed in support bundles;
- use randomly generated values for all secrets (not short or guessable values).

Reference credentials in `docker-compose.yml` via environment variable substitution (e.g. `${MINIO_ACCESS_KEY}`) so the compose file itself is safe to commit once the implementation follow-up above is resolved.

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

The MinIO web console is available at `http://127.0.0.1:<console-port>` (typically `9001` in standard MinIO setups; verify in your compose). Use it only for local administration. Do not expose the console port to the network.

### Data scope

MinIO stores objects that Langfuse web and worker write during trace ingestion, media handling, and batch exports. Loss of MinIO state makes those objects inaccessible even if PostgreSQL and ClickHouse are intact. See [backup-restore.md](backup-restore.md) for consistency requirements.

## Docker/Langfuse availability — Vire behaviour

Vire checks local Langfuse availability before every import run. The following health states are tracked:

| State | Meaning | Vire behaviour |
|---|---|---|
| `healthy` | Stack running; recent import; traces align with sessions | Normal import; AI totals displayed |
| `unavailable` | Docker not running, or one or more stack components unreachable | Vire reports unavailable; offers to open/start Docker where safe; refuses to show AI totals |
| `stale` | Stack reachable but latest trace/import older than expected | Warning displayed; AI totals shown with stale indicator |
| `unknown` | Cannot determine whether local Langfuse state is current or complete | AI totals shown with unknown-source flag; user prompted to check stack |
| `missing` | Runtime session observed but no matching trace arrived | Warning; no cost inferred from runtime alone |
| `wrong_env` | Traces arrived in `default` or an unexpected environment | Warning; traces excluded unless environment is mapped |

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

## Vire import endpoint configuration

| Setting | Default | Notes |
|---|---|---|
| Base URL | `http://127.0.0.1:3000` | Local stack; change only for explicit Cloud override |
| API credentials | Stored in local config / Keychain | Never committed, logged, or exported |
| Environment filter | Per-project in Vire settings | Start with `vire`; add others as needed |

Langfuse Cloud (`https://cloud.langfuse.com`) is supported as an explicit non-default override only. Do not set the base URL to a Cloud endpoint without understanding that this changes the data source for all AI trace imports.
