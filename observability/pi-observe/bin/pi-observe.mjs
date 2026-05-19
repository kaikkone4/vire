#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync, rmdirSync, realpathSync } from 'node:fs';
import { basename, dirname, join, resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { randomUUID, createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const VERSION = '0.1.2';
const now = () => new Date().toISOString();
const home = process.env.HOME || process.cwd();
const scriptDir = dirname(realpathSync(fileURLToPath(import.meta.url)));
const repoRoot = resolve(scriptDir, '../../..');
const defaultDotenvPath = join(repoRoot, 'observability/langfuse/.env');
const stateDir = expand(process.env.PI_OBSERVE_STATE_DIR || join(home, '.local/state/pi-observe'));
const configDir = expand(process.env.PI_OBSERVE_CONFIG_DIR || join(home, '.config/pi-observe'));
const eventsPath = join(stateDir, 'events.jsonl');
const runsPath = join(stateDir, 'runs.json');
const ALLOWED_DOTENV_KEYS = new Set(['LANGFUSE_HOST', 'LANGFUSE_PUBLIC_KEY', 'LANGFUSE_SECRET_KEY', 'LANGFUSE_PROJECT_ID', 'PI_OBSERVE_LANGFUSE_TIMEOUT_MS', 'PI_OBSERVE_USER_ID', 'PI_OBSERVE_ALLOW_REMOTE_LANGFUSE']);
const SCRUB_ENV_KEYS = new Set(['LANGFUSE_HOST', 'LANGFUSE_PUBLIC_KEY', 'LANGFUSE_SECRET_KEY', 'LANGFUSE_PROJECT_ID', 'NEXTAUTH_SECRET', 'SALT', 'ENCRYPTION_KEY', 'LANGFUSE_INIT_USER_PASSWORD', 'POSTGRES_PASSWORD', 'CLICKHOUSE_PASSWORD', 'REDIS_PASSWORD', 'MINIO_ROOT_PASSWORD', 'DATABASE_URL', 'DIRECT_URL', 'REDIS_CONNECTION_STRING']);

function expand(p) { return p.replace(/^~(?=$|\/)/, home); }
function ensureDirs() { mkdirSync(stateDir, { recursive: true, mode: 0o700 }); mkdirSync(configDir, { recursive: true, mode: 0o700 }); }
function hash(s, len = 16) { return createHash('sha256').update(String(s)).digest('hex').slice(0, len); }
function safeToken(value, fallback = 'unknown') {
  const raw = redact(String(value || '')).trim().toLowerCase().replace(/[^a-z0-9._-]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 80);
  return raw || fallback;
}
function safeLabel(value) { return safeToken(basename(String(value || 'command')), 'command'); }
function safeSessionId(value) { return value ? `session-${hash(redact(value), 24)}` : randomUUID(); }
function redact(value) {
  if (value == null) return value;
  let s = String(value);
  s = s.replace(/github_pat_[A-Za-z0-9_]+|gh[pousr]_[A-Za-z0-9_]+/g, '[REDACTED_GITHUB_TOKEN]');
  s = s.replace(/sk-(?:ant-|proj-)?[A-Za-z0-9_-]{12,}/g, '[REDACTED_API_KEY]');
  s = s.replace(/xox[abprs]-[A-Za-z0-9-]+/g, '[REDACTED_SLACK_TOKEN]');
  s = s.replace(/AKIA[A-Z0-9]{16}/g, '[REDACTED_AWS_KEY]');
  s = s.replace(/-----BEGIN [^-]*PRIVATE KEY-----[\s\S]*?-----END [^-]*PRIVATE KEY-----/g, '[REDACTED_PRIVATE_KEY]');
  s = s.replace(/https?:\/\/([^\s/@:]+):([^\s/@]+)@/g, 'https://[REDACTED_CREDENTIALS]@');
  s = s.replace(/[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}/gi, '[REDACTED_EMAIL]');
  s = s.replace(/(^|\n)\s*(?:[A-Z0-9_]*(?:TOKEN|SECRET|PASSWORD|KEY)[A-Z0-9_]*)\s*=\s*[^\n]+/gi, '$1[REDACTED_ENV_LINE]');
  s = s.replace(/\b[A-Za-z0-9+/=]{48,}\b/g, '[REDACTED_HIGH_ENTROPY]');
  return s;
}
function sanitizeObject(obj) {
  if (Array.isArray(obj)) return obj.map(sanitizeObject);
  if (obj && typeof obj === 'object') return Object.fromEntries(Object.entries(obj).map(([k,v]) => [safeToken(k, 'key'), sanitizeObject(v)]));
  return typeof obj === 'string' ? redact(obj) : obj;
}
function parseDotenvFile(path = process.env.PI_OBSERVE_DOTENV || defaultDotenvPath) {
  const out = {};
  if (!path || !existsSync(path)) return out;
  let lines;
  try { lines = readFileSync(path, 'utf8').split(/\r?\n/); } catch { return out; }
  for (const line of lines) {
    if (!line.trim() || line.trimStart().startsWith('#')) continue;
    const m = line.match(/^([A-Z0-9_]+)=(.*)$/);
    if (!m || !ALLOWED_DOTENV_KEYS.has(m[1])) continue;
    let value = m[2].trim();
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) value = value.slice(1, -1);
    // Data-only parser: no shell evaluation, interpolation, exports, or command substitution.
    if (/[`$;]/.test(value)) continue;
    out[m[1]] = value;
  }
  return out;
}
const dotenvConfig = parseDotenvFile();
function cfg(key, fallback = undefined) { return process.env[key] !== undefined && process.env[key] !== '' ? process.env[key] : (dotenvConfig[key] !== undefined && dotenvConfig[key] !== '' ? dotenvConfig[key] : fallback); }
function scrubbedEnv() {
  const env = { ...process.env };
  for (const key of Object.keys(env)) {
    if (SCRUB_ENV_KEYS.has(key) || key.startsWith('LANGFUSE_S3_')) delete env[key];
  }
  return env;
}
function parseArgs(argv) {
  const opts = { command: [], billable: true };
  let i = 0;
  for (; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--') { opts.command = argv.slice(i + 1); break; }
    if (a === '--project') opts.project = argv[++i];
    else if (a === '--tool') opts.tool = argv[++i];
    else if (a === '--role') opts.role = argv[++i];
    else if (a === '--session') opts.session = argv[++i];
    else if (a === '--summary') opts.summary = argv[++i];
    else if (a === '--nonbillable') opts.billable = false;
    else if (a === '--label') opts.label = argv[++i];
    else throw new Error(`Unknown option: ${a}`);
  }
  return opts;
}
function resolveProject(explicit) {
  if (explicit) return { key: safeToken(explicit), confidence: 'explicit' };
  if (process.env.PI_OBSERVE_PROJECT) return { key: safeToken(process.env.PI_OBSERVE_PROJECT), confidence: 'env' };
  for (const name of ['.pi-project', '.vire-project']) {
    let dir = process.cwd();
    while (true) {
      const p = join(dir, name);
      if (existsSync(p)) return { key: safeToken(readFileSync(p, 'utf8').trim().split(/\s+/)[0]), confidence: name };
      const parent = dirname(dir); if (parent === dir) break; dir = parent;
    }
  }
  try {
    const cfgFile = JSON.parse(readFileSync(join(configDir, 'projects.json'), 'utf8'));
    const cwd = process.cwd(); const remote = safeGitRemoteHash();
    for (const [key, rule] of Object.entries(cfgFile.projects || {})) {
      if (rule.paths?.some(p => pathContains(expand(p), cwd))) return { key: safeToken(key), confidence: 'path-map' };
      if (remote && rule.git_remote_hashes?.includes(remote)) return { key: safeToken(key), confidence: 'git-remote-hash' };
    }
  } catch {}
  return { key: safeToken(basename(process.cwd())), confidence: 'low' };
}
function canonicalPath(p) { try { return realpathSync(p); } catch { return resolve(p); } }
function pathContains(parent, child) {
  const base = canonicalPath(parent);
  const target = canonicalPath(child);
  const rel = relative(base, target);
  return rel === '' || (!!rel && !rel.startsWith('..') && !rel.startsWith('/') && !rel.startsWith('\\') && !resolve(rel).startsWith('/..'));
}
function safeGitBranchHash() { try { const b = execFileSync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { encoding:'utf8', timeout: 500, stdio:['ignore','pipe','ignore'] }).trim(); return process.env.PI_OBSERVE_CAPTURE_GIT_BRANCH === 'true' ? safeToken(redact(b), 'branch') : hash(b); } catch { return undefined; } }
function safeGitRemoteHash() { try { const r = execFileSync('git', ['config', '--get', 'remote.origin.url'], { encoding:'utf8', timeout: 500, stdio:['ignore','pipe','ignore'] }).trim(); return hash(r); } catch { return undefined; } }
function appendEvent(e) { ensureDirs(); appendFileSync(eventsPath, JSON.stringify(sanitizeObject(e)) + '\n', { mode: 0o600 }); }
function readRuns() { try { return JSON.parse(readFileSync(runsPath, 'utf8')); } catch { return { active: {} }; } }
function writeRuns(r) { ensureDirs(); writeFileSync(runsPath, JSON.stringify(r, null, 2), { mode: 0o600 }); }
function withStateLock(fn) {
  ensureDirs(); const lock = join(stateDir, '.runs.lock'); const deadline = Date.now() + 5000;
  while (true) {
    try { mkdirSync(lock); break; } catch { if (Date.now() > deadline) throw new Error('Timed out waiting for pi-observe state lock'); Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 25); }
  }
  try { return fn(); } finally { try { rmdirSync(lock); } catch {} }
}
function initEntry(state, project) { state.active[project] ||= { count: 0, runs: [], run_started_at: {}, idle_countdown_started_at: null, idle_started_at: null }; state.active[project].runs ||= []; state.active[project].run_started_at ||= {}; return state.active[project]; }
function addActive(project, runId) {
  return withStateLock(() => {
    const state = readRuns(); const entry = initEntry(state, project); const prev = entry.runs.length;
    if (!entry.runs.includes(runId)) entry.runs.push(runId);
    entry.run_started_at[runId] ||= now();
    entry.count = entry.runs.length; const canceledIdle = prev === 0 && entry.idle_countdown_started_at;
    entry.idle_countdown_started_at = null; entry.idle_started_at = null; entry.updated_at = now(); writeRuns(state);
    return { count: entry.count, previousCount: prev, canceledIdle };
  });
}
function removeActive(project, runId) {
  return withStateLock(() => {
    const state = readRuns(); const entry = initEntry(state, project); const before = entry.runs.length;
    if (runId) { entry.runs = entry.runs.filter(r => r !== runId); delete entry.run_started_at[runId]; } else { entry.runs = []; entry.run_started_at = {}; }
    entry.count = entry.runs.length; const removed = before !== entry.runs.length;
    if (entry.count === 0 && removed) entry.idle_countdown_started_at = now();
    entry.updated_at = now(); writeRuns(state); return { count: entry.count, removed, idleStartedAt: entry.idle_countdown_started_at };
  });
}
function reconcileState() {
  const threshold = Number(process.env.PI_OBSERVE_IDLE_THRESHOLD_MS || 300000);
  const orphanTimeout = Number(process.env.PI_OBSERVE_ORPHAN_TIMEOUT_MS || 21600000);
  const ts = Date.now(); const generated = [];
  const result = withStateLock(() => {
    const state = readRuns();
    for (const [project, entryRaw] of Object.entries(state.active || {})) {
      const entry = initEntry(state, project);
      const orphaned = [];
      for (const runId of [...entry.runs]) {
        const started = Date.parse(entry.run_started_at[runId] || entry.updated_at || now());
        if (Number.isFinite(started) && ts - started >= orphanTimeout) orphaned.push(runId);
      }
      for (const runId of orphaned) {
        entry.runs = entry.runs.filter(r => r !== runId); delete entry.run_started_at[runId];
        generated.push({ event:'tool_orphaned', project, run_id: runId, timeout_ms: orphanTimeout, ts: now() });
      }
      const beforeCount = entry.count || 0; entry.count = entry.runs.length;
      if (beforeCount > 0 && entry.count === 0 && orphaned.length) {
        entry.idle_countdown_started_at = now(); entry.idle_started_at = null;
        generated.push({ event:'idle_countdown_started', project, after_run_id: orphaned.at(-1), threshold_ms: threshold, ts: entry.idle_countdown_started_at });
      }
      if (entry.count === 0 && entry.idle_countdown_started_at && !entry.idle_started_at) {
        const idleStart = Date.parse(entry.idle_countdown_started_at);
        if (Number.isFinite(idleStart) && ts - idleStart >= threshold) {
          entry.idle_started_at = now(); generated.push({ event:'idle_started', project, threshold_ms: threshold, ts: entry.idle_started_at });
        }
      }
      entry.updated_at = now();
    }
    writeRuns(state); return state.active || {};
  });
  for (const event of generated) appendEvent(event);
  return { active: result, events: generated };
}
function isLoopbackLangfuseHost(host) {
  try {
    const u = new URL(host);
    const h = u.hostname.toLowerCase().replace(/^\[|\]$/g, '');
    return h === 'localhost' || h === '127.0.0.1' || h === '::1' || h === '0:0:0:0:0:0:0:1';
  } catch { return false; }
}
function safeHostLabel(host) { try { const u = new URL(host); return `${u.protocol}//${safeToken(u.hostname, 'host')}${u.port ? ':' + u.port : ''}`; } catch { return 'invalid-host'; } }
function inspectIngestionBody(text) {
  if (!text || !text.trim()) return { ok: true, body: null };
  try {
    const body = JSON.parse(text);
    const haystack = JSON.stringify(body).toLowerCase();
    const errorValue = body.errors ?? body.error ?? body.failures ?? body.failed ?? body.rejected ?? body.rejections;
    const errorCountValue = body.errorCount ?? body.error_count ?? body.failureCount ?? body.failedCount ?? body.rejectedCount;
    const errorCount = Array.isArray(errorValue) ? errorValue.length : (typeof errorValue === 'object' && errorValue ? Object.keys(errorValue).length : Number(errorValue ?? errorCountValue ?? 0));
    const successValue = body.successes ?? body.successful ?? body.succeeded;
    const successCount = Array.isArray(successValue) ? successValue.length : Number(successValue ?? body.successCount ?? body.success_count ?? NaN);
    const hasExplicitErrorText = /\b(rejected|failed|invalid)\b/.test(haystack);
    if ((Number.isFinite(errorCount) && errorCount > 0) || (Number.isFinite(successCount) && successCount === 0 && hasExplicitErrorText) || hasExplicitErrorText) return { ok: false, reason: 'body-errors', body };
    return { ok: true, body };
  } catch {
    return { ok: true, body: null };
  }
}
async function sendLangfuse(batch, { warn = true } = {}) {
  if (cfg('PI_OBSERVE_ENABLED') === 'false') return { attempted: false, ok: true };
  const host = cfg('LANGFUSE_HOST', 'http://localhost:3000');
  const pub = cfg('LANGFUSE_PUBLIC_KEY'), sec = cfg('LANGFUSE_SECRET_KEY');
  if (!pub || !sec) return { attempted: false, ok: false, reason: 'missing-keys' };
  if (!isLoopbackLangfuseHost(host) && cfg('PI_OBSERVE_ALLOW_REMOTE_LANGFUSE') !== 'true') {
    if (warn) console.warn(`[pi-observe] remote Langfuse host ${safeHostLabel(host)} blocked by default; set PI_OBSERVE_ALLOW_REMOTE_LANGFUSE=true to opt in`);
    return { attempted: false, ok: false, reason: 'remote-host-blocked' };
  }
  if (!isLoopbackLangfuseHost(host) && warn) console.warn(`[pi-observe] remote Langfuse telemetry explicitly enabled for ${safeHostLabel(host)}`);
  const ac = new AbortController(); const t = setTimeout(() => ac.abort(), Number(cfg('PI_OBSERVE_LANGFUSE_TIMEOUT_MS', 400)));
  try {
    const res = await fetch(`${host.replace(/\/$/,'')}/api/public/ingestion`, { method:'POST', signal: ac.signal, headers:{ 'content-type':'application/json', authorization: `Basic ${Buffer.from(`${pub}:${sec}`).toString('base64')}` }, body: JSON.stringify({ batch }) });
    const text = await res.text();
    const bodyCheck = inspectIngestionBody(text);
    if (!res.ok || !bodyCheck.ok) {
      if (warn) console.warn(`[pi-observe] telemetry rejected by Langfuse (${res.status}${bodyCheck.reason ? `/${bodyCheck.reason}` : ''}); command/result preserved locally`);
      return { attempted: true, ok: false, status: res.status, reason: bodyCheck.reason || 'http-rejected' };
    }
    return { attempted: true, ok: true, status: res.status, body: bodyCheck.body };
  } catch (e) {
    if (warn) console.warn(`[pi-observe] telemetry unavailable; command/result preserved locally (${redact(e.name || 'error')})`);
    return { attempted: true, ok: false, reason: e.name || 'error' };
  } finally { clearTimeout(t); }
}
async function run(argv) {
  const opts = parseArgs(argv); if (!opts.command.length) throw new Error('Missing command after --');
  if (process.env.PI_OBSERVE_ENABLED === 'false') return spawnPassthrough(opts.command);
  ensureDirs(); const project = resolveProject(opts.project); const runId = randomUUID(); const traceId = randomUUID().replaceAll('-',''); const sessionId = safeSessionId(opts.session || process.env.PI_OBSERVE_SESSION); const start = Date.now();
  const meta = { wrapper_version: VERSION, project_key: project.key, project_confidence: project.confidence, tool: safeToken(opts.tool || 'command'), role: opts.role ? safeToken(opts.role) : undefined, cwd_basename: safeToken(basename(process.cwd()), 'cwd'), git_branch: safeGitBranchHash(), git_remote_hash: safeGitRemoteHash(), command_label: safeLabel(opts.label || opts.command[0]), billable: opts.billable, summary: opts.summary ? redact(opts.summary) : undefined };
  appendEvent({ event:'tool_started', project:project.key, tool:meta.tool, role:meta.role, run_id:runId, session_id: sessionId, ts: now(), billable: opts.billable, metadata: meta });
  if (opts.billable) { const added = addActive(project.key, runId); if (added.canceledIdle) appendEvent({ event:'idle_countdown_canceled', project:project.key, by_run_id:runId, ts:now() }); }
  await sendLangfuse([{ id: randomUUID(), timestamp: now(), type:'trace-create', body:{ id: traceId, name: `${meta.tool}${meta.role ? '.'+meta.role : ''}`, userId: safeToken(cfg('PI_OBSERVE_USER_ID', 'local-janne')), sessionId, tags:['local', meta.tool, meta.role, project.key].filter(Boolean), metadata: sanitizeObject(meta) } }]);
  const code = await spawnPassthrough(opts.command);
  const end = Date.now(); const status = code === 0 ? 'success' : code === 130 ? 'canceled' : 'failed';
  appendEvent({ event:'tool_finished', project:project.key, run_id:runId, status, exit_code: code, duration_ms:end-start, ts:now() });
  let removed = opts.billable ? removeActive(project.key, runId) : { count: 0, removed: false };
  if (opts.billable && removed.removed && removed.count === 0) appendEvent({ event:'idle_countdown_started', project:project.key, after_run_id:runId, threshold_ms:Number(process.env.PI_OBSERVE_IDLE_THRESHOLD_MS || 300000), ts:removed.idleStartedAt || now() });
  await sendLangfuse([{ id: randomUUID(), timestamp: now(), type:'trace-update', body:{ id: traceId, tags:['local', meta.tool, meta.role, project.key, status].filter(Boolean), metadata: sanitizeObject({ ...meta, status, exit_code: code, duration_ms:end-start, end_time: now() }) } }]);
  process.exitCode = code;
}
function spawnPassthrough(cmd) { return new Promise(resolve => { const child = spawn(cmd[0], cmd.slice(1), { stdio:'inherit', env: scrubbedEnv() }); child.on('error', e => { console.error(`[pi-observe] failed to start command: ${redact(e.message)}`); resolve(127); }); child.on('close', (code, signal) => resolve(code ?? (signal === 'SIGINT' ? 130 : 1))); }); }
function manualRunId(project, opts) { return opts.session ? `manual:${project}:${safeSessionId(opts.session)}` : `manual:${project}:${safeToken(opts.tool || 'manual')}:${safeToken(opts.role || 'default')}`; }
function mark(active, argv) {
  const opts = parseArgs(argv); const project = resolveProject(opts.project); const runId = manualRunId(project.key, opts); const tool = safeToken(opts.tool || 'manual'); const role = opts.role ? safeToken(opts.role) : undefined;
  appendEvent({ event: active ? 'manual_active' : 'manual_inactive', project: project.key, tool, role, run_id: runId, summary: opts.summary ? redact(opts.summary) : undefined, ts: now() });
  if (active) { const added = addActive(project.key, runId); if (added.canceledIdle) appendEvent({ event:'idle_countdown_canceled', project: project.key, by_run_id: runId, ts: now() }); }
  else { const removed = removeActive(project.key, runId); if (removed.removed && removed.count === 0) appendEvent({ event:'idle_countdown_started', project: project.key, after_run_id: runId, threshold_ms:Number(process.env.PI_OBSERVE_IDLE_THRESHOLD_MS || 300000), ts: removed.idleStartedAt || now() }); }
}
function status() { const reconciled = reconcileState(); console.log(JSON.stringify({ state_dir: stateDir, events_path: eventsPath, active: reconciled.active, reconciled_events: reconciled.events.length }, null, 2)); }
function reconcileCommand() { const reconciled = reconcileState(); console.log(JSON.stringify({ active: reconciled.active, events: reconciled.events }, null, 2)); }
async function smokeIngest(argv) { const opts = parseArgs(argv); const project = resolveProject(opts.project); const id = randomUUID().replaceAll('-',''); const result = await sendLangfuse([{ id: randomUUID(), timestamp: now(), type:'trace-create', body:{ id, name:'pi-observe.smoke-ingest', userId: safeToken(cfg('PI_OBSERVE_USER_ID', 'local-janne')), tags:['local','smoke-test',project.key], metadata:{ wrapper_version: VERSION, project_key: project.key, smoke_test: true } } }], { warn: true }); if (!result.attempted) { console.error('[pi-observe] Langfuse API keys are not configured; ingestion smoke skipped'); process.exitCode = 2; } else if (!result.ok) { console.error('[pi-observe] Langfuse ingestion smoke was not accepted'); process.exitCode = 1; } else { console.log('[pi-observe] Langfuse ingestion accepted'); } }
function help() { console.log(`pi-observe ${VERSION}\nUsage:\n  pi-observe run [--project key] [--tool name] [--role role] [--summary text] [--nonbillable] -- command [args...]\n  pi-observe mark-active [--project key] [--tool name] [--summary text]\n  pi-observe mark-inactive [--project key] [--tool name]\n  pi-observe smoke-ingest [--project key]\n  pi-observe reconcile\n  pi-observe status\n\nDefaults are metadata-only; raw prompts/output are not captured. Langfuse credentials are loaded from observability/langfuse/.env without shell sourcing and are not passed to child commands.`); }
const [cmd, ...rest] = process.argv.slice(2);
try { if (cmd === 'run') await run(rest); else if (cmd === 'mark-active') mark(true, rest); else if (cmd === 'mark-inactive') mark(false, rest); else if (cmd === 'smoke-ingest') await smokeIngest(rest); else if (cmd === 'reconcile') reconcileCommand(); else if (cmd === 'status') status(); else help(); } catch (e) { console.error(`[pi-observe] ${redact(e.message)}`); process.exit(2); }
