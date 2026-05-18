import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, mkdirSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const cli = new URL('../observability/pi-observe/bin/pi-observe.mjs', import.meta.url).pathname;
const setup = new URL('../scripts/setup-local-observability.sh', import.meta.url).pathname;
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
  const first = spawnSync('bash', [setup], { input: 'n\nn\nn\nn\nn\n', encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, HOME: home, PATH: process.env.PATH } });
  assert.equal(first.status, 0, first.stderr);
  const envPath = join(lf, '.env');
  const envText = readFileSync(envPath, 'utf8');
  assert.match(envText, /^NEXTAUTH_SECRET=[A-Za-z0-9]+/m);
  assert.match(envText, /^POSTGRES_PASSWORD=[A-Za-z0-9]+/m);
  assert.equal((statSync(envPath).mode & 0o777), 0o600);
  writeFileSync(envPath, 'NEXTAUTH_SECRET=keep-me\n');
  const second = spawnSync('bash', [setup], { input: 'n\nn\nn\nn\nn\n', encoding: 'utf8', env: { ...process.env, PI_OBSERVE_ROOT_DIR: root, HOME: home, PATH: process.env.PATH } });
  assert.equal(second.status, 0, second.stderr);
  assert.equal(readFileSync(envPath, 'utf8'), 'NEXTAUTH_SECRET=keep-me\n');
});
