#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="$ROOT_DIR/observability/langfuse/.env"
read_env_value() {
  local key="$1"
  [[ -f "$ENV_FILE" ]] || return 1
  awk -F= -v k="$key" '
    $0 ~ "^[[:space:]]*#" || $0 !~ "=" { next }
    $1 == k { sub(/^[^=]*=/, ""); gsub(/^[[:space:]]+|[[:space:]]+$/, ""); gsub(/^\"|\"$/, ""); gsub(/^'"'"'|'"'"'$/, ""); print; exit }
  ' "$ENV_FILE"
}
HOST="$(read_env_value LANGFUSE_HOST || true)"; HOST="${HOST:-http://localhost:3000}"
printf 'Checking Langfuse health at %s ...\n' "$HOST"
if command -v curl >/dev/null 2>&1; then
  if curl -fsS "$HOST/api/public/health" >/dev/null; then
    printf 'Health endpoint responded.\n'
  else
    printf 'Health endpoint not ready yet or unavailable; continuing with fail-open wrapper test.\n'
  fi
fi
STATE_DIR="$(mktemp -d)"
PI_OBSERVE_STATE_DIR="$STATE_DIR" PI_OBSERVE_PROJECT=vire "$ROOT_DIR/observability/pi-observe/bin/pi-observe.mjs" run --tool smoke-test --role verification --summary "local smoke test" -- node -e "console.log('pi-observe smoke ok')"
printf 'Local events written to %s/events.jsonl\n' "$STATE_DIR"
PUBLIC_KEY="$(read_env_value LANGFUSE_PUBLIC_KEY || true)"
SECRET_KEY="$(read_env_value LANGFUSE_SECRET_KEY || true)"
if [[ -n "$PUBLIC_KEY" && -n "$SECRET_KEY" ]]; then
  printf 'Checking whether Langfuse ingestion accepts a smoke trace...\n'
  PI_OBSERVE_STATE_DIR="$STATE_DIR" PI_OBSERVE_PROJECT=vire "$ROOT_DIR/observability/pi-observe/bin/pi-observe.mjs" smoke-ingest --project vire
  printf 'If accepted, check Langfuse UI for trace pi-observe.smoke-ingest.\n'
else
  printf 'LANGFUSE_PUBLIC_KEY/SECRET_KEY are not configured; local event-store smoke test completed without remote trace.\n'
fi
