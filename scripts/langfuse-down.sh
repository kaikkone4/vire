#!/usr/bin/env bash
set -euo pipefail
SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS:-false}" == "true" && -n "${PI_OBSERVE_ROOT_DIR:-}" ]]; then
  ROOT_DIR="$PI_OBSERVE_ROOT_DIR"
else
  ROOT_DIR="$SCRIPT_ROOT"
fi
cd "$ROOT_DIR/observability/langfuse"
docker compose --env-file .env down "$@"
printf 'Langfuse containers stopped. Named volumes are preserved unless you passed -v.\n'
