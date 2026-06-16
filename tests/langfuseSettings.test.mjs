import test from 'node:test';
import assert from 'node:assert/strict';
import { environmentsToCsv, parseEnvironmentsCsv, secretStateLabel, testConnectionDisabledReason } from '../src/langfuse-settings.ts';

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
