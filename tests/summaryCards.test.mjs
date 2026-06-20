import test from 'node:test';
import assert from 'node:assert/strict';
import { summaryCards } from '../src/summary-cards.ts';

// A summary card shaped like the backend `SummaryRow` (lib.rs), snake_case fields. duration_minutes is
// human/manual time; ai_minutes is accepted AI-suggested time kept distinct; ai_cost_* are the new
// TASK-034 B fields (NULL when no accepted AI entry in range carries a cost).
const card = (over = {}) => ({
  project_id: 'p-1',
  project_name: 'Veronavi',
  duration_minutes: 120,
  ai_minutes: 90,
  ai_cost_total: 0.4231,
  ai_cost_currency: 'USD',
  ...over,
});

test('B5: AI-suggested sub-line carries cost on the project card', () => {
  const html = summaryCards([card()], false, 'No report data for this range.');
  assert.match(html, /<span>Veronavi<\/span>/);
  assert.match(html, /<strong>2h 0m<\/strong>/); // human headline
  assert.match(html, /AI-suggested 1h 30m · 0\.42 USD/);
});

test('B5: AI cost absent renders "—", never "0" (absence ≠ zero)', () => {
  const html = summaryCards([card({ ai_cost_total: null, ai_cost_currency: null })], false, 'empty');
  assert.match(html, /AI-suggested 1h 30m · —/);
  assert.doesNotMatch(html, /AI-suggested 1h 30m · 0/);
});

test('B5: a manual-only project (no AI time) shows no AI sub-line at all', () => {
  const html = summaryCards([card({ ai_minutes: 0, ai_cost_total: null, ai_cost_currency: null })], false, 'empty');
  assert.match(html, /<strong>2h 0m<\/strong>/);
  assert.doesNotMatch(html, /AI-suggested/);
});

test('B5: lead "Total tracked" card aggregates AI cost across projects', () => {
  const html = summaryCards(
    [card({ ai_cost_total: 1.0, ai_cost_currency: 'USD' }), card({ project_id: 'p-2', project_name: 'Atlas', duration_minutes: 60, ai_minutes: 30, ai_cost_total: 0.5, ai_cost_currency: 'USD' })],
    true,
    'empty',
  );
  // Lead card: human total 3h 0m; AI total 2h 0m; cost summed 1.50 USD; reported separately.
  assert.match(html, /<span>Total tracked<\/span>/);
  assert.match(html, /<strong>3h 0m<\/strong>/);
  assert.match(html, /AI-suggested 2h 0m · 1\.50 USD, reported separately/);
});

test('B5: mixed-currency aggregate renders "—" (not source-derivable as a single figure)', () => {
  const html = summaryCards(
    [card({ ai_cost_total: 1.0, ai_cost_currency: 'USD' }), card({ project_id: 'p-2', project_name: 'Atlas', ai_cost_total: 2.0, ai_cost_currency: 'EUR' })],
    true,
    'empty',
  );
  assert.match(html, /AI-suggested .* · —, reported separately/);
});

test('B5: no cards → the empty message, never a bare grid or "0"', () => {
  const html = summaryCards([], false, 'No report data for this range.');
  assert.match(html, /No report data for this range\./);
  assert.doesNotMatch(html, /AI-suggested/);
});

test('B5: project name is escaped (no raw HTML injection)', () => {
  const html = summaryCards([card({ project_name: '<img src=x onerror=alert(1)>' })], false, 'empty');
  assert.doesNotMatch(html, /<img src=x/);
  assert.match(html, /&lt;img src=x/);
});
