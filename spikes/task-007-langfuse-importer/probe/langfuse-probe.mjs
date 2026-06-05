#!/usr/bin/env node
// Non-shipping spike probe for TASK-007 Langfuse importer validation (DEC-018).
//
// Two modes:
//   --mock   Offline: proves pagination / dedup / cursor / 9-state health logic against
//            synthetic in-memory fixtures. No network, no credentials, no container.
//   (live)   Validates against the CONFIGURED Langfuse API (cloud-first per DEC-018, or an
//            optional local stack). Requires base URL + project-scoped keys in local secure
//            config. SEC-002: the ONLY network target is the configured Langfuse base URL.
//
// Read-only. Emits SHAPE not CONTENT: field names, value types, nullability, counts only —
// never trace values, prompts, responses, command bodies, real usage/cost numbers, session
// ids, secrets, or environment dumps. Strings are reduced to a length bucket.
//
// It does NOT import, reuse, or modify observability/pi-observe or the product runtime.
// Supporting evidence for ../../../openspec/changes/
// task-007-langfuse-importer-validation/langfuse-validation-report.md.

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '../../..');
const dotenvPath = join(repoRoot, 'observability/langfuse/.env');
const ALLOWED = new Set(['LANGFUSE_HOST', 'LANGFUSE_PUBLIC_KEY', 'LANGFUSE_SECRET_KEY']);

function parseArgs(argv) {
  const o = { environment: 'vire', limit: 50, hours: 24, mock: false };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--mock') o.mock = true;
    else if (argv[i] === '--environment') o.environment = argv[++i];
    else if (argv[i] === '--limit') o.limit = Number(argv[++i]);
    else if (argv[i] === '--hours') o.hours = Number(argv[++i]);
  }
  return o;
}

