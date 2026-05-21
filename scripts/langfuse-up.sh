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
env_value() {
  local key="$1" default="${2:-}"
  local value
  value="$(awk -v key="$key" '
    $0 ~ "^[[:space:]]*#" { next }
    $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
      sub(/^[^=]*=/, "")
      gsub(/^[[:space:]]+|[[:space:]]+$/, "")
      if (($0 ~ /^".*"$/) || ($0 ~ /^'"'"'.*'"'"'$/)) $0 = substr($0, 2, length($0) - 2)
      print
      exit
    }
  ' .env 2>/dev/null || true)"
  printf '%s\n' "${value:-$default}"
}
raw_host="$(env_value LANGFUSE_HOST http://localhost:3000)"
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

  local pg_user pg_password pg_db max_attempts sleep_seconds attempt err_file rc
  pg_user="$(env_value POSTGRES_USER langfuse)"
  pg_password="$(env_value POSTGRES_PASSWORD)"
  pg_db="$(env_value POSTGRES_DB langfuse)"
  max_attempts="${LANGFUSE_POSTGRES_PREFLIGHT_RETRIES:-30}"
  sleep_seconds="${LANGFUSE_POSTGRES_PREFLIGHT_SLEEP_SECONDS:-2}"
  if [[ -z "$pg_password" ]]; then
    return 0
  fi

  printf 'Existing Langfuse Postgres volume detected; verifying .env database credentials before starting Langfuse.\n'
  docker compose --env-file .env up -d postgres >/dev/null
  err_file="$(mktemp "${TMPDIR:-/tmp}/langfuse-postgres-preflight.XXXXXX")"
  trap 'rm -f "$err_file"' RETURN
  for ((attempt = 1; attempt <= max_attempts; attempt++)); do
    : >"$err_file"
    if PGUSER="$pg_user" PGDATABASE="$pg_db" PGPASSWORD="$pg_password" \
      docker compose --env-file .env exec -T -e PGUSER -e PGDATABASE -e PGPASSWORD postgres \
      sh -c 'psql -U "$PGUSER" -d "$PGDATABASE" -tAc "SELECT 1"' >/dev/null 2>"$err_file"; then
      rm -f "$err_file"
      trap - RETURN
      return 0
    else
      rc=$?
    fi
    if grep -Eiq 'password authentication failed|authentication failed|no password supplied|role ".*" does not exist|database ".*" does not exist' "$err_file"; then
      rm -f "$err_file"
      trap - RETURN
      printf '\nPostgres rejected the credentials from observability/langfuse/.env.\n' >&2
      printf 'Most likely the named Docker volume was initialized with an older POSTGRES_PASSWORD; Postgres ignores later POSTGRES_PASSWORD changes for existing data directories.\n' >&2
      printf 'If you do not need the local Langfuse data, reset the local volumes interactively:\n' >&2
      printf '  ./scripts/langfuse-down.sh -v\n' >&2
      printf '  ./scripts/langfuse-up.sh\n' >&2
      printf 'For non-interactive automation only, pass --force or set LANGFUSE_DOWN_FORCE=true after confirming data deletion is intentional.\n' >&2
      printf 'If you need the data, restore the previous POSTGRES_PASSWORD in .env or perform a manual Postgres password rotation from inside the database.\n' >&2
      printf 'Postgres may have been started for this check; stop it with ./scripts/langfuse-down.sh if needed.\n' >&2
      exit 1
    fi
    if (( attempt < max_attempts )); then
      sleep "$sleep_seconds"
    fi
  done
  rm -f "$err_file"
  trap - RETURN
  printf '\nPostgres did not become ready for an authenticated credential check after %s attempts.\n' "$max_attempts" >&2
  printf 'This looks like a startup/readiness problem, not a confirmed password mismatch. Check logs with:\n' >&2
  printf '  cd observability/langfuse && docker compose --env-file .env logs postgres --tail=100\n' >&2
  exit "$rc"
}
check_postgres_credentials
docker compose --env-file .env up -d
printf 'Langfuse starting on %s (health may take a minute).\n' "$(sanitize_url_for_display "$raw_host")"
