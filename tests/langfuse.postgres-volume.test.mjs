import test from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const up = new URL('../scripts/langfuse-up.sh', import.meta.url).pathname;
const down = new URL('../scripts/langfuse-down.sh', import.meta.url).pathname;
const upScript = readFileSync(up, 'utf8');
const downScript = readFileSync(down, 'utf8');
const setupScript = readFileSync(new URL('../scripts/setup-local-observability.sh', import.meta.url), 'utf8');
const readme = readFileSync(new URL('../observability/langfuse/README.md', import.meta.url), 'utf8');

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'langfuse-pg-preflight-'));
  const lf = join(root, 'observability/langfuse');
  const bin = join(root, 'fake-bin');
  mkdirSync(lf, { recursive: true });
  mkdirSync(bin, { recursive: true });
  writeFileSync(join(lf, '.env'), [
    'LANGFUSE_HOST=https://user:password@example.com:8443/path?token=secret#frag',
    'POSTGRES_USER=langfuse',
    'POSTGRES_PASSWORD=new-secret-value',
    'POSTGRES_DB=langfuse',
    '',
  ].join('\n'));
  return { root, lf, bin, log: join(root, 'docker.log'), count: join(root, 'exec-count') };
}

function installFakeDocker({ bin, log, count }) {
  writeFileSync(join(bin, 'docker'), `#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$*" >> "${log}"
if [[ "$1" == "compose" && "\${2:-}" == "version" ]]; then exit 0; fi
if [[ "$1" == "volume" && "\${2:-}" == "inspect" ]]; then
  [[ "\${FAKE_VOLUME_EXISTS:-true}" == "true" ]] && exit 0 || exit 1
fi
if [[ "$1" == "compose" ]]; then
  shift
  if [[ "\${1:-}" == "--env-file" ]]; then shift 2; fi
  case "\${1:-}" in
    up)
      if [[ " $* " == *" postgres "* ]]; then printf 'up-postgres\\n' >> "${log}"; else printf 'up-full\\n' >> "${log}"; fi
      exit 0
      ;;
    exec)
      current=0
      if [[ -f "${count}" ]]; then current="$(cat "${count}")"; fi
      current=$((current + 1))
      printf '%s' "$current" > "${count}"
      if [[ "\${PGPASSWORD:-}" != "\${EXPECTED_PASSWORD:-new-secret-value}" || "\${PGUSER:-}" != "langfuse" || "\${PGDATABASE:-}" != "langfuse" ]]; then
        printf 'psql: error: FATAL: password authentication failed for user\\n' >&2
        exit 2
      fi
      case "\${FAKE_EXEC_MODE:-success}" in
        success)
          exit 0
          ;;
        readiness-then-success)
          if (( current == 1 )); then
            printf 'psql: error: connection to server failed: Connection refused\\n' >&2
            exit 2
          fi
          exit 0
          ;;
        auth-failure)
          printf 'psql: error: FATAL: password authentication failed for user\\n' >&2
          exit 2
          ;;
      esac
      ;;
  esac
fi
exit 0
`);
  chmodSync(join(bin, 'docker'), 0o755);
}

function runUp(root, bin, extraEnv = {}) {
  return spawnSync('bash', [up], {
    encoding: 'utf8',
    env: {
      ...process.env,
      PI_OBSERVE_ROOT_DIR: root,
      PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'true',
      LANGFUSE_POSTGRES_PREFLIGHT_RETRIES: '3',
      LANGFUSE_POSTGRES_PREFLIGHT_SLEEP_SECONDS: '0',
      PATH: `${bin}:${process.env.PATH}`,
      ...extraEnv,
    },
  });
}

