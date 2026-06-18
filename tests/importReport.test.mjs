import test from 'node:test';
import assert from 'node:assert/strict';
import { renderImportReport, skipReasonLabel } from '../src/import-report.ts';

// A report shaped like the backend `ImportReport` (TASK-029 A/B/C): the 611-skip field-report case,
// after the B fix — the v3 id-list traces are imported (informational `observations_not_embedded`),
// not dropped, and a handful genuinely degrade to `schema_changed`.
const reportNewData = () => ({
  total_traces_seen: 640,
  total_unique: 611,
  total_duplicates: 29,
  total_skipped_schema: 0,
  total_skip_reasons: [{ reason: 'observations_not_embedded', count: 611 }],
  reached_page_limit: false,
  environment_count: 2,
  environments: [
    {
      environment: 'vire',
      health: 'healthy',
      pages: 7,
      traces_seen: 400,
      unique: 380,
      duplicates: 20,
      skipped_schema: 0,
      skip_reasons: [{ reason: 'observations_not_embedded', count: 380 }],
      skip_samples: [
        { reason: 'observations_not_embedded', keys: ['id', 'timestamp', 'observations'], field: 'observations', field_type: 'array', element_type: 'string' },
      ],
      reached_page_limit: false,
      warnings: [],
    },
    {
      environment: 'staging',
      health: 'healthy',
      pages: 5,
      traces_seen: 240,
      unique: 231,
      duplicates: 9,
      skipped_schema: 0,
      skip_reasons: [{ reason: 'observations_not_embedded', count: 231 }],
      skip_samples: [],
      reached_page_limit: false,
      warnings: [],
    },
  ],
});

test('null report renders nothing (integration disabled returns no report)', () => {
  assert.equal(renderImportReport(null), '');
});

test('headline reports new traces and the report is a single aria-live block', () => {
  const html = renderImportReport(reportNewData(), 'incremental');
  assert.match(html, /aria-live="polite"/);
  assert.match(html, /Imported 611 new traces across 2 environments — 29 duplicates, 0 skipped\./);
  // Incremental: no backfill prefix.
  assert.doesNotMatch(html, /Backfill/);
});

test('reasons are GROUPED — one line per reason, never the repeated per-trace warning', () => {
  const html = renderImportReport(reportNewData(), 'incremental');
  // One grouped diagnostics line carrying the 611 count.
  assert.match(html, /Diagnostics \(grouped\):<\/b> 611 observations not embedded/);
  // The legacy opaque repeated string is gone.
  assert.doesNotMatch(html, /did not match the expected shape/);
  // The grouped count appears once, not 611 times.
  assert.equal(html.split('observations not embedded').length - 1 <= 4, true);
});

test('each environment shows seen / new / duplicate / skipped', () => {
  const html = renderImportReport(reportNewData(), 'incremental');
  assert.match(html, /<b>vire<\/b>: healthy — 400 seen, 380 new, 20 duplicate, 0 skipped/);
  assert.match(html, /<b>staging<\/b>: healthy — 240 seen, 231 new, 9 duplicate, 0 skipped/);
});

test('structural samples show key/type NAMES only — never a field value', () => {
  const html = renderImportReport(reportNewData(), 'incremental');
  assert.match(html, /Shape samples/);
  assert.match(html, /keys \[id, timestamp, observations\]/);
  assert.match(html, /<code>observations<\/code>: array of string/);
});

test('backfill headline is distinguished from incremental', () => {
  const r = reportNewData();
  r.total_unique = 0;
  const html = renderImportReport(r, 'backfill');
  assert.match(html, /Backfill — No new traces imported across 2 environments \(640 seen, 29 duplicates, 0 skipped\)/);
});

test('a page-limit run says so rather than truncating silently', () => {
  const r = reportNewData();
  r.reached_page_limit = true;
  r.environments[0].reached_page_limit = true;
  const html = renderImportReport(r, 'backfill');
  assert.match(html, /reached the pagination limit/i);
  assert.match(html, /Re-run to continue — no data was truncated silently/);
  // The "re-run to continue" claim is truthful: the backend persists a continuation boundary, so the
  // note states that repeated runs reach progressively further back (not the same page 1 every time).
  assert.match(html, /resumes from the oldest history already fetched/);
  assert.match(html, /reach progressively further back/);
  assert.match(html, /<b>vire<\/b>:[^<]*<b>reached page limit — re-run to continue<\/b>/);
});

test('genuine degrades surface as skipped with their grouped reason', () => {
  const r = reportNewData();
  r.total_skipped_schema = 3;
  r.total_skip_reasons = [
    { reason: 'observations_not_embedded', count: 600 },
    { reason: 'generation_lacks_usage_and_cost', count: 3 },
  ];
  r.environments[0].skipped_schema = 3;
  r.environments[0].skip_reasons = [
    { reason: 'observations_not_embedded', count: 377 },
    { reason: 'generation_lacks_usage_and_cost', count: 3 },
  ];
  const html = renderImportReport(r, 'incremental');
  assert.match(html, /3 generation lacks usage and cost/);
  assert.match(html, /<b>vire<\/b>: healthy — 400 seen, 380 new, 20 duplicate, 3 skipped/);
});

test('environment names, reasons, and sample keys are escaped (no raw HTML injection)', () => {
  const r = reportNewData();
  r.environments[0].environment = '<img src=x onerror=alert(1)>';
  r.environments[0].skip_samples = [
    { reason: 'field_type_mismatch', keys: ['<script>', 'id'], field: '<b>timestamp</b>', field_type: 'number' },
  ];
  const html = renderImportReport(r, 'incremental');
  assert.doesNotMatch(html, /<img src=x/);
  assert.doesNotMatch(html, /<script>/);
  assert.match(html, /&lt;img src=x/);
  assert.match(html, /&lt;script&gt;/);
});

test('SEC-011 — the rendered report contains no secret/prompt/value material', () => {
  // Even if a (malicious or buggy) sample carried secret-shaped strings, the report only ever renders
  // structural names; this asserts the rendered output is free of the secret markers and that the
  // model surfaces structure (key/type names), never values.
  const html = renderImportReport(reportNewData(), 'incremental');
  for (const needle of ['sk-', 'pk-', 'Bearer', 'Authorization', 'oat01', 'prompt', 'session']) {
    assert.ok(!html.includes(needle), `import report must not contain ${needle}`);
  }
});

test('skipReasonLabel maps every fixed reason and falls back for unknowns', () => {
  assert.equal(skipReasonLabel('missing_trace_id'), 'missing trace id');
  assert.equal(skipReasonLabel('observations_fetch_failed'), 'observations fetch failed');
  // Unknown future variant → its raw key (escaped at render).
  assert.equal(skipReasonLabel('some_future_reason'), 'some_future_reason');
});
