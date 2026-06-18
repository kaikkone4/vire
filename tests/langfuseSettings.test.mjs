import test from 'node:test';
import assert from 'node:assert/strict';
import {
  environmentsToCsv,
  parseEnvironmentsCsv,
  secretStateLabel,
  testConnectionDisabledReason,
  DEFAULT_IMPORT_RANGE,
  CUSTOM_RANGE_PRESET,
  IMPORT_RANGE_PRESETS,
  canonicalImportRange,
  parseImportRangeControl,
  importRangeLabel,
} from '../src/langfuse-settings.ts';

test('environments round-trip through the CSV field without surfacing blanks or stray whitespace', () => {
  assert.equal(environmentsToCsv(['vire', 'staging']), 'vire, staging');
  assert.deepEqual(parseEnvironmentsCsv('vire, staging'), ['vire', 'staging']);
  assert.deepEqual(parseEnvironmentsCsv('  vire ,, staging ,  '), ['vire', 'staging']);
  assert.deepEqual(parseEnvironmentsCsv(''), []);
});

test('secretStateLabel exposes only presence, never a stored secret value', () => {
  assert.equal(secretStateLabel(true), 'set');
  assert.equal(secretStateLabel(false), 'not set');
});

test('Test connection is blocked in the UI while the integration is disabled', () => {
  // Enabled → no reason (button stays clickable).
  assert.equal(testConnectionDisabledReason(true), '');
  // Disabled → a non-empty tooltip reason; the button is rendered disabled so no probe can fire.
  const reason = testConnectionDisabledReason(false);
  assert.ok(reason.length > 0);
  assert.match(reason, /enable/i);
});

// ---- D3: import-range control canonicalization ------------------------------

test('the default range is last_30d and the picker offers it plus a custom option', () => {
  assert.equal(DEFAULT_IMPORT_RANGE, 'last_30d');
  const values = IMPORT_RANGE_PRESETS.map((p) => p.value);
  assert.deepEqual(values, ['last_7d', 'last_30d', 'last_90d', 'all', 'custom']);
  assert.equal(CUSTOM_RANGE_PRESET, 'custom');
});

test('keyword presets canonicalize straight through to the backend vocabulary', () => {
  assert.equal(canonicalImportRange('last_7d', ''), 'last_7d');
  assert.equal(canonicalImportRange('last_30d', ''), 'last_30d');
  assert.equal(canonicalImportRange('last_90d', ''), 'last_90d');
  assert.equal(canonicalImportRange('all', ''), 'all');
});

test('a custom date canonicalizes to the UTC RFC3339 since:<floor> the backend normalizes to', () => {
  assert.equal(canonicalImportRange('custom', '2026-01-15'), 'since:2026-01-15T00:00:00Z');
  // whitespace tolerated
  assert.equal(canonicalImportRange('custom', ' 2026-01-15 '), 'since:2026-01-15T00:00:00Z');
});

test('an empty or malformed custom date falls back to the default rather than sending garbage', () => {
  assert.equal(canonicalImportRange('custom', ''), DEFAULT_IMPORT_RANGE);
  assert.equal(canonicalImportRange('custom', 'not-a-date'), DEFAULT_IMPORT_RANGE);
  assert.equal(canonicalImportRange('custom', '2026-1-5'), DEFAULT_IMPORT_RANGE);
  // An unknown preset also resolves to the default.
  assert.equal(canonicalImportRange('bogus', ''), DEFAULT_IMPORT_RANGE);
});

test('parsing a stored canonical value seeds the right preset and date', () => {
  assert.deepEqual(parseImportRangeControl('last_7d'), { preset: 'last_7d', sinceDate: '' });
  assert.deepEqual(parseImportRangeControl('all'), { preset: 'all', sinceDate: '' });
  assert.deepEqual(parseImportRangeControl('since:2026-01-15T00:00:00Z'), {
    preset: 'custom',
    sinceDate: '2026-01-15',
  });
  // Unknown/malformed stored value → default preset (mirrors the backend fallback).
  assert.deepEqual(parseImportRangeControl('garbage'), { preset: DEFAULT_IMPORT_RANGE, sinceDate: '' });
});

test('canonical → control → canonical round-trips for every supported shape', () => {
  for (const value of ['last_7d', 'last_30d', 'last_90d', 'all', 'since:2026-03-01T00:00:00Z']) {
    const { preset, sinceDate } = parseImportRangeControl(value);
    assert.equal(canonicalImportRange(preset, sinceDate), value);
  }
});

test('importRangeLabel is human-readable and never echoes a secret', () => {
  assert.equal(importRangeLabel('last_30d'), 'Last 30 days');
  assert.equal(importRangeLabel('all'), 'All history');
  assert.equal(importRangeLabel('since:2026-01-15T00:00:00Z'), 'Since 2026-01-15');
  // The range value is non-secret by construction, but assert no credential markers leak regardless.
  for (const value of ['last_7d', 'all', 'since:2026-01-15T00:00:00Z']) {
    const label = importRangeLabel(value);
    for (const needle of ['sk-', 'pk-', 'Bearer', 'Authorization']) {
      assert.ok(!label.includes(needle), `range label must not contain ${needle}`);
    }
  }
});
