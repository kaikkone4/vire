import test from 'node:test';
import assert from 'node:assert/strict';
import { nextScrollTop } from '../src/scroll.ts';

// TASK-031: shell() re-renders the whole tree, recreating the <main> scroll container. nextScrollTop
// decides where the new <main> lands: keep the user's place on a same-view re-render, top otherwise.

test('same-view re-render keeps the previous scroll position', () => {
  assert.equal(nextScrollTop(true, 420), 420);
  assert.equal(nextScrollTop(true, 0), 0);
});

test('view change resets to the top regardless of previous scroll', () => {
  assert.equal(nextScrollTop(false, 420), 0);
  assert.equal(nextScrollTop(false, 0), 0);
});
