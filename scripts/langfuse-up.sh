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
postgres_volume_exists() {
  docker volume inspect vire-local-langfuse_langfuse_postgres >/dev/null 2>&1 \
    || docker volume inspect langfuse_postgres >/dev/null 2>&1
}
check_postgres_credentials() {
  if ! command -v docker >/dev/null 2>&1 || ! docker compose version >/dev/null 2>&1; then
    return 0
  fi
  if ! postgres_volume_exists; then
    return 0
  fi

  printf 'Existing Langfuse Postgres volume detected; verifying .env database credentials before starting Langfuse.\n'
  docker compose --env-file .env up -d postgres >/dev/null
  if ! docker compose --env-file .env exec -T postgres sh -c 'PGPASSWORD="$POSTGRES_PASSWORD" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc "SELECT 1"' >/dev/null; then
    printf '\nPostgres rejected the credentials from observability/langfuse/.env.\n' >&2
    printf 'Most likely the named Docker volume was initialized with an older POSTGRES_PASSWORD; Postgres ignores later POSTGRES_PASSWORD changes for existing data directories.\n' >&2
    printf 'If you do not need the local Langfuse data, reset the local volumes:\n' >&2
    printf '  ./scripts/langfuse-down.sh -v\n' >&2
    printf '  ./scripts/langfuse-up.sh\n' >&2
    printf 'If you need the data, restore the previous POSTGRES_PASSWORD in .env or perform a manual Postgres password rotation from inside the database.\n' >&2
    exit 1
  fi
}
check_postgres_credentials
docker compose --env-file .env up -d
printf 'Langfuse starting on %s (health may take a minute).\n' "$(sanitize_url_for_display "$raw_host")"
