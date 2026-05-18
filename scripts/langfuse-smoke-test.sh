#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="$ROOT_DIR/observability/langfuse/.env"
if [[ -f "$ENV_FILE" ]]; then set -a; # shellcheck disable=SC1090
  source "$ENV_FILE"; set +a; fi
HOST="${LANGFUSE_HOST:-http://localhost:3000}"
printf 'Checking Langfuse health at %s ...\n' "$HOST"
if command -v curl >/dev/null 2>&1; then
  curl -fsS "$HOST/api/public/health" >/dev/null && printf 'Health endpoint responded.\n' || printf 'Health endpoint not ready yet; continuing with fail-open wrapper test.\n'
fi
STATE_DIR="$(mktemp -d)"
PI_OBSERVE_STATE_DIR="$STATE_DIR" PI_OBSERVE_PROJECT=vire "$ROOT_DIR/observability/pi-observe/bin/pi-observe.mjs" run --tool smoke-test --role verification --summary "local smoke test" -- node -e "console.log('pi-observe smoke ok')"
printf 'Local events written to %s/events.jsonl\n' "$STATE_DIR"
if [[ -n "${LANGFUSE_PUBLIC_KEY:-}" && -n "${LANGFUSE_SECRET_KEY:-}" ]]; then
  printf 'API keys are configured; check Langfuse UI for trace smoke-test.verification.\n'
else
  printf 'LANGFUSE_PUBLIC_KEY/SECRET_KEY are not configured; local event-store smoke test completed without remote trace.\n'
fi
