import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, existsSync, mkdirSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import http from 'node:http';

const cli = new URL('../observability/pi-observe/bin/pi-observe.mjs', import.meta.url).pathname;

function tempDirs() {
  const root = mkdtempSync(join(tmpdir(), 'pi-observe-sec-'));
  return { root, state: join(root, 'state'), config: join(root, 'config'), dotenv: join(root, '.env') };
}

function run(args, dirs, extraEnv = {}) {
  return spawnSync(process.execPath, [cli, ...args], {
    encoding: 'utf8',
    env: { ...process.env, PI_OBSERVE_STATE_DIR: dirs.state, PI_OBSERVE_CONFIG_DIR: dirs.config, PI_OBSERVE_DOTENV: dirs.dotenv, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '', ...extraEnv },
  });
}

function runAsync(args, dirs, extraEnv = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [cli, ...args], {
      encoding: 'utf8',
      env: { ...process.env, PI_OBSERVE_STATE_DIR: dirs.state, PI_OBSERVE_CONFIG_DIR: dirs.config, PI_OBSERVE_DOTENV: dirs.dotenv, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '', ...extraEnv },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '', stderr = '';
    child.stdout.on('data', d => stdout += d);
    child.stderr.on('data', d => stderr += d);
    child.on('error', reject);
    child.on('close', status => resolve({ status, stdout, stderr }));
  });
}

test('enabled wrapper preserves unrelated user runtime environment without injecting dotenv Langfuse secrets', () => {
  const dirs = tempDirs();
  writeFileSync(dirs.dotenv, 'LANGFUSE_PUBLIC_KEY=pk-from-dotenv\nLANGFUSE_SECRET_KEY=sk-from-dotenv\n');
  const script = 'if (process.env.DATABASE_URL !== "postgres://app-db") process.exit(42); if (process.env.LANGFUSE_SECRET_KEY || process.env.LANGFUSE_PUBLIC_KEY) process.exit(43); console.log("env preserved")';
  const res = run(['run', '--project', 'vire', '--', process.execPath, '-e', script], dirs, {
    DATABASE_URL: 'postgres://app-db',
    REDIS_CONNECTION_STRING: 'redis://app-redis',
  });
  assert.equal(res.status, 0, res.stderr);
  assert.match(res.stdout, /env preserved/);
});