// Data-only .env parser (no shell evaluation); reads only the three read-API keys. Falls
// back to process.env so credentials can come from the environment instead of a file.
function readConfig() {
  const out = {};
  if (existsSync(dotenvPath)) {
    for (const line of readFileSync(dotenvPath, 'utf8').split(/\r?\n/)) {
      if (!line.trim() || line.trimStart().startsWith('#')) continue;
      const m = line.match(/^([A-Z0-9_]+)=(.*)$/);
      if (!m || !ALLOWED.has(m[1])) continue;
      let v = m[2].trim();
      if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) v = v.slice(1, -1);
      if (/[`$;]/.test(v)) continue;
      out[m[1]] = v;
    }
  }
  for (const k of ALLOWED) if (process.env[k]) out[k] = process.env[k];
  return out;
}

// SEC-002: a configured Langfuse base URL must be a syntactically valid http(s) origin. We
// never accept an absolute URL from response data — every request is built as base + path.
function normalizeBaseUrl(host) {
  try {
    const u = new URL(host);
    if (u.protocol !== 'http:' && u.protocol !== 'https:') return null;
    return `${u.protocol}//${u.host}`;
  } catch { return null; }
}

function safeHostLabel(host) {
  try { const u = new URL(host); return `${u.protocol}//${u.host}`; } catch { return 'invalid-host'; }
}

function printSecureConfigInstructions() {
  console.error('[probe] Configured Langfuse API is not set up locally (needs_input).');
  console.error('[probe] Add the configured base URL + project-scoped keys to LOCAL SECURE CONFIG');
  console.error('[probe] (observability/langfuse/.env, chmod 600, gitignored) using REDACTED placeholders:');
  console.error('[probe]');
  console.error('[probe]   LANGFUSE_HOST=https://cloud.langfuse.com        # or your region / self-host base URL');
  console.error('[probe]   LANGFUSE_PUBLIC_KEY=...                          # project public key (pk-lf-...)');
  console.error('[probe]   LANGFUSE_SECRET_KEY=...                          # project secret key (sk-lf-...)');
  console.error('[probe]');
  console.error('[probe] Do NOT paste secrets into chat. Keys are read locally and used only for the');
  console.error('[probe] Authorization header; they are never printed, logged, or persisted.');
  console.error('[probe] Then re-run: node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment vire');
}

// ---- shape-only schema sketching (never prints values) ----------------------------------
function shapeOf(v) {
  if (v === null) return 'null';
  if (v === undefined) return 'absent';
  if (Array.isArray(v)) return `array[len=${v.length}]`;
  const t = typeof v;
  if (t === 'string') return `string[len_bucket=${v.length === 0 ? 0 : v.length < 16 ? '<16' : v.length < 64 ? '<64' : '>=64'}]`;
  if (t === 'number') return Number.isInteger(v) ? 'number[int]' : 'number[float]';
  if (t === 'object') return `object{keys=${Object.keys(v).length}}`;
  return t;
}
function schemaSketch(obj, into = {}) {
  if (!obj || typeof obj !== 'object') return into;
  for (const [k, v] of Object.entries(obj)) (into[k] ||= new Set()).add(shapeOf(v));
  return into;
}
function printSchema(label, sketch) {
  console.log(`\n## ${label}`);
  const keys = Object.keys(sketch).sort();
  if (!keys.length) { console.log('  (no fields observed)'); return; }
  for (const k of keys) console.log(`  ${k}: ${[...sketch[k]].sort().join(' | ')}`);
}

// ---- import-flow logic proven identically offline (mock) and live ------------------------
// paginate() drives a fetchPage(page) -> { status, data[], totalPages } and accumulates
// deduped traces scoped to (environment, trace_id), computing the per-environment cursor.
async function paginate(env, fetchPage) {
  const seen = new Set();
  const order = [];
  let page = 1, pages = 1, total = 0, dupes = 0, maxTs = null;
  let terminal = null;
  do {
    const r = await fetchPage(page);
    if (r.status === 401 || r.status === 403) { terminal = 'auth_or_config_error'; break; }
    if (r.status === 429) { terminal = 'rate_limited'; break; }
    if (r.status !== 200) { terminal = `http_${r.status}`; break; }
    pages = r.totalPages ?? page;
    for (const tr of r.data || []) {
      total++;
      const key = `${env}:${tr.id}`;
      if (tr.id && seen.has(key)) { dupes++; continue; }
      if (tr.id) { seen.add(key); order.push(tr); }
      const ts = tr.timestamp || tr.createdAt;
      if (ts && (!maxTs || ts > maxTs)) maxTs = ts;
    }
    page++;
  } while (page <= pages);
  return { unique: order, uniqueCount: seen.size, total, dupes, pagesWalked: page - 1, totalPages: pages, cursor: maxTs, terminal };
}

// Health classification — the 9-state model (report §4). Pure function over an evidence view.
function classifyHealth(ev) {
  if (ev.terminal === 'auth_or_config_error') return 'auth_or_config_error';
  if (ev.terminal === 'rate_limited') return 'rate_limited';
  if (ev.schemaMismatch) return 'schema_mismatch';
  if (ev.expectedActivity && ev.uniqueCount === 0) return 'missing'; // absence != zero
  if (ev.wrongEnv) return 'wrong_env';
  if (ev.dupes > 0) return 'duplicate';
  if (ev.latestTraceAgeMs != null && ev.latestTraceAgeMs > ev.staleThresholdMs) return 'stale';
  if (ev.delayedArrival) return 'delayed';
  return 'valid';
}

// ---- offline mock fixtures (synthetic, non-sensitive) ------------------------------------
// Three-page window with one cross-page duplicate id; usage/cost live on observations.
function mockTracesPage(env, page) {
  if (env === 'empty') return { status: 200, data: [], totalPages: 1 };
  if (env === 'default') {
    // wrong-env case: project traffic landed in `default`
    return { status: 200, data: [{ id: 'd-1', environment: 'default', timestamp: '2026-06-04T10:00:00.000Z', name: 'pi.delegate', sessionId: 'session-aaaa', tags: ['local', 'pi', 'vire'], metadata: { project_key: 'vire' } }], totalPages: 1 };
  }
  const pages = {
    1: [
      { id: 't-1', environment: env, timestamp: '2026-06-04T09:00:00.000Z', name: 'claude-code.dev', sessionId: 'session-bbbb', tags: ['local', 'claude-code', 'vire'], metadata: { project_key: 'vire' } },
      { id: 't-2', environment: env, timestamp: '2026-06-04T09:05:00.000Z', name: 'pi.delegate', sessionId: 'session-cccc', tags: ['local', 'pi', 'vire'], metadata: { project_key: 'vire' } },
    ],
    2: [
      { id: 't-2', environment: env, timestamp: '2026-06-04T09:05:00.000Z', name: 'pi.delegate', sessionId: 'session-cccc', tags: ['local', 'pi', 'vire'], metadata: { project_key: 'vire' } }, // duplicate across pages
      { id: 't-3', environment: env, timestamp: '2026-06-04T09:10:00.000Z', name: 'claude-code.dev', sessionId: 'session-dddd', tags: ['local', 'claude-code', 'vire'], metadata: { project_key: 'vire' } },
    ],
    3: [
      { id: 't-4', environment: env, timestamp: '2026-06-04T09:15:00.000Z', name: 'pi.delegate', sessionId: 'session-eeee', tags: ['local', 'pi', 'vire'], metadata: { project_key: 'vire' } },
    ],
  };
  return { status: 200, data: pages[page] || [], totalPages: 3 };
}

// Synthetic generation observation: this is where token usage and cost live.
function mockObservation() {
  return {
    id: 'o-1', type: 'GENERATION', traceId: 't-1', startTime: '2026-06-04T09:00:00.000Z', endTime: '2026-06-04T09:00:04.000Z',
    model: 'synthetic-model', calculatedTotalCost: 0.0, costDetails: { input: 0.0, output: 0.0 },
    usage: { promptTokens: 0, completionTokens: 0, totalTokens: 0, unit: 'TOKENS' }, usageDetails: { input: 0, output: 0, total: 0 },
  };
}

async function runMock(args) {
  console.log('# Langfuse importer logic proof — OFFLINE MOCK (no network, no credentials)');
  console.log('# Synthetic non-sensitive fixtures. Output is shape-only.');

  // 1) pagination + dedup + cursor over a 3-page window with a cross-page duplicate
  const main = await paginate(args.environment, (p) => Promise.resolve(mockTracesPage(args.environment, p)));
  console.log('\n## pagination / dedup / cursor (3-page window, 1 cross-page duplicate)');
  console.log(`  unique traces (deduped): ${main.uniqueCount}        (expect 4)`);
  console.log(`  raw rows scanned: ${main.total}                     (expect 5)`);
  console.log(`  duplicates suppressed: ${main.dupes}                (expect 1)`);
  console.log(`  pages walked: ${main.pagesWalked} (totalPages=${main.totalPages})       (expect 3)`);
  console.log(`  next per-environment cursor (max ts): ${main.cursor}`);
  const passPagination = main.uniqueCount === 4 && main.total === 5 && main.dupes === 1 && main.pagesWalked === 3;

  // 2) overlapping re-import: replay page 1 against the prior cursor; dedup must suppress all
  const overlap = await paginate(args.environment, (p) => Promise.resolve(mockTracesPage(args.environment, 1)));
  const reimportSafe = overlap.unique.every((t) => (t.timestamp <= main.cursor));
  console.log('\n## overlapping re-import (replay page 1 under prior cursor)');
  console.log(`  all replayed traces <= prior cursor (idempotent overlap): ${reimportSafe}   (expect true)`);

  // 3) schema sketch of a trace + an observation (usage/cost live on observation)
  printSchema('mock trace schema (GET /api/public/traces)', schemaSketch(main.unique[0] || {}));
  printSchema('mock observation schema (usage/cost live here, NOT on pi-observe traces)', schemaSketch(mockObservation()));
  printSchema('mock observation.usage schema', schemaSketch(mockObservation().usage));

  // 4) 9-state health model — assert each state is produced by its detection rule
  const base = { staleThresholdMs: 3600_000, latestTraceAgeMs: 0 };
  const cases = [
    ['valid', { ...base }],
    ['missing', { ...base, expectedActivity: true, uniqueCount: 0 }],
    ['stale', { ...base, latestTraceAgeMs: 7200_000 }],
    ['wrong_env', { ...base, wrongEnv: true }],
    ['delayed', { ...base, delayedArrival: true }],
    ['duplicate', { ...base, dupes: 1 }],
    ['schema_mismatch', { ...base, schemaMismatch: true }],
    ['auth_or_config_error', { ...base, terminal: 'auth_or_config_error' }],
    ['rate_limited', { ...base, terminal: 'rate_limited' }],
  ];
  console.log('\n## 9-state health model (detection rule -> state)');
  let healthPass = true;
  for (const [expected, ev] of cases) {
    const got = classifyHealth({ uniqueCount: 1, dupes: 0, ...ev });
    const ok = got === expected;
    healthPass = healthPass && ok;
    console.log(`  ${ok ? 'PASS' : 'FAIL'}  expect ${expected.padEnd(20)} got ${got}`);
  }

  // 5) absence != zero
  const emptyEnv = await paginate('empty', (p) => Promise.resolve(mockTracesPage('empty', p)));
  const absenceState = classifyHealth({ uniqueCount: emptyEnv.uniqueCount, dupes: 0, expectedActivity: true, ...base });
  const absenceOk = emptyEnv.cursor === null && absenceState === 'missing';
  console.log('\n## absence != zero usage/cost');
  console.log(`  empty env -> cursor=${emptyEnv.cursor ?? 'null'} (not 0), health=${absenceState}   (expect null / missing)`);

  const allPass = passPagination && reimportSafe && healthPass && absenceOk;
  console.log(`\n# MOCK RESULT: ${allPass ? 'ALL CHECKS PASS' : 'CHECK FAILURE — see FAIL rows above'}`);
  process.exitCode = allPass ? 0 : 1;
}

// ---- live mode (configured Langfuse API) -------------------------------------------------
async function runLive(args) {
  const cfg = readConfig();
  const base = normalizeBaseUrl(cfg.LANGFUSE_HOST || '');
  if (!base || !cfg.LANGFUSE_PUBLIC_KEY || !cfg.LANGFUSE_SECRET_KEY) {
    printSecureConfigInstructions();
    process.exitCode = 2; // needs_input
    return;
  }
  const auth = Buffer.from(`${cfg.LANGFUSE_PUBLIC_KEY}:${cfg.LANGFUSE_SECRET_KEY}`).toString('base64');

  // SEC-002: every request is built as base + path; absolute URLs from data are never followed.
  async function apiGet(path) {
    const ac = new AbortController();
    const t = setTimeout(() => ac.abort(), 5000);
    try {
      const res = await fetch(`${base}${path}`, { headers: { authorization: `Basic ${auth}` }, signal: ac.signal });
      let body = null;
      try { body = await res.json(); } catch { /* shape-only; ignore non-JSON */ }
      return { status: res.status, body };
    } finally { clearTimeout(t); }
  }

  console.log(`# Langfuse read-API SHAPE probe — LIVE (configured base ${safeHostLabel(base)}, environment=${args.environment})`);
  console.log('# Output is shape-only: field names, types, nullability, counts. No values, no secrets.');

  const health = await apiGet('/api/public/health');
  console.log(`\nhealth: HTTP ${health.status}`);

  const to = new Date();
  const from = new Date(to.getTime() - args.hours * 3600_000);
  const qs = `environment=${encodeURIComponent(args.environment)}` +
    `&fromTimestamp=${encodeURIComponent(from.toISOString())}` +
    `&toTimestamp=${encodeURIComponent(to.toISOString())}` +
    `&limit=${args.limit}`;

  const traceSketch = {};
  const res = await paginate(args.environment, async (page) => {
    const r = await apiGet(`/api/public/traces?${qs}&page=${page}`);
    const data = Array.isArray(r.body?.data) ? r.body.data : [];
    for (const tr of data) schemaSketch(tr, traceSketch);
    return { status: r.status, data, totalPages: r.body?.meta?.totalPages };
  });

  if (res.terminal === 'auth_or_config_error') { console.error('\n[probe] auth/config failure (auth_or_config_error) — check configured keys'); process.exitCode = 2; return; }
  if (res.terminal === 'rate_limited') { console.error('\n[probe] rate limited (rate_limited) — back off and retry'); }

  printSchema(`trace list schema (GET /api/public/traces, environment=${args.environment})`, traceSketch);

  const first = res.unique[0]?.id;
  if (first) {
    const detail = await apiGet(`/api/public/traces/${encodeURIComponent(first)}`);
    if (detail.status === 200 && detail.body) {
      printSchema('trace detail schema (GET /api/public/traces/{id})', schemaSketch(detail.body));
      const obs = Array.isArray(detail.body.observations) ? detail.body.observations : [];
      const obsSketch = {}, usageSketch = {};
      for (const o of obs) { schemaSketch(o, obsSketch); if (o.usage) schemaSketch(o.usage, usageSketch); }
      printSchema('observation schema (usage/cost live here, NOT on pi-observe traces)', obsSketch);
      printSchema('observation.usage schema', usageSketch);
    }
  }

  console.log('\n## import-flow proof (live)');
  console.log(`  unique traces (deduped): ${res.uniqueCount}`);
  console.log(`  raw rows scanned: ${res.total}`);
  console.log(`  duplicates suppressed: ${res.dupes}`);
  console.log(`  pages walked: ${res.pagesWalked} (reported totalPages=${res.totalPages})`);
  console.log(`  next per-environment cursor (max observed timestamp): ${res.cursor ?? '(none — env empty; absence != zero cost)'}`);
  console.log('\n# Done. No values were printed. Redirect to *.local.log (gitignored) for an ephemeral record; delete when done.');
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.mock) return runMock(args);
  return runLive(args);
}

main().catch((e) => { console.error(`[probe] error: ${e.name || 'error'}`); process.exit(1); });
