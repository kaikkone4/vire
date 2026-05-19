import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';

const cli = new URL('../observability/pi-observe/bin/pi-observe.mjs', import.meta.url).pathname;

function tempState() {
  const state = mkdtempSync(join(tmpdir(), 'pi-observe-adv-'));
  return { state, config: join(state, 'config') };
}

function run(args, options = {}) {
  const dirs = options.dirs || tempState();
  const res = spawnSync(process.execPath, [cli, ...args], {
    encoding: 'utf8',
    cwd: options.cwd,
    env: {
      ...process.env,
      PI_OBSERVE_STATE_DIR: dirs.state,
      PI_OBSERVE_CONFIG_DIR: dirs.config,
      LANGFUSE_PUBLIC_KEY: '',
      LANGFUSE_SECRET_KEY: '',
      ...options.env,
    },
  });
  return { res, ...dirs };
}

function events(state) {
  return readFileSync(join(state, 'events.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
}

function activeState(state) {
  return JSON.parse(readFileSync(join(state, 'runs.json'), 'utf8')).active;
}

test('PI_OBSERVE_ENABLED=false runs the command but writes no local telemetry state', () => {
  const { res, state } = run(['run', '--project', 'vire', '--', process.execPath, '-e', 'console.log("disabled ok")'], {
    env: { PI_OBSERVE_ENABLED: 'false' },
  });

  assert.equal(res.status, 0, res.stderr);
  assert.match(res.stdout, /disabled ok/);
  assert.equal(existsSync(join(state, 'events.jsonl')), false);
  assert.equal(existsSync(join(state, 'runs.json')), false);
});

test('Langfuse connection failures fail open and still record local finish status', () => {
  const { res, state } = run(['run', '--project', 'vire', '--tool', 'fail-open', '--', process.execPath, '-e', 'console.log("wrapped command ran")'], {
    env: {
      LANGFUSE_HOST: 'http://127.0.0.1:1',
      LANGFUSE_PUBLIC_KEY: 'pk-test',
      LANGFUSE_SECRET_KEY: 'sk-test-secret-value',
      PI_OBSERVE_LANGFUSE_TIMEOUT_MS: '50',
    },
  });

  assert.equal(res.status, 0, res.stderr);
  assert.match(res.stdout, /wrapped command ran/);
  assert.equal(events(state).find(e => e.event === 'tool_finished')?.status, 'success');
});

test('concurrent runs for one project do not start idle countdown until the last run exits', async () => {
  const dirs = tempState();
  const env = { ...process.env, PI_OBSERVE_STATE_DIR: dirs.state, PI_OBSERVE_CONFIG_DIR: dirs.config, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '' };
  const long = spawn(process.execPath, [cli, 'run', '--project', 'vire', '--tool', 'slow', '--', process.execPath, '-e', 'setTimeout(() => {}, 350)'], { env, stdio: 'ignore' });

  await new Promise(resolve => setTimeout(resolve, 80));
  const short = spawnSync(process.execPath, [cli, 'run', '--project', 'vire', '--tool', 'fast', '--', process.execPath, '-e', ''], { env, encoding: 'utf8' });
  assert.equal(short.status, 0, short.stderr);

  const afterShort = events(dirs.state);
  assert.equal(afterShort.filter(e => e.event === 'tool_finished').length, 1);
  assert.equal(afterShort.some(e => e.event === 'idle_countdown_started'), false, 'idle countdown started while another run was still active');
  assert.equal(activeState(dirs.state).vire.count, 1);

  await new Promise((resolve, reject) => {
    long.on('exit', code => code === 0 ? resolve() : reject(new Error(`long run exited ${code}`)));
    long.on('error', reject);
  });

  const finalEvents = events(dirs.state);
  assert.equal(finalEvents.filter(e => e.event === 'tool_finished').length, 2);
  assert.equal(finalEvents.filter(e => e.event === 'idle_countdown_started').length, 1);
  assert.equal(activeState(dirs.state).vire.count, 0);
});

test('manual mark-active/mark-inactive without an explicit session clears active run bookkeeping', () => {
  const dirs = tempState();
  const active = run(['mark-active', '--project', 'vire', '--tool', 'cursor', '--summary', 'manual GUI work'], { dirs });
  assert.equal(active.res.status, 0, active.res.stderr);
  const inactive = run(['mark-inactive', '--project', 'vire', '--tool', 'cursor'], { dirs });
  assert.equal(inactive.res.status, 0, inactive.res.stderr);

  const state = activeState(dirs.state).vire;
  assert.equal(state.count, 0);
  assert.deepEqual(state.runs, [], 'inactive manual marker should not leave stale active run ids');
  assert.equal(events(dirs.state).at(-1).event, 'idle_countdown_started');
});

test('manual inactive for another tool does not decrement a running delegate', async () => {
  const dirs = tempState();
  const env = { ...process.env, PI_OBSERVE_STATE_DIR: dirs.state, PI_OBSERVE_CONFIG_DIR: dirs.config, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '' };
  const long = spawn(process.execPath, [cli, 'run', '--project', 'vire', '--tool', 'delegate', '--', process.execPath, '-e', 'setTimeout(() => {}, 250)'], { env, stdio: 'ignore' });
  await new Promise(resolve => setTimeout(resolve, 80));
  const inactive = spawnSync(process.execPath, [cli, 'mark-inactive', '--project', 'vire', '--tool', 'cursor'], { env, encoding: 'utf8' });
  assert.equal(inactive.status, 0, inactive.stderr);
  assert.equal(activeState(dirs.state).vire.count, 1);
  assert.equal(events(dirs.state).some(e => e.event === 'idle_countdown_started'), false);
  await new Promise((resolve, reject) => {
    long.on('exit', code => code === 0 ? resolve() : reject(new Error(`long run exited ${code}`)));
    long.on('error', reject);
  });
  assert.equal(activeState(dirs.state).vire.count, 0);
});

test('concurrent runs for different projects keep independent state', async () => {
  const dirs = tempState();
  const env = { ...process.env, PI_OBSERVE_STATE_DIR: dirs.state, PI_OBSERVE_CONFIG_DIR: dirs.config, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '' };
  const a = spawn(process.execPath, [cli, 'run', '--project', 'vire', '--tool', 'slow', '--', process.execPath, '-e', 'setTimeout(() => {}, 250)'], { env, stdio: 'ignore' });
  await new Promise(resolve => setTimeout(resolve, 50));
  const b = spawnSync(process.execPath, [cli, 'run', '--project', 'other', '--tool', 'fast', '--', process.execPath, '-e', ''], { env, encoding: 'utf8' });
  assert.equal(b.status, 0, b.stderr);
  const mid = activeState(dirs.state);
  assert.equal(mid.vire.count, 1);
  assert.equal(mid.other.count, 0);
  await new Promise((resolve, reject) => {
    a.on('exit', code => code === 0 ? resolve() : reject(new Error(`project a exited ${code}`)));
    a.on('error', reject);
  });
  const final = activeState(dirs.state);
  assert.equal(final.vire.count, 0);
  assert.equal(final.other.count, 0);
});
