#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync, rmdirSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { randomUUID, createHash, randomBytes } from 'node:crypto';
import { execFileSync } from 'node:child_process';

const VERSION = '0.1.0';
const now = () => new Date().toISOString();
const home = process.env.HOME || process.cwd();
const stateDir = expand(process.env.PI_OBSERVE_STATE_DIR || join(home, '.local/state/pi-observe'));
const configDir = expand(process.env.PI_OBSERVE_CONFIG_DIR || join(home, '.config/pi-observe'));
const eventsPath = join(stateDir, 'events.jsonl');
const runsPath = join(stateDir, 'runs.json');

function expand(p) { return p.replace(/^~(?=$|\/)/, home); }
function ensureDirs() { mkdirSync(stateDir, { recursive: true, mode: 0o700 }); mkdirSync(configDir, { recursive: true, mode: 0o700 }); }
function redact(value) {
  if (value == null) return value;
  let s = String(value);
  s = s.replace(/github_pat_[A-Za-z0-9_]+|gh[pousr]_[A-Za-z0-9_]+/g, '[REDACTED_GITHUB_TOKEN]');
  s = s.replace(/sk-(?:ant-|proj-)?[A-Za-z0-9_-]{12,}/g, '[REDACTED_API_KEY]');
  s = s.replace(/xox[abprs]-[A-Za-z0-9-]+/g, '[REDACTED_SLACK_TOKEN]');
  s = s.replace(/AKIA[A-Z0-9]{16}/g, '[REDACTED_AWS_KEY]');
  s = s.replace(/-----BEGIN [^-]*PRIVATE KEY-----[\s\S]*?-----END [^-]*PRIVATE KEY-----/g, '[REDACTED_PRIVATE_KEY]');
  s = s.replace(/https?:\/\/([^\s/@:]+):([^\s/@]+)@/g, 'https://[REDACTED_CREDENTIALS]@');
  s = s.replace(/([A-Za-z0-9+/=]{36,})(?=.*\b(token|key|secret|password)\b)/gi, '[REDACTED_HIGH_ENTROPY]');
  return s;
}
function sanitizeObject(obj) {
  if (Array.isArray(obj)) return obj.map(sanitizeObject);
  if (obj && typeof obj === 'object') return Object.fromEntries(Object.entries(obj).map(([k,v]) => [k, sanitizeObject(v)]));
  return typeof obj === 'string' ? redact(obj) : obj;
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
  if (explicit) return { key: explicit, confidence: 'explicit' };
  if (process.env.PI_OBSERVE_PROJECT) return { key: process.env.PI_OBSERVE_PROJECT, confidence: 'env' };
  for (const name of ['.pi-project', '.vire-project']) {
    let dir = process.cwd();
    while (true) {
      const p = join(dir, name);
      if (existsSync(p)) return { key: readFileSync(p, 'utf8').trim().split(/\s+/)[0], confidence: name };
      const parent = dirname(dir); if (parent === dir) break; dir = parent;
    }
  }
  try {
    const cfg = JSON.parse(readFileSync(join(configDir, 'projects.json'), 'utf8'));
    const cwd = process.cwd();
    for (const [key, rule] of Object.entries(cfg.projects || {})) {
      if (rule.paths?.some(p => cwd.startsWith(expand(p)))) return { key, confidence: 'path-map' };
      const remote = safeGitRemoteHash();
      if (remote && rule.git_remote_hashes?.includes(remote)) return { key, confidence: 'git-remote-hash' };
    }
  } catch {}
  return { key: basename(process.cwd()), confidence: 'low' };
}
function safeGitBranch() { try { return execFileSync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { encoding:'utf8', timeout: 500, stdio:['ignore','pipe','ignore'] }).trim(); } catch { return undefined; } }
function safeGitRemoteHash() { try { const r = execFileSync('git', ['config', '--get', 'remote.origin.url'], { encoding:'utf8', timeout: 500, stdio:['ignore','pipe','ignore'] }).trim(); return createHash('sha256').update(r).digest('hex').slice(0,16); } catch { return undefined; } }
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
function updateActive(project, delta, runId) {
  return withStateLock(() => {
    const state = readRuns(); state.active[project] ||= { count: 0, runs: [] };
    state.active[project].count = Math.max(0, state.active[project].count + delta);
    if (delta > 0) state.active[project].runs.push(runId);
    else state.active[project].runs = state.active[project].runs.filter(r => r !== runId);
    state.active[project].updated_at = now(); writeRuns(state); return state.active[project].count;
  });
}
async function sendLangfuse(batch) {
  if (process.env.PI_OBSERVE_ENABLED === 'false') return;
  const host = process.env.LANGFUSE_HOST || 'http://localhost:3000';
  const pub = process.env.LANGFUSE_PUBLIC_KEY, sec = process.env.LANGFUSE_SECRET_KEY;
  if (!pub || !sec) return;
  const ac = new AbortController(); const t = setTimeout(() => ac.abort(), Number(process.env.PI_OBSERVE_LANGFUSE_TIMEOUT_MS || 1200));
  try {
    await fetch(`${host.replace(/\/$/,'')}/api/public/ingestion`, { method:'POST', signal: ac.signal, headers:{ 'content-type':'application/json', authorization: `Basic ${Buffer.from(`${pub}:${sec}`).toString('base64')}` }, body: JSON.stringify({ batch }) });
  } catch (e) { console.warn(`[pi-observe] telemetry unavailable; command/result preserved locally (${e.name || 'error'})`); }
  finally { clearTimeout(t); }
}
async function run(argv) {
  const opts = parseArgs(argv); if (!opts.command.length) throw new Error('Missing command after --');
  if (process.env.PI_OBSERVE_ENABLED === 'false') return spawnPassthrough(opts.command);
  ensureDirs(); const project = resolveProject(opts.project); const runId = randomUUID(); const traceId = randomUUID().replaceAll('-',''); const start = Date.now();
  const meta = { wrapper_version: VERSION, project_key: project.key, project_confidence: project.confidence, tool: opts.tool || 'command', role: opts.role, cwd_basename: basename(process.cwd()), git_branch: safeGitBranch(), git_remote_hash: safeGitRemoteHash(), command_label: opts.label || opts.command[0], billable: opts.billable, summary: opts.summary ? redact(opts.summary) : undefined };
  appendEvent({ event:'tool_started', project:project.key, tool:meta.tool, role:opts.role, run_id:runId, session_id: opts.session || process.env.PI_OBSERVE_SESSION || randomUUID(), ts: now(), billable: opts.billable, metadata: meta });
  if (opts.billable) updateActive(project.key, 1, runId);
  await sendLangfuse([{ id: randomUUID(), timestamp: now(), type:'trace-create', body:{ id: traceId, name: `${meta.tool}${opts.role ? '.'+opts.role : ''}`, userId: process.env.PI_OBSERVE_USER_ID || 'local-janne', sessionId: opts.session || process.env.PI_OBSERVE_SESSION || runId, tags:['local', meta.tool, opts.role, project.key].filter(Boolean), metadata: sanitizeObject(meta) } }]);
  const code = await spawnPassthrough(opts.command);
  const end = Date.now(); const status = code === 0 ? 'success' : code === 130 ? 'canceled' : 'failed';
  appendEvent({ event:'tool_finished', project:project.key, run_id:runId, status, exit_code: code, duration_ms:end-start, ts:now() });
  let active = opts.billable ? updateActive(project.key, -1, runId) : 0;
  if (opts.billable && active === 0) appendEvent({ event:'idle_countdown_started', project:project.key, after_run_id:runId, threshold_ms:Number(process.env.PI_OBSERVE_IDLE_THRESHOLD_MS || 300000), ts:now() });
  await sendLangfuse([{ id: randomUUID(), timestamp: now(), type:'trace-update', body:{ id: traceId, tags:['local', meta.tool, opts.role, project.key, status].filter(Boolean), metadata: sanitizeObject({ ...meta, status, exit_code: code, duration_ms:end-start, end_time: now() }) } }]);
  process.exitCode = code;
}
function spawnPassthrough(cmd) { return new Promise(resolve => { const child = spawn(cmd[0], cmd.slice(1), { stdio:'inherit' }); child.on('error', e => { console.error(`[pi-observe] failed to start command: ${e.message}`); resolve(127); }); child.on('close', (code, signal) => resolve(code ?? (signal === 'SIGINT' ? 130 : 1))); }); }
function mark(active, argv) { const opts = parseArgs(argv); const project = resolveProject(opts.project); const runId = opts.session || randomUUID(); appendEvent({ event: active ? 'manual_active' : 'manual_inactive', project: project.key, tool: opts.tool || 'manual', role: opts.role, run_id: runId, summary: opts.summary ? redact(opts.summary) : undefined, ts: now() }); if (active) updateActive(project.key, 1, runId); else { const c = updateActive(project.key, -1, runId); if (c === 0) appendEvent({ event:'idle_countdown_started', project: project.key, after_run_id: runId, threshold_ms:Number(process.env.PI_OBSERVE_IDLE_THRESHOLD_MS || 300000), ts: now() }); } }
function status() { console.log(JSON.stringify({ state_dir: stateDir, events_path: eventsPath, active: readRuns().active }, null, 2)); }
function help() { console.log(`pi-observe ${VERSION}\nUsage:\n  pi-observe run [--project key] [--tool name] [--role role] [--summary text] -- command [args...]\n  pi-observe mark-active [--project key] [--tool name] [--summary text]\n  pi-observe mark-inactive [--project key] [--tool name]\n  pi-observe status\n\nDefaults are metadata-only; raw prompts/output are not captured.`); }
const [cmd, ...rest] = process.argv.slice(2);
try { if (cmd === 'run') await run(rest); else if (cmd === 'mark-active') mark(true, rest); else if (cmd === 'mark-inactive') mark(false, rest); else if (cmd === 'status') status(); else help(); } catch (e) { console.error(`[pi-observe] ${e.message}`); process.exit(2); }
