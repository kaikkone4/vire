#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="${PI_OBSERVE_ROOT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
LF_DIR="$ROOT_DIR/observability/langfuse"
ENV_FILE="$LF_DIR/.env"
EXAMPLE_FILE="$LF_DIR/.env.example"

say(){ printf '\n==> %s\n' "$*"; }
ask(){ printf '%s [y/N] ' "$*"; read -r ans || true; case "$ans" in y|Y|yes|YES) return 0;; *) return 1;; esac; }
have(){ command -v "$1" >/dev/null 2>&1; }
open_url(){ if have open; then open "$1"; elif have xdg-open; then xdg-open "$1"; else printf 'Open manually: %s\n' "$1"; fi; }
secret(){
  local len="${1:-48}"
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex "$(( (len + 1) / 2 ))" | cut -c1-"$len"
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "$len" <<'PY'
import secrets, string, sys
alphabet = string.ascii_letters + string.digits
print(''.join(secrets.choice(alphabet) for _ in range(int(sys.argv[1]))))
PY
  else
    # Finite dd input avoids the tr|head SIGPIPE failure mode under pipefail.
    LC_ALL=C dd if=/dev/urandom bs=256 count=4 2>/dev/null | LC_ALL=C tr -dc 'A-Za-z0-9' | awk -v n="$len" '{ out = out $0 } END { if (length(out) < n) exit 1; print substr(out, 1, n) }'
  fi
}
replace_empty(){ local key="$1" value="$2"; perl -0pi -e "s/^$key=\s*\$/$key=$value/m" "$ENV_FILE"; }

say "Local observability setup (Langfuse + pi-observe)"
printf 'OS: %s / %s\n' "$(uname -s)" "$(uname -m)"

if ! have docker; then
  say "Docker CLI was not found. This script will not install system software automatically."
  if ask "Open Docker Desktop installation docs?"; then open_url "https://docs.docker.com/desktop/"; fi
  if have brew && ask "Open Colima installation docs instead?"; then open_url "https://github.com/abiosoft/colima"; fi
else
  say "Docker found: $(docker --version)"
  if ! docker compose version >/dev/null 2>&1; then
    say "docker compose v2 was not found."
    if ask "Open Docker Compose installation docs?"; then open_url "https://docs.docker.com/compose/install/"; fi
  else
    printf 'Compose: %s\n' "$(docker compose version)"
  fi
  if ! docker info >/dev/null 2>&1; then
    say "Docker daemon is not running or not reachable."
    if have colima && ask "Colima is installed. Start it now?"; then colima start; fi
    if [[ "$(uname -s)" == "Darwin" ]] && ask "Try opening Docker Desktop now?"; then open -a Docker || true; fi
  fi
fi

if ! have node; then
  say "Node.js was not found; pi-observe needs Node 18+ for built-in fetch."
  if ask "Open Node.js installation docs?"; then open_url "https://nodejs.org/en/download"; fi
else
  printf 'Node: %s\n' "$(node --version)"
fi
if ! have npm; then
  say "npm was not found."
  if ask "Open npm docs?"; then open_url "https://docs.npmjs.com/downloading-and-installing-node-js-and-npm"; fi
else
  printf 'npm: %s\n' "$(npm --version)"
fi
if ! have cargo; then
  say "Rust/Cargo was not found. Vire native Tauri builds need Rust, but Langfuse does not."
  if ask "Open rustup installation instructions?"; then open_url "https://rustup.rs/"; fi
else
  printf 'Cargo: %s\n' "$(cargo --version)"
fi

if [[ ! -f "$ENV_FILE" ]]; then
  say "Creating local .env from .env.example (secrets stay local and are gitignored)."
  cp "$EXAMPLE_FILE" "$ENV_FILE"
  chmod 600 "$ENV_FILE"
else
  say ".env already exists; not overwriting."
fi

say "Generating missing local-only secrets in .env."
for key in NEXTAUTH_SECRET SALT POSTGRES_PASSWORD CLICKHOUSE_PASSWORD REDIS_PASSWORD MINIO_ROOT_PASSWORD LANGFUSE_INIT_USER_PASSWORD; do
  if grep -q "^$key=\s*$" "$ENV_FILE"; then replace_empty "$key" "$(secret 48)"; fi
done
if grep -q '^ENCRYPTION_KEY=\s*$' "$ENV_FILE"; then replace_empty ENCRYPTION_KEY "$(openssl rand -hex 32 2>/dev/null || secret 64)"; fi

PORT="$(grep -E '^LANGFUSE_PORT=' "$ENV_FILE" | tail -1 | cut -d= -f2 || true)"; PORT="${PORT:-3000}"
if command -v lsof >/dev/null 2>&1 && lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  say "Port $PORT is already in use. Edit $ENV_FILE and set LANGFUSE_PORT to another localhost port before starting."
fi

mkdir -p "$HOME/.local/bin"
ln -sf "$ROOT_DIR/observability/pi-observe/bin/pi-observe.mjs" "$HOME/.local/bin/pi-observe"
say "Installed/updated pi-observe symlink at $HOME/.local/bin/pi-observe"
printf 'Add this to your shell profile if needed: export PATH="$HOME/.local/bin:$PATH"\n'

if ask "Start the local Langfuse stack now?"; then
  "$ROOT_DIR/scripts/langfuse-up.sh"
else
  say "Skipped start. Later run: ./scripts/langfuse-up.sh"
fi

say "Next steps"
printf '1. Open http://localhost:%s and sign in with LANGFUSE_INIT_USER_EMAIL/PASSWORD from your local .env.\n' "$PORT"
printf '2. Create/copy project API keys into observability/langfuse/.env as LANGFUSE_PUBLIC_KEY and LANGFUSE_SECRET_KEY.\n'
printf '3. Run ./scripts/langfuse-smoke-test.sh\n'
