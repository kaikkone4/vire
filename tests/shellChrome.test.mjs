import test from 'node:test';
import assert from 'node:assert/strict';
import { titlebar } from '../src/shell-chrome.ts';

test('titlebar renders the brand and version but never the fake traffic-light cluster (E1/E5)', () => {
  const html = titlebar('Vire', 'v0.1 local');
  assert.match(html, /class="titlebar"/);
  assert.match(html, /<b>Vire<\/b>/);
  assert.match(html, /<code>v0\.1 local<\/code>/);
  // The fake macOS traffic lights are gone — native window decorations provide the real controls.
  assert.doesNotMatch(html, /traffic/);
  assert.doesNotMatch(html, /<span>/);
});

test('titlebar escapes its inputs so a hostile brand/version string cannot inject markup', () => {
  const html = titlebar('<img src=x onerror=alert(1)>', '"><script>');
  assert.doesNotMatch(html, /<img/);
  assert.doesNotMatch(html, /<script>/);
  assert.match(html, /&lt;img/);
});
