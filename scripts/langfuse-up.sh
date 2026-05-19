#!/usr/bin/env bash
set -euo pipefail
SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS:-false}" == "true" && -n "${PI_OBSERVE_ROOT_DIR:-}" ]]; then
  ROOT_DIR="$PI_OBSERVE_ROOT_DIR"
else
  ROOT_DIR="$SCRIPT_ROOT"
fi
cd "$ROOT_DIR/observability/langfuse"
if [[ ! -f .env ]]; then
  printf 'Missing observability/langfuse/.env. Run ./scripts/setup-local-observability.sh first.\n' >&2
  exit 1
fi
sanitize_url_for_display() {
  node -e '
    try { const u = new URL(process.argv[1] || "http://localhost:3000"); console.log(`${u.protocol}//${u.hostname}${u.port ? `:${u.port}` : ""}`); }
    catch { console.log("invalid-host"); }
  ' "$1" 2>/dev/null || printf '%s\n' 'invalid-host'
}
raw_host="$(awk -F= '$1 == "LANGFUSE_HOST" { sub(/^[^=]*=/, ""); print; exit }' .env 2>/dev/null || true)"
raw_host="${raw_host:-http://localhost:3000}"
docker compose --env-file .env up -d
printf 'Langfuse starting on %s (health may take a minute).\n' "$(sanitize_url_for_display "$raw_host")"
