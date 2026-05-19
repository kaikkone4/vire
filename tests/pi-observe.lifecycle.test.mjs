import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, statSync, chmodSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const cli = new URL('../observability/pi-observe/bin/pi-observe.mjs', import.meta.url).pathname;
const setup = new URL('../scripts/setup-local-observability.sh', import.meta.url).pathname;
const langfuseUp = new URL('../scripts/langfuse-up.sh', import.meta.url).pathname;
const langfuseDown = new URL('../scripts/langfuse-down.sh', import.meta.url).pathname;
const langfuseSmoke = new URL('../scripts/langfuse-smoke-test.sh', import.meta.url).pathname;
const exampleEnv = new URL('../observability/langfuse/.env.example', import.meta.url).pathname;

function dirs(prefix = 'pi-observe-life-') {
  const root = mkdtempSync(join(tmpdir(), prefix));
  return { root, state: join(root, 'state'), config: join(root, 'config'), dotenv: join(root, '.env') };
}
function runCli(args, d, env = {}) {
  return spawnSync(process.execPath, [cli, ...args], { encoding: 'utf8', env: { ...process.env, PI_OBSERVE_STATE_DIR: d.state, PI_OBSERVE_CONFIG_DIR: d.config, PI_OBSERVE_DOTENV: d.dotenv, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '', ...env } });
}
function events(state) { return readFileSync(join(state, 'events.jsonl'), 'utf8').trim().split('\n').map(JSON.parse); }
function active(state) { return JSON.parse(readFileSync(join(state, 'runs.json'), 'utf8')).active; }

test('reconcile emits idle_started after countdown threshold elapses', async () => {
  const d = dirs();
  const res = runCli(['run', '--project', 'vire', '--tool', 'idle-test', '--', process.execPath, '-e', ''], d, { PI_OBSERVE_IDLE_THRESHOLD_MS: '1' });
  assert.equal(res.status, 0, res.stderr);
  await new Promise(resolve => setTimeout(resolve, 20));
  const rec = runCli(['reconcile'], d, { PI_OBSERVE_IDLE_THRESHOLD_MS: '1' });
  assert.equal(rec.status, 0, rec.stderr);
  assert.equal(events(d.state).at(-1).event, 'idle_started');
  assert.ok(active(d.state).vire.idle_started_at);
});

test('reconcile marks orphaned active runs after timeout and starts idle countdown', async () => {
  const d = dirs();
  const activeState = { active: { vire: { count: 1, runs: ['orphan-run'], run_started_at: { 'orphan-run': new Date(Date.now() - 1000).toISOString() }, idle_countdown_started_at: null } } };
  mkdirSync(d.state, { recursive: true });
  writeFileSync(join(d.state, 'runs.json'), JSON.stringify(activeState));
  const rec = runCli(['reconcile'], d, { PI_OBSERVE_ORPHAN_TIMEOUT_MS: '1' });
  assert.equal(rec.status, 0, rec.stderr);
  const ev = events(d.state).map(e => e.event);
  assert.ok(ev.includes('tool_orphaned'));
  assert.ok(ev.includes('idle_countdown_started'));
  assert.equal(active(d.state).vire.count, 0);
  assert.deepEqual(active(d.state).vire.runs, []);
});

test('setup first run creates chmod-600 env with generated secrets and does not overwrite existing env', () => {
  const root = mkdtempSync(join(tmpdir(), 'pi-setup-'));
  const lf = join(root, 'observability/langfuse');
  mkdirSync(lf, { recursive: true });
  writeFileSync(join(lf, '.env.example'), readFileSync(exampleEnv));
  mkdirSync(join(root, 'observability/pi-observe/bin'), { recursive: true });
  writeFileSync(join(root, 'observability/pi-observe/bin/pi-observe.mjs'), '#!/usr/bin/env node\n');
  const home = join(root, 'home'); mkdirSync(home);
  const first = spawnSync('bash', [setup], { input: 'n\nn\nn\nn\nn\n', encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'true', HOME: home, PATH: process.env.PATH } });
  assert.equal(first.status, 0, first.stderr);
  const envPath = join(lf, '.env');
  const envText = readFileSync(envPath, 'utf8');
  assert.match(envText, /^NEXTAUTH_SECRET=[A-Za-z0-9]+/m);
  assert.match(envText, /^POSTGRES_PASSWORD=[A-Za-z0-9]+/m);
  assert.equal((statSync(envPath).mode & 0o777), 0o600);
  writeFileSync(envPath, 'NEXTAUTH_SECRET=keep-me\n');
  const second = spawnSync('bash', [setup], { input: 'n\nn\nn\nn\nn\n', encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'true', HOME: home, PATH: process.env.PATH } });
  assert.equal(second.status, 0, second.stderr);
  assert.equal(readFileSync(envPath, 'utf8'), 'NEXTAUTH_SECRET=keep-me\n');
  assert.equal((statSync(envPath).mode & 0o777), 0o600);
});

