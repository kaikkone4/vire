#!/usr/bin/env node
// Non-shipping spike probe for TASK-007 Langfuse importer validation.
// Read-only. Loopback-only. Emits SHAPE not CONTENT: it prints field names,
// value types, nullability, and counts — never trace values, prompts, responses,
// usage/cost numbers tied to real work, secrets, or environment dumps.
//
// Usage (only when the local stack is up and observability/langfuse/.env exists):
//   node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment vire
//   node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment default
//
// It does NOT import, reuse, or modify observability/pi-observe or the product
// runtime. It is supporting evidence for ../../../openspec/changes/
// task-007-langfuse-importer-validation/langfuse-validation-report.md.

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '../../..');
const dotenvPath = join(repoRoot, 'observability/langfuse/.env');
const ALLOWED = new Set(['LANGFUSE_HOST', 'LANGFUSE_PUBLIC_KEY', 'LANGFUSE_SECRET_KEY']);

function parseArgs(argv) {
  const o = { environment: 'vire', limit: 50, hours: 24 };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === '--environment') o.environment = argv[++i];
    else if (argv[i] === '--limit') o.limit = Number(argv[++i]);
    else if (argv[i] === '--hours') o.hours = Number(argv[++i]);
  }
  return o;
}

// Data-only .env parser (no shell evaluation), reads only the three read-API keys.
function readEnv() {
  const out = {};
  if (!existsSync(dotenvPath)) return out;
  for (const line of readFileSync(dotenvPath, 'utf8').split(/\r?\n/)) {
    if (!line.trim() || line.trimStart().startsWith('#')) continue;
    const m = line.match(/^([A-Z0-9_]+)=(.*)$/);
    if (!m || !ALLOWED.has(m[1])) continue;
    let v = m[2].trim();
    if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) v = v.slice(1, -1);
    if (/[`$;]/.test(v)) continue;
    out[m[1]] = v;
  }
  return out;
}

function isLoopback(host) {
  try {
    const h = new URL(host).hostname.toLowerCase().replace(/^\[|\]$/g, '');
    return ['localhost', '127.0.0.1', '::1', '0:0:0:0:0:0:0:1'].includes(h);
  } catch { return false; }
}

// Reduce any value to a non-reversible SHAPE descriptor: type + nullability +
// (for strings) length bucket only. Never returns the underlying value.
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

// Record the key->shape map without ever printing values.
function schemaSketch(obj, into = {}) {
  if (!obj || typeof obj !== 'object') return into;
  for (const [k, v] of Object.entries(obj)) {
    const slot = (into[k] ||= new Set());
    slot.add(shapeOf(v));
  }
  return into;
}

function printSchema(label, sketch) {
  console.log(`\n## ${label}`);
  const keys = Object.keys(sketch).sort();
  if (!keys.length) { console.log('  (no fields observed)'); return; }
  for (const k of keys) console.log(`  ${k}: ${[...sketch[k]].sort().join(' | ')}`);
}

async function getJSON(host, path, auth) {
  const ac = new AbortController();
  const t = setTimeout(() => ac.abort(), 4000);
  try {
    const res = await fetch(`${host.replace(/\/$/, '')}${path}`, {
      signal: ac.signal,
      headers: { authorization: `Basic ${auth}` },
    });
    const status = res.status;
    let body = null;
    try { body = await res.json(); } catch { /* shape-only; ignore non-JSON */ }
    return { status, body };
  } finally { clearTimeout(t); }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const env = readEnv();
  const host = env.LANGFUSE_HOST || 'http://localhost:3000';
  if (!isLoopback(host)) {
    console.error('[probe] refusing non-loopback host (SEC-002). Spike is local-only.');
    process.exit(2);
  }
  if (!env.LANGFUSE_PUBLIC_KEY || !env.LANGFUSE_SECRET_KEY) {
    console.error('[probe] LANGFUSE_PUBLIC_KEY/SECRET_KEY not configured in observability/langfuse/.env.');
    console.error('[probe] Bring up the stack and create project keys first. No secrets are printed.');
    process.exit(2);
  }
  const auth = Buffer.from(`${env.LANGFUSE_PUBLIC_KEY}:${env.LANGFUSE_SECRET_KEY}`).toString('base64');

  console.log(`# Langfuse read-API SHAPE probe (loopback ${new URL(host).host}, environment=${args.environment})`);
  console.log('# Output is shape-only: field names, types, nullability, counts. No values, no secrets.');

  const health = await getJSON(host, '/api/public/health', auth);
  console.log(`\nhealth: HTTP ${health.status}`);

  const to = new Date();
  const from = new Date(to.getTime() - args.hours * 3600_000);
  const q = `environment=${encodeURIComponent(args.environment)}` +
    `&fromTimestamp=${encodeURIComponent(from.toISOString())}` +
    `&toTimestamp=${encodeURIComponent(to.toISOString())}` +
    `&limit=${args.limit}`;

  // Pagination + dedup proof (by trace id, scoped to the queried environment).
  const seen = new Set();
  const traceSketch = {};
  let page = 1, pages = 1, total = 0, dupes = 0, maxTs = null;
  do {
    const r = await getJSON(host, `/api/public/traces?${q}&page=${page}`, auth);
    if (r.status === 401 || r.status === 403) { console.error(`\n[probe] auth/config failure: HTTP ${r.status} (auth_or_config_error)`); break; }
    if (r.status === 429) { console.error('\n[probe] rate limited: HTTP 429 (rate_limited) — back off and retry'); break; }
    if (r.status !== 200 || !r.body) { console.error(`\n[probe] unexpected status HTTP ${r.status}`); break; }
    const data = Array.isArray(r.body.data) ? r.body.data : [];
    pages = r.body.meta?.totalPages ?? page;
    for (const tr of data) {
      total++;
      if (tr.id && seen.has(`${args.environment}:${tr.id}`)) { dupes++; continue; }
      if (tr.id) seen.add(`${args.environment}:${tr.id}`);
      schemaSketch(tr, traceSketch);
      const ts = tr.timestamp || tr.createdAt;
      if (ts && (!maxTs || ts > maxTs)) maxTs = ts;
    }
    page++;
  } while (page <= pages);

  printSchema(`trace list schema (GET /api/public/traces, environment=${args.environment})`, traceSketch);

  // Detail + observation usage/cost shape from the first unique trace only.
  const firstId = [...seen][0]?.split(':').slice(1).join(':');
  if (firstId) {
    const detail = await getJSON(host, `/api/public/traces/${encodeURIComponent(firstId)}`, auth);
    if (detail.status === 200 && detail.body) {
      printSchema('trace detail schema (GET /api/public/traces/{id})', schemaSketch(detail.body));
      const obs = Array.isArray(detail.body.observations) ? detail.body.observations : [];
      const obsSketch = {}, usageSketch = {};
      for (const o of obs) {
        schemaSketch(o, obsSketch);
        if (o.usage) schemaSketch(o.usage, usageSketch);
      }
      printSchema('observation schema (usage/cost live here, NOT on pi-observe traces)', obsSketch);
      printSchema('observation.usage schema', usageSketch);
    }
  }

  console.log('\n## import-flow proof');
  console.log(`  unique traces (deduped): ${seen.size}`);
  console.log(`  raw rows scanned: ${total}`);
  console.log(`  duplicates suppressed: ${dupes}`);
  console.log(`  pages walked: ${page - 1} (reported totalPages=${pages})`);
  console.log(`  next per-environment cursor (max observed timestamp): ${maxTs ?? '(none — env empty; absence != zero cost)'}`);
  console.log('\n# Done. No values were printed. Redirect to *.local.log (gitignored) if you want an ephemeral record; delete when done.');
}

main().catch((e) => { console.error(`[probe] error: ${e.name || 'error'}`); process.exit(1); });
