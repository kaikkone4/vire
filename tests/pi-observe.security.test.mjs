import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs';
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

test('child command receives scrubbed observability secret environment', () => {
  const dirs = tempDirs();
  const script = 'for (const k of ["LANGFUSE_SECRET_KEY","POSTGRES_PASSWORD","REDIS_PASSWORD","MINIO_ROOT_PASSWORD","NEXTAUTH_SECRET"]) if (process.env[k]) process.exit(9); console.log("scrubbed")';
  const res = run(['run', '--project', 'vire', '--', process.execPath, '-e', script], dirs, {
    LANGFUSE_SECRET_KEY: 'dummy-secret',
    POSTGRES_PASSWORD: 'dummy-postgres',
    REDIS_PASSWORD: 'dummy-redis',
    MINIO_ROOT_PASSWORD: 'dummy-minio',
    NEXTAUTH_SECRET: 'dummy-nextauth',
  });
  assert.equal(res.status, 0, res.stderr);
  assert.match(res.stdout, /scrubbed/);
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

test('project resolution uses env, project file, config map, then fallback with sanitization', () => {
  const dirs = tempDirs();
  const cwd = join(dirs.root, 'Unsafe Project');
  mkdirSync(cwd, { recursive: true });
  writeFileSync(join(cwd, '.pi-project'), 'Client Email user@example.com $$$');
  const res = spawnSync(process.execPath, [cli, 'run', '--tool', 'proj', '--', process.execPath, '-e', ''], {
    cwd,
    encoding: 'utf8',
    env: { ...process.env, PI_OBSERVE_STATE_DIR: dirs.state, PI_OBSERVE_CONFIG_DIR: dirs.config, PI_OBSERVE_DOTENV: dirs.dotenv, LANGFUSE_PUBLIC_KEY: '', LANGFUSE_SECRET_KEY: '' },
  });
  assert.equal(res.status, 0, res.stderr);
  const raw = readFileSync(join(dirs.state, 'events.jsonl'), 'utf8');
  assert.match(raw, /client/);
  assert.doesNotMatch(raw, /user@example\.com/);
});
