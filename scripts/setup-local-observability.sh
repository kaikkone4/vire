#!/usr/bin/env bash
set -euo pipefail
SCRIPT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ "${PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS:-false}" == "true" && -n "${PI_OBSERVE_ROOT_DIR:-}" ]]; then
  ROOT_DIR="$PI_OBSERVE_ROOT_DIR"
else
  ROOT_DIR="$SCRIPT_ROOT"
fi
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
is_empty_key(){
  local key="$1"
  awk -v key="$key" '
    $0 ~ "^[[:space:]]*" key "[[:space:]]*=" {
      val = $0; sub(/^[^=]*=/, "", val)
      if (val ~ /^[[:space:]]*$/) found = 1
    }
    END { exit(found ? 0 : 1) }
  ' "$ENV_FILE"
}
replace_empty(){
  local key="$1" value="$2"
  KEY="$key" VALUE="$value" perl -0pi -e 'my $k = $ENV{KEY}; my $v = $ENV{VALUE}; s/^[[:space:]]*\Q$k\E[[:space:]]*=[[:space:]]*$/$k=$v/mg' "$ENV_FILE"
}
ensure_safe_env_file(){
  if [[ -L "$ENV_FILE" ]]; then
    say "Refusing to use symlinked observability/langfuse/.env for safety."
    printf 'This setup script will not chmod or edit symlink targets.\n'
    printf 'Inspect with: ls -l "%s"\n' "$ENV_FILE"
    printf 'To continue, move the symlink aside and create a regular file from the template:\n'
    printf '  mv "%s" "%s.symlink-backup"\n' "$ENV_FILE" "$ENV_FILE"
    printf '  cp "%s" "%s"\n' "$EXAMPLE_FILE" "$ENV_FILE"
    printf '  chmod 600 "%s"\n' "$ENV_FILE"
    return 1
  fi
  if [[ ! -e "$ENV_FILE" ]]; then
    say "Creating local .env from .env.example (secrets stay local and are gitignored)."
    local tmp
    tmp="$(mktemp "$LF_DIR/.env.tmp.XXXXXX")"
    chmod 600 "$tmp"
    cp "$EXAMPLE_FILE" "$tmp"
    chmod 600 "$tmp"
    if ! ln "$tmp" "$ENV_FILE" 2>/dev/null; then
      rm -f "$tmp"
      say "Could not create $ENV_FILE safely; it may have appeared concurrently. Rerun setup after inspecting the path."
      return 1
    fi
    rm -f "$tmp"
  else
    if [[ ! -f "$ENV_FILE" ]]; then
      say "Refusing to use observability/langfuse/.env because it is not a regular file."
      printf 'Inspect with: ls -l "%s"\n' "$ENV_FILE"
      return 1
    fi
    say ".env already exists; not overwriting."
  fi
  local expected_dir actual_dir actual_name
  expected_dir="$(cd "$LF_DIR" && pwd -P)"
  actual_dir="$(cd "$(dirname "$ENV_FILE")" && pwd -P)"
  actual_name="$(basename "$ENV_FILE")"
  if [[ "$actual_dir" != "$expected_dir" || "$actual_name" != ".env" ]]; then
    say "Refusing to use .env outside observability/langfuse."
    printf 'Expected directory: %s\n' "$expected_dir"
    printf 'Actual path: %s/%s\n' "$actual_dir" "$actual_name"
    return 1
  fi
  if [[ -L "$ENV_FILE" || ! -f "$ENV_FILE" ]]; then
    say "Refusing to chmod/edit .env because it is no longer a regular non-symlink file."
    printf 'Inspect with: ls -l "%s"\n' "$ENV_FILE"
    return 1
  fi
  chmod 600 "$ENV_FILE"
}

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
  cargo_candidate=""
  candidate="$HOME/.cargo/bin/cargo"
  if [[ -x "$candidate" ]]; then cargo_candidate="$candidate"; fi
  if [[ -n "$cargo_candidate" ]]; then
    say "Rust/Cargo appears to be installed at $cargo_candidate, but it is not in this shell's PATH."
    printf 'Cargo: %s\n' "$($cargo_candidate --version 2>/dev/null || printf 'installed, version check unavailable')"
    printf 'To update this shell, run: source "$HOME/.cargo/env"\n'
    printf 'Or restart the terminal before running Vire/Tauri cargo commands.\n'
  else
    say "Rust/Cargo was not found. Vire native Tauri builds need Rust, but Langfuse does not."
    if ask "Open rustup installation instructions?"; then open_url "https://rustup.rs/"; fi
  fi
