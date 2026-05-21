# Local Langfuse observability stack

This directory is for Janne's local Pi-Team/development observability. It is not part of the Vire Tauri runtime and does not containerize Vire.

The Compose file is pinned to `langfuse/langfuse:3.63.0` and was structured from Langfuse self-hosting documentation for v3-era deployments (Postgres, Redis, ClickHouse, object storage). It remains intentionally local-only and should be rechecked against upstream docs before changing the Langfuse image tag.

## Quick start

```sh
./scripts/setup-local-observability.sh
./scripts/langfuse-up.sh
./scripts/langfuse-smoke-test.sh
./scripts/langfuse-down.sh
```

The setup script checks Docker/Compose, Node/npm, and Rust/Cargo readiness. It asks before opening install docs or starting Docker/Colima and never installs system-level dependencies silently.

## Data and secrets

- Langfuse binds to `127.0.0.1:${LANGFUSE_PORT:-3000}`.
- `.env` is copied from `.env.example`, chmod `600`, and gitignored.
- Local Compose data lives in Docker named volumes: Postgres, Redis, ClickHouse, and MinIO.
- `pi-observe` local state defaults to `~/.local/state/pi-observe/events.jsonl` and `runs.json`.
- Defaults are metadata/summaries only. Full prompts, terminal streams, file contents, env dumps, and raw outputs are not captured.

After first login, create Langfuse project API keys in the UI and paste only into local `.env`. `./scripts/langfuse-smoke-test.sh` verifies local wrapper state and, when keys are present, checks that Langfuse's ingestion API accepts a smoke trace including response-body/per-event error checks. UI visibility is still called out separately because this local script does not depend on browser automation.


```sh
LANGFUSE_PUBLIC_KEY=...
LANGFUSE_SECRET_KEY=...
```

## Reset and backup

Stop without deleting data:

```sh
./scripts/langfuse-down.sh
```

Dangerous full reset (deletes local Langfuse volumes):

```sh
./scripts/langfuse-down.sh -v
```

The script asks for interactive confirmation before deleting volumes. For non-interactive automation, pass `--force` or set `LANGFUSE_DOWN_FORCE=true` only after taking any needed backups.

Backups are not automated in phase 1. For important data, export from Langfuse and/or back up Docker volumes before reset.

## Security notes

- Do not expose this stack on a public interface without a separate hardening review.
- Do not commit `.env` or Langfuse API keys.
- Do not paste client secrets into summaries.
- `pi-observe` redacts common token/private-key patterns but redaction is not a guarantee.
- EU/GDPR: this stack stores data locally on Janne's machine; no non-EU hosted service is required by this configuration.

## Troubleshooting

```sh
cd observability/langfuse
docker compose --env-file .env ps
docker compose --env-file .env logs langfuse-web langfuse-worker --tail=100
```

Avoid commands that print secrets. Do not share `.env` contents.

### Prisma `P1000` / Postgres authentication failed

If `langfuse-web` or `langfuse-worker` fails during Prisma migrations with `Authentication failed against database server at postgres`, the usual local cause is a password/volume mismatch:

- Docker named volume `vire-local-langfuse_langfuse_postgres` already contains an initialized Postgres data directory.
- `POSTGRES_PASSWORD` in `.env` was generated or changed later.
- The official Postgres image only uses `POSTGRES_PASSWORD` on first initialization; it does not update the password for an existing data directory.
- The Postgres healthcheck can still pass because `pg_isready` checks readiness, not password authentication.

`./scripts/langfuse-up.sh` now verifies the `.env` database credentials against an existing local Postgres volume before starting the full stack and stops with remediation steps if they fail.

If the local Langfuse data is disposable, reset the named volumes and recreate them with the current `.env` secrets:

```sh
./scripts/langfuse-down.sh -v
./scripts/langfuse-up.sh
```

This deletes local Langfuse Postgres, Redis, ClickHouse, and MinIO data after confirmation. Back up or export anything important first. In non-interactive automation, use `./scripts/langfuse-down.sh -v --force` only when deletion is intentional.

If the data is important, do **not** delete volumes. Restore the `POSTGRES_PASSWORD` value that was used when the volume was first initialized, or perform a manual Postgres password rotation from inside the database, then rerun `./scripts/langfuse-up.sh`.
