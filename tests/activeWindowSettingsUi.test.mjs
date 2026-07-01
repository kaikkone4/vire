import test from 'node:test';
import assert from 'node:assert/strict';
import {
  CAPTURE_BOUNDS,
  validateCaptureInput,
  healthMarkerLabel,
  captureStatusBlock,
  privacyTable,
  capturePanel,
  captureBanner,
} from '../src/active-window-settings-ui.ts';

// A valid baseline view (macOS, capture on, one healthy sample) reused across panel/status tests.
const baseInput = {
  capture_enabled: true,
  sample_seconds: 5,
  idle_candidate_seconds: 60,
  idle_away_seconds: 300,
  retention_days: 30,
};
const emptyStatus = {
  last_sample_ts: null,
  samples_today: 0,
  evidence_blocks_retained: 0,
  open_health: [],
  recent_health: [],
};
const view = (over = {}) => ({
  platform_supported: true,
  capture_enabled: true,
  sample_seconds: 5,
  idle_candidate_seconds: 60,
  idle_away_seconds: 300,
  retention_days: 30,
  title_mode: 'redacted',
  status: emptyStatus,
  ...over,
});

// ---- validation (mirrors backend settings_api::validate bounds) ----------------------------------

test('bounds mirror the backend contract', () => {
  assert.deepEqual(CAPTURE_BOUNDS, {
    sampleMin: 1,
    sampleMax: 3600,
    idleCandidateMin: 1,
    idleMax: 86_400,
    retentionMin: 1,
    retentionMax: 3650,
  });
});

test('a valid input passes with no error', () => {
  assert.equal(validateCaptureInput(baseInput), null);
});

test('sample interval of 0 is rejected before the IPC call', () => {
  const err = validateCaptureInput({ ...baseInput, sample_seconds: 0 });
  assert.match(err ?? '', /Sample interval must be between 1 and 3600/);
});

test('sample interval above the cap is rejected', () => {
  assert.ok(validateCaptureInput({ ...baseInput, sample_seconds: 3601 }));
});

test('idle-candidate below 1 is rejected', () => {
  const err = validateCaptureInput({ ...baseInput, idle_candidate_seconds: 0, idle_away_seconds: 300 });
  assert.match(err ?? '', /Idle-candidate threshold/);
});

test('away not greater than idle-candidate is rejected (ordering invariant)', () => {
  const err = validateCaptureInput({ ...baseInput, idle_candidate_seconds: 60, idle_away_seconds: 60 });
  assert.match(err ?? '', /Away threshold must be greater/);
});

test('away above the cap is rejected', () => {
  const err = validateCaptureInput({ ...baseInput, idle_candidate_seconds: 60, idle_away_seconds: 86_401 });
  assert.match(err ?? '', /Away threshold must be at most 86400/);
});

test('retention of 0 and above the cap are rejected', () => {
  assert.ok(validateCaptureInput({ ...baseInput, retention_days: 0 }));
  assert.ok(validateCaptureInput({ ...baseInput, retention_days: 3651 }));
});

test('non-integer / empty (NaN) fields are rejected rather than slipping past range checks', () => {
  const err = validateCaptureInput({ ...baseInput, sample_seconds: Number.NaN });
  assert.match(err ?? '', /whole numbers/);
});

// ---- status/health copy — every gap explained, never a silent empty box --------------------------

test('status: off state names that nothing is being collected', () => {
  const html = captureStatusBlock(view({ capture_enabled: false }));
  assert.match(html, /off/);
  assert.match(html, /No active-window or idle data is being collected/);
});

test('status: enabled but no sample yet says awaiting first sample (last-sample absent, not an error)', () => {
  const html = captureStatusBlock(view({ capture_enabled: true, status: emptyStatus }));
  assert.match(html, /awaiting the first sample/);
});

test('status: healthy shows last sample and counts', () => {
  const html = captureStatusBlock(
    view({
      status: { ...emptyStatus, last_sample_ts: '2026-07-01 14:02:00', samples_today: 12, evidence_blocks_retained: 4 },
    }),
  );
  assert.match(html, /healthy/);
  assert.match(html, /2026-07-01 14:02:00/);
  assert.match(html, /Samples today: <b>12<\/b>/);
  assert.match(html, /Evidence blocks retained: <b>4<\/b>/);
});

