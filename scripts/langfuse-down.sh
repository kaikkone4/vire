#!/usr/bin/env bash
set -euo pipefail
SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS:-false}" == "true" && -n "${PI_OBSERVE_ROOT_DIR:-}" ]]; then
  ROOT_DIR="$PI_OBSERVE_ROOT_DIR"
else
  ROOT_DIR="$SCRIPT_ROOT"
fi
cd "$ROOT_DIR/observability/langfuse"

compose_args=()
deletes_volumes=false
force=false
for arg in "$@"; do
  case "$arg" in
    -v|--volumes)
      deletes_volumes=true
      compose_args+=("$arg")
      ;;
    --force|-f)
      force=true
      ;;
    *)
      compose_args+=("$arg")
      ;;
  esac
done

if [[ "${LANGFUSE_DOWN_FORCE:-false}" == "true" || "${LANGFUSE_DOWN_FORCE:-}" == "1" ]]; then
  force=true
fi

if [[ "$deletes_volumes" == "true" && "$force" != "true" ]]; then
  printf 'WARNING: ./scripts/langfuse-down.sh -v deletes local Langfuse Docker volumes (Postgres, Redis, ClickHouse, MinIO).\n' >&2
  printf 'This removes local observability data and cannot be undone unless you have backups/exports.\n' >&2
  if [[ -t 0 ]]; then
    printf 'Type DELETE to continue: ' >&2
    read -r confirmation || true
    if [[ "$confirmation" != "DELETE" ]]; then
      printf 'Volume deletion cancelled. Rerun without -v to stop containers while preserving data.\n' >&2
      exit 1
    fi
  else
    printf 'Volume deletion requires confirmation. In non-interactive use, pass --force or set LANGFUSE_DOWN_FORCE=true.\n' >&2
    exit 1
  fi
fi

if (( ${#compose_args[@]} > 0 )); then
  docker compose --env-file .env down "${compose_args[@]}"
else
  docker compose --env-file .env down
fi
if [[ "$deletes_volumes" == "true" ]]; then
  printf 'Langfuse containers stopped and named volumes were requested for deletion.\n'
else
  printf 'Langfuse containers stopped. Named volumes are preserved unless you passed -v.\n'
fi
