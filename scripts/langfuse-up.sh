#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR/observability/langfuse"
if [[ ! -f .env ]]; then
  printf 'Missing observability/langfuse/.env. Run ./scripts/setup-local-observability.sh first.\n' >&2
  exit 1
fi
docker compose --env-file .env up -d
printf 'Langfuse starting on %s (health may take a minute).\n' "$(grep -E '^LANGFUSE_HOST=' .env | cut -d= -f2- || echo http://localhost:3000)"
