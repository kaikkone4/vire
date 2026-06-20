import test from 'node:test';
import assert from 'node:assert/strict';
import {
  durationLabel,
  spanLabel,
  tokensLabel,
  costLabel,
  suggestionRow,
  suggestionGroups,
  unmappedNotice,
  suggestionsBody,
} from '../src/suggestions-ui.ts';

// A timed suggestion shaped like the backend `Suggestion` (suggestions/mod.rs), snake_case fields.
const timed = (over = {}) => ({
  id: 's-1',
  project_id: 'p-1',
  project_name: 'Veronavi',
  date: '2026-06-20',
  block_start_ts: '2026-06-20 09:12:00',
  block_end_ts: '2026-06-20 10:48:00',
  duration_minutes: 96,
  trace_count: 4,
  session_count: 2,
  total_tokens: 12345,
  cost_total: 0.4231,
  cost_currency: 'USD',
  health: 'healthy',
  confidence: 'high',
  source: 'langfuse:veronavi',
  reason: '4 Langfuse traces, 2 sessions in env veronavi, 09:12–10:48',
  status: 'pending',
  accepted_entry_id: null,
  created_at: '2026-06-20 11:00:00',
  updated_at: '2026-06-20 11:00:00',
  ...over,
});

// An untimed block: no usable timestamps → unknown duration/tokens/cost (absence ≠ zero).
const untimed = (over = {}) =>
  timed({
    id: 's-2',
    block_start_ts: null,
    block_end_ts: null,
    duration_minutes: null,
    total_tokens: null,
    cost_total: null,
    cost_currency: null,
    confidence: 'low',
    reason: '3 traces with no usable timestamps — needs manual time',
    ...over,
  });

test('durationLabel: known → Xh Ym; unknown → "needs manual time", never a zero duration', () => {
  assert.equal(durationLabel(96), '1h 36m');
  assert.equal(durationLabel(null), 'needs manual time');
  assert.notEqual(durationLabel(null), '0h 0m');
});

test('tokens/cost unknown render "—", never "0" (absence ≠ zero)', () => {
  assert.equal(tokensLabel(null), '—');
  assert.equal(costLabel(null, null), '—');
  assert.equal(tokensLabel(12345), '12,345');
  assert.equal(costLabel(0.4231, 'USD'), '0.42 USD');
});

test('spanLabel: timed block shows the local HH:MM range; untimed shows "—"', () => {
  assert.equal(spanLabel('2026-06-20 09:12:00', '2026-06-20 10:48:00'), '09:12–10:48');
  assert.equal(spanLabel(null, null), '—');
});

test('timed row exposes Accept / Edit / Dismiss with the edit panel hidden by default', () => {
  const html = suggestionRow(timed());
  assert.match(html, /data-accept="s-1"/);
  assert.match(html, /data-edit="s-1"/);
  assert.match(html, /data-dismiss="s-1"/);
  // The edit panel exists but is collapsed until Edit is clicked.
  assert.match(html, /data-edit-panel="s-1" hidden/);
  assert.match(html, /data-accept-edited="s-1"/);
  // Secret-free body carries the aggregate figures, counts, health, confidence, and reason.
  assert.match(html, /1h 36m/);
  assert.match(html, /09:12–10:48/);
  assert.match(html, /4 traces · 2 sessions/);
  assert.match(html, /12,345 tok · 0\.42 USD/);
  assert.match(html, /healthy · high/);
});

test('untimed row cannot be accepted without an edited span — needs-manual-time, no plain Accept', () => {
  const html = suggestionRow(untimed());
  assert.match(html, /needs manual time/);
  // No plain Accept button (would error); only Accept-with-edits is offered.
  assert.doesNotMatch(html, /data-accept="s-2"/);
  assert.match(html, /data-accept-edited="s-2"/);
  // The edit panel is shown (not hidden) and its start/end inputs are required.
  assert.match(html, /data-edit-panel="s-2"><td/);
  assert.match(html, /data-edit-field="start_time"[^>]*required/);
  assert.match(html, /data-edit-field="end_time"[^>]*required/);
  // Tokens/cost still render "—", never "0".
  assert.match(html, /— tok · —/);
  assert.match(html, /Vire never invents a duration/);
});

test('suggestions are grouped by project with a project heading, sorted by date', () => {
  const html = suggestionGroups([
    timed({ id: 'a', project_id: 'p-1', project_name: 'Veronavi', date: '2026-06-20' }),
    timed({ id: 'b', project_id: 'p-1', project_name: 'Veronavi', date: '2026-06-18' }),
    timed({ id: 'c', project_id: 'p-2', project_name: 'Atlas', date: '2026-06-19' }),
  ]);
  assert.match(html, /<h2>Veronavi<\/h2>/);
  assert.match(html, /<h2>Atlas<\/h2>/);
  // Within Veronavi, the earlier date row precedes the later one.
  assert.ok(html.indexOf('data-sug-row="b"') < html.indexOf('data-sug-row="a"'));
});

test('unmapped evidence is surfaced with a Settings link — never dropped or zeroed', () => {
  const html = unmappedNotice([{ environment: 'veronavi', trace_count: 7 }]);
  assert.match(html, /Unmapped AI evidence/);
  assert.match(html, /<b>veronavi<\/b> — 7 traces with no project mapping/);
  assert.match(html, /data-goto-view="Settings"/);
  // Nothing unmapped → no notice.
  assert.equal(unmappedNotice([]), '');
});

test('empty state names the candidate causes — never a bare empty table', () => {
  const noneAtAll = suggestionsBody({ suggestions: [], unmapped: [] });
  assert.match(noneAtAll, /no AI evidence has been imported yet, or every suggestion has already been accepted or dismissed/);
  assert.match(noneAtAll, /Refresh suggestions/);
  // Evidence present but unmapped → the mapping cause + the unmapped notice.
  const unmappedOnly = suggestionsBody({ suggestions: [], unmapped: [{ environment: 'veronavi', trace_count: 3 }] });
  assert.match(unmappedOnly, /not mapped to a project yet/);
  assert.match(unmappedOnly, /Unmapped AI evidence/);
});

test('project names and reasons are escaped (no raw HTML injection)', () => {
  // Reason is rendered in the row; the project name is rendered in the group heading.
  const row = suggestionRow(timed({ reason: '<script>alert(1)</script>' }));
  assert.doesNotMatch(row, /<script>alert/);
  assert.match(row, /&lt;script&gt;/);
  const grouped = suggestionGroups([timed({ project_name: '<img src=x onerror=alert(1)>' })]);
  assert.doesNotMatch(grouped, /<img src=x/);
  assert.match(grouped, /&lt;img src=x/);
});

test('SEC-012 — render carries no secret/payload/prompt/credential material', () => {
  const html = suggestionsBody({ suggestions: [timed(), untimed()], unmapped: [{ environment: 'veronavi', trace_count: 2 }] });
  for (const needle of ['sk-', 'pk-lf-', 'Bearer', 'Authorization', 'oat01', 'payload', 'metadata', 'prompt']) {
    assert.ok(!html.includes(needle), `suggestions render must not contain ${needle}`);
  }
});