test('langfuse-up verifies existing Postgres volume credentials before starting full stack', () => {
  assert.match(upScript, /postgres_volume_exists\(\)/, 'up script should detect an existing Postgres named volume');
  assert.match(upScript, /vire-local-langfuse_langfuse_postgres/, 'up script should check the Compose-prefixed volume name');
  assert.match(upScript, /env_value POSTGRES_PASSWORD/, 'up script should read current .env Postgres password on the host');
  assert.match(upScript, /-e PGUSER -e PGDATABASE -e PGPASSWORD postgres/, 'up script should pass current host credentials into docker compose exec without putting secret values in argv');
  assert.match(upScript, /for \(\(attempt = 1; attempt <= max_attempts; attempt\+\+\)\)/, 'up script should retry while Postgres starts');
  assert.match(upScript, /Prisma P1000|Postgres rejected the credentials/, 'up script should explain the auth failure');
  assert.match(upScript, /\.\/scripts\/langfuse-down\.sh -v/, 'up script should provide the local reset command');
});

test('langfuse-up retries transient Postgres readiness failures before allowing startup', () => {
  const ctx = makeRoot();
  installFakeDocker(ctx);
  const res = runUp(ctx.root, ctx.bin, { FAKE_EXEC_MODE: 'readiness-then-success' });
  assert.equal(res.status, 0, res.stderr);
  assert.equal(readFileSync(ctx.count, 'utf8'), '2');
  assert.match(readFileSync(ctx.log, 'utf8'), /up-full/);
  assert.doesNotMatch(res.stdout + res.stderr, /new-secret-value|password authentication failed|reset the local volumes/);
});

test('langfuse-up reports auth failure without printing the current .env password', () => {
  const ctx = makeRoot();
  installFakeDocker(ctx);
  const res = runUp(ctx.root, ctx.bin, { FAKE_EXEC_MODE: 'auth-failure' });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Postgres rejected the credentials/);
  assert.match(res.stderr, /langfuse-down\.sh -v\n/);
  assert.match(res.stderr, /non-interactive automation only, pass --force/);
  assert.doesNotMatch(res.stdout + res.stderr, /new-secret-value/);
});

test('langfuse-up skips credential exec when no local Postgres volume exists', () => {
  const ctx = makeRoot();
  installFakeDocker(ctx);
  const res = runUp(ctx.root, ctx.bin, { FAKE_VOLUME_EXISTS: 'false' });
  assert.equal(res.status, 0, res.stderr);
  assert.equal(existsSync(ctx.count), false, 'psql auth check should not run without an existing volume');
  assert.match(readFileSync(ctx.log, 'utf8'), /up-full/);
});

test('setup warns when it generates POSTGRES_PASSWORD after a Postgres volume already exists', () => {
  assert.match(setupScript, /generated_postgres_password=false/, 'setup should track whether it generated a new Postgres password');
  assert.match(setupScript, /generated_postgres_password" == "true".*postgres_volume_exists/s, 'setup should warn only when a new password coincides with an existing volume');
  assert.match(setupScript, /Postgres only applies POSTGRES_PASSWORD when its data directory is first initialized/, 'setup warning should state why the mismatch happens');
});

test('langfuse-down requires explicit confirmation or force for volume deletion', () => {
  assert.match(downScript, /Type DELETE to continue/, 'down script should require typed confirmation for interactive -v');
  assert.match(downScript, /LANGFUSE_DOWN_FORCE/, 'down script should preserve non-interactive scriptability with an explicit force env var');
  assert.match(downScript, /--force\|-f/, 'down script should support an explicit force flag');
});

test('README documents Prisma P1000 password and volume mismatch remediation', () => {
  assert.match(readme, /Prisma `P1000` \/ Postgres authentication failed/, 'README should have a targeted troubleshooting section');
  assert.match(readme, /`pg_isready` checks readiness, not password authentication/, 'README should explain why healthcheck can pass');
  assert.match(readme, /\.\/scripts\/langfuse-down\.sh -v/, 'README should document destructive reset command');
  assert.match(readme, /confirmation before deleting volumes/, 'README should document the deletion confirmation');
  assert.match(readme, /do \*\*not\*\* delete volumes/i, 'README should warn users with important data not to delete volumes');
});
