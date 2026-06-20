import test from 'node:test';
import assert from 'node:assert/strict';
import {
  durationLabel,
  spanLabel,
  tokensLabel,
  costLabel,
  addMinutesHHMM,
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

// A timed block whose start and end fall in the same clock minute (DEC-034). The engine still derives
// a duration (>= 1 min); accept rounds the end up to start + duration so the stored span is never zero.
const sameMinute = (over = {}) =>
  timed({
    id: 's-3',
    block_start_ts: '2026-06-20 09:00:10',
    block_end_ts: '2026-06-20 09:00:50',
    duration_minutes: 1,
    ...over,
  });

test('addMinutesHHMM mirrors the backend bump: adds minutes, clamps a midnight cross to 23:59', () => {
  assert.equal(addMinutesHHMM('09:00', 1), '09:01');
  assert.equal(addMinutesHHMM('09:12', 96), '10:48');
  // A bump that would cross midnight is clamped to the same day's 23:59 (matches lib.rs).
  assert.equal(addMinutesHHMM('23:59', 1), '23:59');
});

test('A3: same-minute block pre-fills the edit End default to start + duration (never == start)', () => {
  const html = suggestionRow(sameMinute());
  // Start renders the block minute; End is bumped to start + duration (>= 1) so the editable span the
  // user sees equals what accept will store — strictly after Start, never a zero span.
  assert.match(html, /data-edit-field="start_time" type="time" value="09:00"/);
  assert.match(html, /data-edit-field="end_time" type="time" value="09:01"/);
  assert.doesNotMatch(html, /data-edit-field="end_time" type="time" value="09:00"/);
});

test('A3: a normal multi-minute block keeps its real End — no bump applied', () => {
  const html = suggestionRow(timed());
  assert.match(html, /data-edit-field="start_time" type="time" value="09:12"/);
  assert.match(html, /data-edit-field="end_time" type="time" value="10:48"/);
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
  // C2: the "not auto-trackable" state is surfaced on the row itself (badge), not only in the panel.
  // It sits in the summary <tr>, before the always-open edit panel.
  assert.match(html, /not auto-trackable — add time manually/);
  assert.ok(html.indexOf('not auto-trackable') < html.indexOf('data-edit-panel="s-2"'));
});

test('C2: a timed row carries no not-auto-trackable badge (it is trackable)', () => {
  const html = suggestionRow(timed());
  assert.doesNotMatch(html, /not auto-trackable/);
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

test('C1: unmapped evidence is surfaced with a Settings link and the trackability copy', () => {
  const html = unmappedNotice([{ environment: 'veronavi', trace_count: 7 }]);
  assert.match(html, /Unmapped AI evidence/);
  assert.match(html, /<b>veronavi<\/b> — 7 traces with no project mapping/);
  // C1: tightened copy names the cause + the action.
  assert.match(html, /not trackable until mapped/);
  assert.match(html, /Map in Settings/);
  assert.match(html, /data-goto-view="Settings"/);
  // Nothing unmapped → no notice.
  assert.equal(unmappedNotice([]), '');
});

test('C3/C4: empty state names every candidate cause with an action — never a bare empty table', () => {
  const noneAtAll = suggestionsBody({ suggestions: [], unmapped: [] });
  // Nothing imported / all decided, with its action.
  assert.match(noneAtAll, /Nothing imported yet, or all decided/);
  assert.match(noneAtAll, /Open Settings to import/);
  // Untimed / not-auto-trackable cause is always named (a cause of "no actionable suggestion").
  assert.match(noneAtAll, /Evidence has no usable time/);
  assert.match(noneAtAll, /not auto-trackable/);
  assert.match(noneAtAll, /Refresh suggestions/);
  // Default (source healthy) → no source-down cause and no "0".
  assert.doesNotMatch(noneAtAll, /unavailable or disabled/);
  assert.ok(!noneAtAll.includes('>0<') && !/\b0 suggestions\b/.test(noneAtAll));

  // Evidence present but unmapped → the mapping cause + the unmapped notice.
  const unmappedOnly = suggestionsBody({ suggestions: [], unmapped: [{ environment: 'veronavi', trace_count: 3 }] });
  assert.match(unmappedOnly, /isn't mapped to a project/);
  assert.match(unmappedOnly, /not trackable until mapped/);
  assert.match(unmappedOnly, /Unmapped AI evidence/);

  // Source disabled/down is named as a cause only when the view reports it degraded.
  const sourceDown = suggestionsBody({ suggestions: [], unmapped: [] }, { sourceDegraded: true });
  assert.match(sourceDown, /unavailable or disabled/);
  assert.match(sourceDown, /never zero/);
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