test('safe dotenv parser loads only allowlisted Langfuse keys without shell execution', async () => {
  const dirs = tempDirs();
  let requests = 0;
  const server = http.createServer((req, res) => {
    requests += 1;
    assert.equal(req.url, '/api/public/ingestion');
    assert.match(req.headers.authorization || '', /^Basic /);
    res.writeHead(207, { 'content-type': 'application/json' });
    res.end('{}');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  writeFileSync(dirs.dotenv, `LANGFUSE_HOST=http://127.0.0.1:${port}\nLANGFUSE_PUBLIC_KEY=pk-local\nLANGFUSE_SECRET_KEY=sk-local\nPOSTGRES_PASSWORD=should-not-load\nLANGFUSE_INIT_USER_PASSWORD=$(touch ${join(dirs.root, 'pwned')})\n`);
  const res = await runAsync(['run', '--project', 'vire', '--tool', 'dotenv-test', '--', process.execPath, '-e', 'console.log("dotenv ok")'], dirs);
  server.close();
  assert.equal(res.status, 0, res.stderr);
  assert.equal(existsSync(join(dirs.root, 'pwned')), false, 'dotenv parser must not execute shell syntax');
  assert.ok(requests >= 1, 'expected at least one Langfuse ingestion request using dotenv keys');
});

test('Langfuse 2xx body with ingestion errors warns and fails smoke-ingest', async () => {
  const dirs = tempDirs();
  const server = http.createServer((_req, res) => { res.writeHead(207, { 'content-type': 'application/json' }); res.end(JSON.stringify({ successes: 0, errors: [{ message: 'invalid event' }] })); });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  writeFileSync(dirs.dotenv, `LANGFUSE_HOST=http://127.0.0.1:${server.address().port}\nLANGFUSE_PUBLIC_KEY=pk-local\nLANGFUSE_SECRET_KEY=sk-local\n`);
  const res = await runAsync(['smoke-ingest', '--project', 'vire'], dirs);
  server.close();
  assert.equal(res.status, 1);
  assert.match(res.stderr, /body-errors|not accepted|telemetry rejected/);
  assert.doesNotMatch(res.stderr, /sk-local|pk-local/);
});

test('remote Langfuse host is blocked unless explicitly opted in', async () => {
  const dirs = tempDirs();
  let requests = 0;
  const server = http.createServer((_req, res) => { requests += 1; res.writeHead(200); res.end('{}'); });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  writeFileSync(dirs.dotenv, `LANGFUSE_HOST=http://example.com:${server.address().port}\nLANGFUSE_PUBLIC_KEY=pk-local\nLANGFUSE_SECRET_KEY=sk-local\n`);
  const res = await runAsync(['run', '--project', 'vire', '--tool', 'remote-block', '--', process.execPath, '-e', 'console.log("ran")'], dirs);
  server.close();
  assert.equal(res.status, 0);
  assert.match(res.stdout, /ran/);
  assert.match(res.stderr, /remote Langfuse host/);
  assert.equal(requests, 0);
  assert.doesNotMatch(res.stderr, /sk-local|pk-local/);
});

test('session ids are hashed before local storage', () => {
  const dirs = tempDirs();
  const res = run(['run', '--project', 'vire', '--session', 'client@example.com secret-session', '--', process.execPath, '-e', ''], dirs);
  assert.equal(res.status, 0, res.stderr);
  const raw = readFileSync(join(dirs.state, 'events.jsonl'), 'utf8');
  assert.doesNotMatch(raw, /client@example\.com|secret-session/);
  assert.match(raw, /session-[a-f0-9]{24}/);
});

test('Langfuse HTTP rejection warns without blocking wrapped command', async () => {
  const dirs = tempDirs();
  const server = http.createServer((_req, res) => { res.writeHead(401); res.end('nope'); });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  writeFileSync(dirs.dotenv, `LANGFUSE_HOST=http://127.0.0.1:${server.address().port}\nLANGFUSE_PUBLIC_KEY=pk-local\nLANGFUSE_SECRET_KEY=sk-local\n`);
  const res = await runAsync(['run', '--project', 'vire', '--tool', 'reject-test', '--', process.execPath, '-e', 'console.log("still ran")'], dirs);
  server.close();
  assert.equal(res.status, 0);
  assert.match(res.stdout, /still ran/);
  assert.match(res.stderr, /telemetry rejected/);
  assert.doesNotMatch(res.stderr, /sk-local|pk-local/);
});

test('project resolution accepts only whole-marker safe project keys', () => {
  const dirs = tempDirs();
  const cwd = join(dirs.root, 'Safe Project');
  mkdirSync(cwd, { recursive: true });
  writeFileSync(join(cwd, '.pi-project'), '  Client_1.2-key\n');
  const res = spawnSync(process.execPath, [cli, 'run', '--tool', 'proj', '--', process.execPath, '-e', ''], {
    cwd,
    encoding: 'utf8',
    env: { ...process.env, PI_OBSERVE_STATE_DIR: dirs.state, PI_OBSERVE_CONFIG_DIR: dirs.config, PI_OBSERVE_DOTENV: dirs.dotenv, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '' },
  });
  assert.equal(res.status, 0, res.stderr);
  const raw = readFileSync(join(dirs.state, 'events.jsonl'), 'utf8');
  assert.match(raw, /"project":"client_1.2-key"/);
});

test('invalid project marker contents are ignored instead of becoming unknown or redacted placeholders', () => {
  const dirs = tempDirs();
  const symbolCwd = join(dirs.root, 'symbol-marker');
  const secretCwd = join(dirs.root, 'secret-marker');
  const mixedCwd = join(dirs.root, 'mixed-marker');
  const mappedCwd = join(dirs.root, 'mapped-marker');
  mkdirSync(symbolCwd, { recursive: true });
  mkdirSync(secretCwd, { recursive: true });
  mkdirSync(mixedCwd, { recursive: true });
  mkdirSync(mappedCwd, { recursive: true });
  mkdirSync(dirs.config, { recursive: true });
  writeFileSync(join(symbolCwd, '.pi-project'), '!!!\n');
  writeFileSync(join(secretCwd, '.pi-project'), 'ghp_abcdefghijklmnopqrstuvwxyz1234567890\n');
  writeFileSync(join(mixedCwd, '.pi-project'), 'vire extra-data user@example.com\n');
  writeFileSync(join(mappedCwd, '.pi-project'), '!!!\n');
  writeFileSync(join(dirs.config, 'projects.json'), JSON.stringify({ projects: { configured: { paths: [mappedCwd] } } }));

  for (const [cwd, stateName, expected] of [[symbolCwd, 'symbol-state', 'symbol-marker'], [secretCwd, 'secret-state', 'secret-marker'], [mixedCwd, 'mixed-state', 'mixed-marker'], [mappedCwd, 'mapped-state', 'configured']]) {
    const state = join(dirs.root, stateName);
    const res = spawnSync(process.execPath, [cli, 'run', '--tool', 'marker-content', '--', process.execPath, '-e', ''], {
      cwd,
      encoding: 'utf8',
      env: { ...process.env, PI_OBSERVE_STATE_DIR: state, PI_OBSERVE_CONFIG_DIR: dirs.config, PI_OBSERVE_DOTENV: dirs.dotenv, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '' },
    });
    assert.equal(res.status, 0, res.stderr);
    const raw = readFileSync(join(state, 'events.jsonl'), 'utf8');
    assert.match(raw, new RegExp(`"project":"${expected}"`));
    assert.doesNotMatch(raw, /"project":"unknown"|"project":"vire"|redacted_github_token|ghp_|user@example\.com/);
  }
});

test('project marker symlinks and oversized/non-file markers are ignored safely', () => {
  const dirs = tempDirs();
  const symlinkCwd = join(dirs.root, 'symlink-marker');
  const oversizedCwd = join(dirs.root, 'oversized-marker');
  const directoryCwd = join(dirs.root, 'directory-marker');
  mkdirSync(symlinkCwd, { recursive: true });
  mkdirSync(oversizedCwd, { recursive: true });
  mkdirSync(directoryCwd, { recursive: true });
  const markerTarget = join(dirs.root, 'marker-target');
  writeFileSync(markerTarget, 'vire\n');
  symlinkSync(markerTarget, join(symlinkCwd, '.pi-project'));
  writeFileSync(join(oversizedCwd, '.pi-project'), `vire${'x'.repeat(300)}\n`);
  mkdirSync(join(directoryCwd, '.pi-project'));

  for (const [cwd, expected] of [[symlinkCwd, 'symlink-marker'], [oversizedCwd, 'oversized-marker'], [directoryCwd, 'directory-marker']]) {
    const state = join(dirs.root, `${expected}-state`);
    const res = spawnSync(process.execPath, [cli, 'run', '--tool', 'marker-safety', '--', process.execPath, '-e', ''], {
      cwd,
      encoding: 'utf8',
      env: { ...process.env, PI_OBSERVE_STATE_DIR: state, PI_OBSERVE_CONFIG_DIR: dirs.config, PI_OBSERVE_DOTENV: dirs.dotenv, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '' },
    });
    assert.equal(res.status, 0, res.stderr);
    const raw = readFileSync(join(state, 'events.jsonl'), 'utf8');
    assert.doesNotMatch(raw, /"project":"vire"/);
    assert.match(raw, new RegExp(`"project":"${expected}"`));
  }
});

test('path project mapping matches only real directory boundaries, not same-prefix siblings', () => {
  const dirs = tempDirs();
  const mapped = join(dirs.root, 'client');
  const child = join(mapped, 'nested');
  const sibling = join(dirs.root, 'client-other');
  mkdirSync(child, { recursive: true });
  mkdirSync(sibling, { recursive: true });
  mkdirSync(dirs.config, { recursive: true });
  writeFileSync(join(dirs.config, 'projects.json'), JSON.stringify({ projects: { vire: { paths: [mapped] } } }));

  const baseEnv = { ...process.env, PI_OBSERVE_CONFIG_DIR: dirs.config, PI_OBSERVE_DOTENV: dirs.dotenv, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '' };
  const inside = spawnSync(process.execPath, [cli, 'run', '--tool', 'path-map', '--', process.execPath, '-e', ''], {
    cwd: child,
    encoding: 'utf8',
    env: { ...baseEnv, PI_OBSERVE_STATE_DIR: join(dirs.root, 'inside-state') },
  });
  assert.equal(inside.status, 0, inside.stderr);
  assert.match(readFileSync(join(dirs.root, 'inside-state/events.jsonl'), 'utf8'), /"project":"vire"/);

  const outside = spawnSync(process.execPath, [cli, 'run', '--tool', 'path-map', '--', process.execPath, '-e', ''], {
    cwd: sibling,
    encoding: 'utf8',
    env: { ...baseEnv, PI_OBSERVE_STATE_DIR: join(dirs.root, 'outside-state') },
  });
  assert.equal(outside.status, 0, outside.stderr);
  const outsideRaw = readFileSync(join(dirs.root, 'outside-state/events.jsonl'), 'utf8');
  assert.doesNotMatch(outsideRaw, /"project":"vire"/);
  assert.match(outsideRaw, /"project":"client-other"/);
});
