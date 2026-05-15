import test from 'node:test';
import assert from 'node:assert/strict';
import { escapeHtml } from '../src/html.ts';

test('escapeHtml neutralizes stored DOM XSS payload characters in text and attributes', () => {
  const payload = `"><img src=x onerror="globalThis.__xss=1"><script>alert('x')</script>&`;
  const escaped = escapeHtml(payload);
  assert.equal(escaped, '&quot;&gt;&lt;img src=x onerror=&quot;globalThis.__xss=1&quot;&gt;&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;&amp;');
  assert.doesNotMatch(escaped, /<script|<img|onerror="/);
});
