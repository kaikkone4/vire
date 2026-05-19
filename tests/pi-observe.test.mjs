import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const cli = new URL('../observability/pi-observe/bin/pi-observe.mjs', import.meta.url).pathname;

function run(args, env = {}) {
  const state = mkdtempSync(join(tmpdir(), 'pi-observe-'));
  const res = spawnSync(process.execPath, [cli, ...args], { encoding: 'utf8', env: { ...process.env, PI_OBSERVE_STATE_DIR: state, PI_OBSERVE_CONFIG_DIR: join(state, 'config'), LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '', ...env } });
  return { res, state };
}

test('wrapped success records start, finish, and idle countdown locally', () => {
  const { res, state } = run(['run', '--project', 'vire', '--tool', 'test', '--role', 'unit', '--', process.execPath, '-e', 'console.log("ok")']);
  assert.equal(res.status, 0, res.stderr);
  assert.match(res.stdout, /ok/);
  const events = readFileSync(join(state, 'events.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
  assert.equal(events[0].event, 'tool_started');
  assert.equal(events[0].project, 'vire');
  assert.equal(events[1].event, 'tool_finished');
  assert.equal(events[1].status, 'success');
  assert.equal(events[2].event, 'idle_countdown_started');
});

test('wrapped failure propagates exit code and records failed status', () => {
  const { res, state } = run(['run', '--project', 'vire', '--', process.execPath, '-e', 'process.exit(7)']);
  assert.equal(res.status, 7);
  const events = readFileSync(join(state, 'events.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
  assert.equal(events.find(e => e.event === 'tool_finished').status, 'failed');
  assert.equal(events.find(e => e.event === 'tool_finished').exit_code, 7);
});

test('summary redaction removes common token shapes', () => {
  const { state } = run(['run', '--project', 'vire', '--summary', 'token ghp_abcdefghijklmnopqrstuvwxyz123456 and sk-proj-abcdefghijklmnop', '--', process.execPath, '-e', '']);
  const raw = readFileSync(join(state, 'events.jsonl'), 'utf8');
  assert.doesNotMatch(raw, /ghp_/);
  assert.doesNotMatch(raw, /sk-proj-/);
  assert.match(raw, /REDACTED/);
});
