import test from 'node:test';
import assert from 'node:assert/strict';
import { localDateInputValue } from '../src/date.ts';

test('localDateInputValue uses the local calendar date instead of UTC ISO date', () => {
  const previousTimezone = process.env.TZ;
  process.env.TZ = 'Europe/Helsinki';
  try {
    const justAfterLocalMidnight = new Date(2026, 0, 1, 0, 30, 0);
    assert.equal(localDateInputValue(justAfterLocalMidnight), '2026-01-01');
    assert.equal(justAfterLocalMidnight.toISOString().slice(0, 10), '2025-12-31');
  } finally {
    if (previousTimezone === undefined) {
      delete process.env.TZ;
    } else {
      process.env.TZ = previousTimezone;
    }
  }
});
