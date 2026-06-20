import test from 'node:test';
import assert from 'node:assert/strict';
import { REPORT_RANGE_PRESETS, reportRange } from '../src/report-ranges.ts';

// Inclusive day count between two YYYY-MM-DD strings, parsed as LOCAL dates (no UTC parsing, which
// `new Date('YYYY-MM-DD')` would do). This is the independent invariant the off-by-one defect would break.
function inclusiveDays(start, end) {
  const [sy, sm, sd] = start.split('-').map(Number);
  const [ey, em, ed] = end.split('-').map(Number);
  const a = new Date(sy, sm - 1, sd);
  const b = new Date(ey, em - 1, ed);
  return Math.round((b - a) / 86400000) + 1;
}

test('presets are the ordered Last 7/14/30/90 day windows', () => {
  assert.deepEqual(
    REPORT_RANGE_PRESETS.map((p) => ({ key: p.key, label: p.label, days: p.days })),
    [
      { key: 'last_7d', label: 'Last 7 days', days: 7 },
      { key: 'last_14d', label: 'Last 14 days', days: 14 },
      { key: 'last_30d', label: 'Last 30 days', days: 30 },
      { key: 'last_90d', label: 'Last 90 days', days: 90 },
    ],
  );
});

test('reportRange returns an inclusive window ending today (fixed mid-month reference)', () => {
  const ref = new Date(2026, 5, 20); // local 2026-06-20, no time-zone ambiguity
  assert.deepEqual(reportRange(7, ref), { start: '2026-06-14', end: '2026-06-20' });
  assert.deepEqual(reportRange(14, ref), { start: '2026-06-07', end: '2026-06-20' });
  assert.deepEqual(reportRange(30, ref), { start: '2026-05-22', end: '2026-06-20' });
  assert.deepEqual(reportRange(90, ref), { start: '2026-03-23', end: '2026-06-20' });
});

test('every preset window is valid (start ≤ end) and exactly `days` calendar days inclusive', () => {
  const ref = new Date(2026, 5, 20);
  for (const p of REPORT_RANGE_PRESETS) {
    const { start, end } = reportRange(p.days, ref);
    assert.ok(start <= end, `${p.key}: start ${start} must be ≤ end ${end}`);
    assert.equal(end, '2026-06-20', `${p.key}: end is always today`);
    assert.equal(inclusiveDays(start, end), p.days, `${p.key}: window spans ${p.days} inclusive days`);
  }
});

test('a window crossing a month/year boundary stays correct in local time (no UTC off-by-one)', () => {
  // tasks.md boundary case: today = 2026-01-03, Last 7 days → start 2025-12-28 (crosses year boundary).
  const ref = new Date(2026, 0, 3); // local 2026-01-03
  assert.deepEqual(reportRange(7, ref), { start: '2025-12-28', end: '2026-01-03' });
  // Last 30 days from Jan 3 reaches back into the prior year as well.
  const last30 = reportRange(30, ref);
  assert.equal(last30.end, '2026-01-03');
  assert.equal(inclusiveDays(last30.start, last30.end), 30);
  assert.ok(last30.start < last30.end);
});

test('a one-day window collapses to today on both ends (inclusive semantics edge)', () => {
  const ref = new Date(2026, 5, 20);
  assert.deepEqual(reportRange(1, ref), { start: '2026-06-20', end: '2026-06-20' });
});
