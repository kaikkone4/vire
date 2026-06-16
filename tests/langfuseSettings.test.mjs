import test from 'node:test';
import assert from 'node:assert/strict';
import { environmentsToCsv, parseEnvironmentsCsv, secretStateLabel } from '../src/langfuse-settings.ts';

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