test('status: degraded names the open state and when it began (with cause), not a blank box', () => {
  const html = captureStatusBlock(
    view({
      status: {
        ...emptyStatus,
        open_health: [{ state: 'no_gui_session', since_or_start_ts: '2026-07-01 14:02:00', detail: null }],
      },
    }),
  );
  assert.match(html, /degraded/);
  assert.match(html, /no_gui_session since 2026-07-01 14:02:00/);
});

test('status: macOS-only note when the platform is unsupported', () => {
  const html = captureStatusBlock(view({ platform_supported: false }));
  assert.match(html, /available on macOS only/);
});

test('status: recent-health history is surfaced when present', () => {
  const html = captureStatusBlock(
    view({
      status: {
        ...emptyStatus,
        last_sample_ts: '2026-07-01 14:02:00',
        recent_health: [{ state: 'sampling_gap', since_or_start_ts: '2026-07-01 09:00:00', detail: null }],
      },
    }),
  );
  assert.match(html, /Recent capture health \(1\)/);
  assert.match(html, /sampling_gap since 2026-07-01 09:00:00/);
});

test('healthMarkerLabel escapes a hostile detail/state code', () => {
  const label = healthMarkerLabel({ state: '<img onerror=x>', since_or_start_ts: 't', detail: '<script>' });
  const html = captureStatusBlock(view({ status: { ...emptyStatus, open_health: [{ state: '<img onerror=x>', since_or_start_ts: 't', detail: '<script>' }] } }));
  assert.ok(label.includes('<img'), 'raw label is pre-escape (escaping happens at render)');
  assert.ok(!html.includes('<img onerror'), 'rendered state is escaped');
  assert.ok(!html.includes('<script>'), 'rendered detail is escaped');
});

// ---- privacy table -------------------------------------------------------------------------------

test('privacy table lists captured vs never-captured truthfully', () => {
  const html = privacyTable();
  assert.match(html, /Frontmost app bundle id/);
  assert.match(html, /Coarse idle state/);
  assert.match(html, /Window titles/);
  assert.match(html, /Keystrokes/);
  assert.match(html, /secrets/);
});

// ---- full panel ----------------------------------------------------------------------------------

test('panel renders the toggle, the four numeric knobs, save, status, and privacy table', () => {
  const html = capturePanel(view());
  assert.match(html, /id="captureForm"/);
  assert.match(html, /name="capture_enabled"/);
  assert.match(html, /name="sample_seconds"/);
  assert.match(html, /name="idle_candidate_seconds"/);
  assert.match(html, /name="idle_away_seconds"/);
  assert.match(html, /name="retention_days"/);
  assert.match(html, /Save capture settings/);
  assert.match(html, /privacy-table/);
  assert.match(html, /Capture status/);
});

test('panel: no window-title opt-in is offered; title_mode is shown read-only', () => {
  const html = capturePanel(view());
  assert.match(html, /Window titles: <b>never captured<\/b>/);
  assert.match(html, /title_mode = redacted/);
  assert.ok(!/name="title_mode"/.test(html), 'no title_mode input control exists');
  assert.ok(!/store.{0,20}title/i.test(html) || /never/i.test(html), 'no "store titles" affordance');
});

test('panel: checkbox reflects the enabled state', () => {
  assert.match(capturePanel(view({ capture_enabled: true })), /name="capture_enabled" checked/);
  assert.ok(!/name="capture_enabled" checked/.test(capturePanel(view({ capture_enabled: false }))));
});

test('panel: non-macOS disables every control and states macOS-only', () => {
  const html = capturePanel(view({ platform_supported: false }));
  assert.match(html, /available on macOS only/);
  // toggle + four inputs + save button all disabled
  assert.equal((html.match(/disabled/g) ?? []).length, 6);
});

// ---- banner (truthful, replaces the hard-coded "capture deferred" copy) ---------------------------

test('banner: when capture is ON it says active-app + idle ARE recorded, never denies collection', () => {
  const html = captureBanner(view({ capture_enabled: true }));
  assert.match(html, /Active-window capture is on/);
  assert.match(html, /recorded locally/);
  assert.ok(!/does not collect/i.test(html));
  assert.ok(!/not implemented/i.test(html));
});

test('banner: when OFF it says nothing is collected and can be enabled', () => {
  const html = captureBanner(view({ capture_enabled: false }));
  assert.match(html, /off/);
  assert.match(html, /enable it in Settings/i);
});

test('banner: non-macOS says macOS-only', () => {
  assert.match(captureBanner(view({ platform_supported: false })), /macOS only/);
});

test('banner: null view is an explicit "unavailable", never a false claim', () => {
  const html = captureBanner(null);
  assert.match(html, /unavailable/);
});