test('setup fills whitespace-only secret values without overwriting non-empty values', () => {
  const root = mkdtempSync(join(tmpdir(), 'pi-setup-empty-'));
  const lf = join(root, 'observability/langfuse');
  mkdirSync(lf, { recursive: true });
  writeFileSync(join(lf, '.env.example'), readFileSync(exampleEnv));
  writeFileSync(join(lf, '.env'), 'NEXTAUTH_SECRET=   \nSALT=\t \nENCRYPTION_KEY=   \nPOSTGRES_PASSWORD=keep-me\nLANGFUSE_PORT=3000\n');
  const home = join(root, 'home'); mkdirSync(home);
  mkdirSync(join(root, 'observability/pi-observe/bin'), { recursive: true });
  writeFileSync(join(root, 'observability/pi-observe/bin/pi-observe.mjs'), '#!/usr/bin/env node\n');
  const res = spawnSync('bash', [setup], { input: 'n\nn\nn\nn\nn\n', encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'true', HOME: home, PATH: process.env.PATH } });
  assert.equal(res.status, 0, res.stderr);
  const envText = readFileSync(join(lf, '.env'), 'utf8');
  assert.match(envText, /^NEXTAUTH_SECRET=\S+/m);
  assert.match(envText, /^SALT=\S+/m);
  assert.match(envText, /^ENCRYPTION_KEY=\S+/m);
  assert.match(envText, /^POSTGRES_PASSWORD=keep-me$/m);
});

test('langfuse-down ignores PI_OBSERVE_ROOT_DIR unless explicit test override is enabled', () => {
  const root = mkdtempSync(join(tmpdir(), 'pi-down-root-'));
  const lf = join(root, 'observability/langfuse');
  const bin = join(root, 'fake-bin');
  const marker = join(root, 'docker-pwd');
  const scriptLf = new URL('../observability/langfuse', import.meta.url).pathname;
  mkdirSync(lf, { recursive: true });
  mkdirSync(bin, { recursive: true });
  writeFileSync(join(lf, '.env'), 'LANGFUSE_HOST=http://localhost:3000\n');
  writeFileSync(join(bin, 'docker'), `#!/usr/bin/env bash\npwd > "${marker}"\nexit 0\n`);
  chmodSync(join(bin, 'docker'), 0o755);

  const ignored = spawnSync('bash', [langfuseDown], { encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'false', PATH: `${bin}:${process.env.PATH}` } });
  assert.equal(ignored.status, 0, ignored.stderr);
  assert.equal(readFileSync(marker, 'utf8').trim(), scriptLf);

  const honored = spawnSync('bash', [langfuseDown], { encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'true', PATH: `${bin}:${process.env.PATH}` } });
  assert.equal(honored.status, 0, honored.stderr);
  assert.equal(readFileSync(marker, 'utf8').trim(), lf);
});

test('helper scripts display sanitized Langfuse host values only', () => {
  const root = mkdtempSync(join(tmpdir(), 'pi-helper-host-'));
  const lf = join(root, 'observability/langfuse');
  const bin = join(root, 'fake-bin');
  mkdirSync(lf, { recursive: true });
  mkdirSync(bin, { recursive: true });
  writeFileSync(join(lf, '.env'), 'LANGFUSE_HOST=https://user:password@example.com:8443/path?token=secret#frag\n');
  writeFileSync(join(bin, 'docker'), '#!/usr/bin/env bash\nexit 0\n');
  chmodSync(join(bin, 'docker'), 0o755);
  const res = spawnSync('bash', [langfuseUp], { encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'true', PATH: `${bin}:${process.env.PATH}` } });
  assert.equal(res.status, 0, res.stderr);
  assert.match(res.stdout, /https:\/\/example\.com:8443/);
  assert.doesNotMatch(res.stdout + res.stderr, /user|password|token=secret|frag|\/path/);
});

test('smoke test sanitizes remote host display and skips curl without explicit opt-in', () => {
  const root = mkdtempSync(join(tmpdir(), 'pi-smoke-host-'));
  const lf = join(root, 'observability/langfuse');
  const piBin = join(root, 'observability/pi-observe/bin');
  const fakeBin = join(root, 'fake-bin');
  const curlMarker = join(root, 'curl-called');
  mkdirSync(lf, { recursive: true });
  mkdirSync(piBin, { recursive: true });
  mkdirSync(fakeBin, { recursive: true });
  writeFileSync(join(lf, '.env'), 'LANGFUSE_HOST=https://user:password@example.com:8443/path?token=secret#frag\n');
  writeFileSync(join(piBin, 'pi-observe.mjs'), '#!/usr/bin/env bash\nif [[ "$1" == "run" ]]; then echo "stub run"; fi\nexit 0\n');
  chmodSync(join(piBin, 'pi-observe.mjs'), 0o755);
  writeFileSync(join(fakeBin, 'curl'), `#!/usr/bin/env bash\ntouch "${curlMarker}"\nexit 0\n`);
  chmodSync(join(fakeBin, 'curl'), 0o755);
  const res = spawnSync('bash', [langfuseSmoke], { encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, PI_OBSERVE_ALLOW_ROOT_OVERRIDE_FOR_TESTS: 'true', PATH: `${fakeBin}:${process.env.PATH}` } });
  assert.equal(res.status, 0, res.stderr);
  assert.match(res.stdout, /https:\/\/example\.com:8443/);
  assert.match(res.stdout, /Skipping health curl for non-loopback/);
  assert.doesNotMatch(res.stdout + res.stderr, /user|password|token=secret|frag|\/path/);
  assert.equal(existsSync(curlMarker), false, 'curl should not run for remote host without explicit opt-in');
});
