import test from 'node:test';
import assert from 'node:assert/strict';
import { optionalText } from '../src/forms.ts';

test('optionalText maps blank optional fields to null for Option payloads', () => {
  assert.equal(optionalText(''), null);
  assert.equal(optionalText('   '), null);
  assert.equal(optionalText(null), null);
  assert.equal(optionalText(' billable note '), 'billable note');
});
