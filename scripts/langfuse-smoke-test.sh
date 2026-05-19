#!/usr/bin/env bash
set -euo pipefail
SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS:-false}" == "true" && -n "${PI_OBSERVE_ROOT_DIR:-}" ]]; then
  ROOT_DIR="$PI_OBSERVE_ROOT_DIR"
else
  ROOT_DIR="$SCRIPT_ROOT"
fi
ENV_FILE="$ROOT_DIR/observability/langfuse/.env"
read_env_value() {
  local key="$1"
  [[ -f "$ENV_FILE" ]] || return 1
  awk -F= -v k="$key" '
    $0 ~ "^[[:space:]]*#" || $0 !~ "=" { next }
    $1 == k { sub(/^[^=]*=/, ""); gsub(/^[[:space:]]+|[[:space:]]+$/, ""); gsub(/^\"|\"$/, ""); gsub(/^'"'"'|'"'"'$/, ""); print; exit }
  ' "$ENV_FILE" 2>/dev/null
}
sanitize_url_for_display() {
  node -e '
    try { const u = new URL(process.argv[1] || "http://localhost:3000"); console.log(`${u.protocol}//${u.hostname}${u.port ? `:${u.port}` : ""}`); }
    catch { console.log("invalid-host"); }
  ' "$1" 2>/dev/null || printf '%s\n' 'invalid-host'
}
is_loopback_host() {
  node -e '
    try { const h = new URL(process.argv[1]).hostname.toLowerCase().replace(/^\[|\]$/g, ""); process.exit(["localhost", "127.0.0.1", "::1", "0:0:0:0:0:0:0:1"].includes(h) ? 0 : 1); }
    catch { process.exit(1); }
  ' "$1" 2>/dev/null
}
HOST="$(read_env_value LANGFUSE_HOST || true)"; HOST="${HOST:-http://localhost:3000}"
DISPLAY_HOST="$(sanitize_url_for_display "$HOST")"
printf 'Checking Langfuse health at %s ...\n' "$DISPLAY_HOST"
if command -v curl >/dev/null 2>&1; then
  if is_loopback_host "$HOST" || [[ "${PI_OBSERVE_ALLOW_REMOTE_LANGFUSE:-$(read_env_value PI_OBSERVE_ALLOW_REMOTE_LANGFUSE || true)}" == "true" ]]; then
    if curl -fsS "$HOST/api/public/health" >/dev/null; then
      printf 'Health endpoint responded.\n'
    else
      printf 'Health endpoint not ready yet or unavailable; continuing with fail-open wrapper test.\n'
    fi
  else
    printf 'Skipping health curl for non-loopback Langfuse host %s; set PI_OBSERVE_ALLOW_REMOTE_LANGFUSE=true to opt in.\n' "$DISPLAY_HOST"
  fi
fi
STATE_DIR="$(mktemp -d)"
PI_OBSERVE_STATE_DIR="$STATE_DIR" PI_OBSERVE_PROJECT=vire "$ROOT_DIR/observability/pi-observe/bin/pi-observe.mjs" run --tool smoke-test --role verification --summary "local smoke test" -- node -e "console.log('pi-observe smoke ok')"
PI_OBSERVE_STATE_DIR="$STATE_DIR" PI_OBSERVE_PROJECT=vire PI_OBSERVE_IDLE_THRESHOLD_MS=0 "$ROOT_DIR/observability/pi-observe/bin/pi-observe.mjs" reconcile >/dev/null
printf 'Local events written to %s/events.jsonl\n' "$STATE_DIR"
PUBLIC_KEY="$(read_env_value LANGFUSE_PUBLIC_KEY || true)"
SECRET_KEY="$(read_env_value LANGFUSE_SECRET_KEY || true)"
if [[ -n "$PUBLIC_KEY" && -n "$SECRET_KEY" ]]; then
  printf 'Checking whether Langfuse ingestion accepts a smoke trace...\n'
  PI_OBSERVE_STATE_DIR="$STATE_DIR" PI_OBSERVE_PROJECT=vire "$ROOT_DIR/observability/pi-observe/bin/pi-observe.mjs" smoke-ingest --project vire
  printf 'Ingestion API accepted the smoke trace including response-body checks; check Langfuse UI for trace pi-observe.smoke-ingest if you need visual confirmation.\n'
else
  printf 'LANGFUSE_PUBLIC_KEY/SECRET_KEY are not configured; local event-store smoke test completed without remote trace.\n'
fi
