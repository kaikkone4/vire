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
