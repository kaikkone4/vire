import test from 'node:test';
import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const down = new URL('../scripts/langfuse-down.sh', import.meta.url).pathname;

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'langfuse-down-confirm-'));
  const lf = join(root, 'observability/langfuse');
  const bin = join(root, 'fake-bin');
  mkdirSync(lf, { recursive: true });
  mkdirSync(bin, { recursive: true });
  writeFileSync(join(lf, '.env'), 'POSTGRES_PASSWORD=dummy\n');
  const log = join(root, 'docker.log');
  writeFileSync(join(bin, 'docker'), `#!/usr/bin/env bash\nprintf '%s\\n' "$*" >> "${log}"\nexit 0\n`);
  chmodSync(join(bin, 'docker'), 0o755);
  return { root, bin, log };
}

function runDown(ctx, args = [], extraEnv = {}) {
  return spawnSync('bash', [down, ...args], {
    encoding: 'utf8',
    input: '',
    env: {
      ...process.env,
      PI_OBSERVE_ROOT_DIR: ctx.root,
      PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'true',
      PATH: `${ctx.bin}:${process.env.PATH}`,
      ...extraEnv,
    },
  });
}

test('langfuse-down -v refuses non-interactive volume deletion without invoking docker', () => {
  const ctx = makeRoot();
  const res = runDown(ctx, ['-v']);
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /Volume deletion requires confirmation/);
  assert.equal(existsSync(ctx.log), false, 'docker compose down -v must not run before confirmation');
});

test('langfuse-down -v --force preserves explicit automation path', () => {
  const ctx = makeRoot();
  const res = runDown(ctx, ['-v', '--force']);
  assert.equal(res.status, 0, res.stderr);
  assert.match(readFileSync(ctx.log, 'utf8'), /compose --env-file \.env down -v/);
  assert.match(res.stdout, /named volumes were requested for deletion/);
});
