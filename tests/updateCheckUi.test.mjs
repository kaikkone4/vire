import test from 'node:test';
import assert from 'node:assert/strict';
import {
  updateCheckIdle,
  updateCheckPending,
  updateCheckResult,
  updateCheckPanel,
} from '../src/update-check-ui.ts';

// --- idle state ---

test('idle: renders check and open buttons', () => {
  const html = updateCheckIdle();
  assert.ok(html.includes('id="checkForUpdates"'), 'check button present');
  assert.ok(html.includes('id="openReleasesPage"'), 'open releases button present');
});

// --- pending state ---

test('pending: check button is disabled', () => {
  const html = updateCheckPending();
  assert.ok(html.includes('disabled'), 'check button disabled during check');
  assert.ok(html.includes('id="openReleasesPage"'), 'open releases button still present');
});

// --- result states ---

test('result: up_to_date renders current version and no update copy', () => {
  const html = updateCheckResult({ status: 'up_to_date', current: '0.8.0' });
  assert.ok(html.includes('0.8.0'), 'current version shown');
  assert.ok(html.includes('up to date'), 'up-to-date copy');
  assert.ok(!html.includes('Update available'), 'no update-available copy');
  assert.ok(html.includes('id="checkForUpdates"'), 'check-again button present');
});

test('result: update_available renders latest version and open release button', () => {
  const html = updateCheckResult({
    status: 'update_available',
    current: '0.8.0',
    latest: '0.9.0',
    release_url: 'https://github.com/kaikkonen4/vire/releases/tag/v0.9.0',
  });
  assert.ok(html.includes('0.9.0'), 'latest version shown');
  assert.ok(html.includes('Update available'), 'update-available copy');
  assert.ok(html.includes('id="openReleasesSpecific"'), 'specific release button present');
  assert.ok(html.includes('id="checkForUpdates"'), 'check-again button present');
  assert.ok(html.includes('id="openReleasesPage"'), 'open releases index button present');
});

test('result: unknown renders fail-soft copy, no error modal implied', () => {
  const html = updateCheckResult({ status: 'unknown', reason: 'network error: timed out' });
  assert.ok(html.includes("Couldn't check"), 'fail-soft copy present');
  assert.ok(!html.includes('network error: timed out'), 'raw error reason not rendered to user');
  assert.ok(html.includes('id="checkForUpdates"'), 'check-again button present');
});

// --- XSS: escape caller-supplied text ---

test('result: up_to_date escapes version string', () => {
  const html = updateCheckResult({ status: 'up_to_date', current: '<script>x</script>' });
  assert.ok(!html.includes('<script>'), 'script tag escaped');
  assert.ok(html.includes('&lt;script&gt;'), 'escaped form present');
});

test('result: update_available escapes latest version', () => {
  const html = updateCheckResult({
    status: 'update_available',
    current: '0.8.0',
    latest: '<img onerror=x>',
    release_url: 'https://github.com/kaikkonen4/vire/releases/tag/safe',
  });
  assert.ok(!html.includes('<img'), 'img tag not unescaped in latest version');
});

test('result: update_available escapes release_url in data attribute', () => {
  // The release_url is placed in a data-release-url attribute — escapeHtml neutralises quotes.
  const html = updateCheckResult({
    status: 'update_available',
    current: '0.8.0',
    latest: '0.9.0',
    release_url: 'https://example.com/" onclick="evil()',
  });
  assert.ok(!html.includes('" onclick="evil()'), 'attribute injection escaped');
});

// --- panel wrapper ---

test('panel: null result renders idle state', () => {
  const html = updateCheckPanel(null);
  assert.ok(html.includes('id="updateCheckResult"'), 'result container present');
  assert.ok(html.includes('id="checkForUpdates"'), 'check button present');
  assert.ok(html.includes('<h2>App updates</h2>'), 'section heading present');
});

test('panel: pending=true renders disabled check button', () => {
  const html = updateCheckPanel(null, true);
  assert.ok(html.includes('disabled'), 'button disabled in pending state');
});

test('panel: with result renders result block', () => {
  const html = updateCheckPanel({ status: 'up_to_date', current: '0.8.0' });
  assert.ok(html.includes('up to date'), 'result rendered inside panel');
});

test('panel: privacy copy mentions no app data sent', () => {
  const html = updateCheckPanel(null);
  assert.ok(html.includes('No app data is sent'), 'privacy reassurance present');
});