else
  printf 'Cargo: %s\n' "$(cargo --version)"
fi

if ! ensure_safe_env_file; then
  say "Setup cannot continue until observability/langfuse/.env is a regular local file."
  exit 1
fi

say "Ensured $ENV_FILE permissions are 0600 before filling missing secrets."
say "Generating missing local-only secrets in .env."
for key in NEXTAUTH_SECRET SALT POSTGRES_PASSWORD CLICKHOUSE_PASSWORD REDIS_PASSWORD MINIO_ROOT_PASSWORD LANGFUSE_INIT_USER_PASSWORD; do
  if is_empty_key "$key"; then replace_empty "$key" "$(secret 48)"; fi
done
if is_empty_key ENCRYPTION_KEY; then replace_empty ENCRYPTION_KEY "$(openssl rand -hex 32 2>/dev/null || secret 64)"; fi

PORT="$(grep -E '^LANGFUSE_PORT=' "$ENV_FILE" | tail -1 | cut -d= -f2 || true)"; PORT="${PORT:-3000}"
if command -v lsof >/dev/null 2>&1 && lsof -nP -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  say "Port $PORT is already in use. Edit $ENV_FILE and set LANGFUSE_PORT to another localhost port before starting."
fi

install_pi_observe_link(){
  local bin_dir="$HOME/.local/bin"
  local target="$bin_dir/pi-observe"
  local source="$ROOT_DIR/observability/pi-observe/bin/pi-observe.mjs"
  if [[ -e "$HOME/.local" && ! -d "$HOME/.local" ]]; then
    say "Cannot install pi-observe symlink: $HOME/.local exists but is not a directory."
    printf 'Fix manually, then rerun setup. No sudo was run by this script.\n'
    return 0
  fi
  if ! mkdir -p "$bin_dir" 2>/dev/null; then
    say "Cannot create $bin_dir; skipping pi-observe symlink."
    printf 'Inspect ownership/permissions with: ls -ld "%s" "%s" "%s"\n' "$HOME" "$HOME/.local" "$bin_dir"
    printf 'If ownership is wrong, fix explicitly, e.g.: sudo chown -R "%s" "%s"\n' "$(id -un)" "$HOME/.local"
    printf 'Then rerun setup. This script never runs sudo automatically.\n'
    return 0
  fi
  if [[ ! -w "$bin_dir" ]]; then
    say "$bin_dir is not writable; skipping pi-observe symlink."
    printf 'Inspect ownership/permissions with: ls -ld "%s"\n' "$bin_dir"
    printf 'If ownership is wrong, fix explicitly, e.g.: sudo chown -R "%s" "%s"\n' "$(id -un)" "$HOME/.local"
    printf 'Alternative: add an alias manually to %s\n' "$source"
    return 0
  fi
  if [[ -e "$target" && ! -w "$target" && ! -L "$target" ]]; then
    say "$target exists and is not writable; skipping pi-observe symlink."
    printf 'Inspect it with: ls -l "%s"\n' "$target"
    printf 'Move/remove it or fix ownership, then rerun setup. This script never runs sudo automatically.\n'
    return 0
  fi
  if ln -sfn "$source" "$target" 2>/dev/null; then
    say "Installed/updated pi-observe symlink at $target"
    printf 'Add this to your shell profile if needed: export PATH="$HOME/.local/bin:$PATH"\n'
  else
    say "Could not create pi-observe symlink at $target; continuing setup."
    printf 'Inspect ownership/permissions with: ls -ld "%s" "%s"\n' "$bin_dir" "$target"
    printf 'If ownership is wrong, fix explicitly, e.g.: sudo chown -R "%s" "%s"\n' "$(id -un)" "$HOME/.local"
    printf 'Manual fallback: run "%s" directly.\n' "$source"
  fi
}
install_pi_observe_link

if ask "Start the local Langfuse stack now?"; then
  "$ROOT_DIR/scripts/langfuse-up.sh"
else
  say "Skipped start. Later run: ./scripts/langfuse-up.sh"
fi

say "Next steps"
printf '1. Open http://localhost:%s and sign in with LANGFUSE_INIT_USER_EMAIL/PASSWORD from your local .env.\n' "$PORT"
printf '2. Create/copy project API keys into observability/langfuse/.env as LANGFUSE_PUBLIC_KEY and LANGFUSE_SECRET_KEY.\n'
printf '3. Run ./scripts/langfuse-smoke-test.sh\n'
