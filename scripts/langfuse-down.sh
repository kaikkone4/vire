#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR/observability/langfuse"
docker compose --env-file .env down "$@"
printf 'Langfuse containers stopped. Named volumes are preserved unless you passed -v.\n'
