// Pure builders for the Settings "Active-window capture" panel and the truthful capture banner
// (TASK-056 C). No DOM or IPC here so the markup, validation, and status/health copy stay
// unit-testable; the click/submit handlers that invoke get_/set_active_window_capture_settings live
// in main.ts bindCaptureSettings(). This surface reads and writes ONLY the five capture `settings`
// knobs — it requests no permission, exposes no window-title opt-in, and carries only bundle
// ids/names, coarse idle states, timestamps, counts, and bounded coarse state/detail codes.

import { escapeHtml as esc } from './html';

// Mirrors the backend `HealthMarker` (active_window/model.rs) — Tauri serializes snake_case as-is.
export type HealthMarker = { state: string; since_or_start_ts: string; detail: string | null };

// Mirrors the backend `CaptureStatusView`.
export type CaptureStatus = {
  last_sample_ts: string | null;
  samples_today: number;
  evidence_blocks_retained: number;
  open_health: HealthMarker[];
  recent_health: HealthMarker[];
};

// Mirrors the backend `CaptureSettingsView` returned by get_/set_active_window_capture_settings.
export type CaptureSettingsView = {
  platform_supported: boolean;
  capture_enabled: boolean;
  sample_seconds: number;
  idle_candidate_seconds: number;
  idle_away_seconds: number;
  retention_days: number;
  title_mode: string; // always "redacted" — informational, never user-editable
  status: CaptureStatus;
};

// Mirrors the backend `CaptureSettingsInput` accepted by set_active_window_capture_settings. NO
// title_mode — the backend rejects any non-allowlisted key, and this UI offers no title control.
export type CaptureSettingsInput = {
  capture_enabled: boolean;
  sample_seconds: number;
  idle_candidate_seconds: number;
  idle_away_seconds: number;
  retention_days: number;
};

// Safe bounds — MUST mirror the backend `settings_api::validate` bounds exactly so inline validation
// rejects the same inputs the backend would, with a matching message.
export const CAPTURE_BOUNDS = {
  sampleMin: 1,
  sampleMax: 3600,
  idleCandidateMin: 1,
  idleMax: 86_400,
  retentionMin: 1,
  retentionMax: 3650,
} as const;

// Inline pre-flight validation. Returns a clear error string (null when valid) so the panel can
// reject a bad write BEFORE the IPC call. Mirrors the backend messages; the backend re-validates
// authoritatively. NaN/non-integer inputs (empty fields) are rejected explicitly — otherwise a NaN
// would slip past the numeric range comparisons.
export function validateCaptureInput(input: CaptureSettingsInput): string | null {
  const ints = [
    input.sample_seconds,
    input.idle_candidate_seconds,
    input.idle_away_seconds,
    input.retention_days,
  ];
  if (ints.some((n) => !Number.isInteger(n))) {
    return 'Sample, idle, away, and retention must be whole numbers.';
  }
  if (input.sample_seconds < CAPTURE_BOUNDS.sampleMin || input.sample_seconds > CAPTURE_BOUNDS.sampleMax) {
    return `Sample interval must be between ${CAPTURE_BOUNDS.sampleMin} and ${CAPTURE_BOUNDS.sampleMax} seconds.`;
  }
  if (
    input.idle_candidate_seconds < CAPTURE_BOUNDS.idleCandidateMin ||
    input.idle_candidate_seconds > CAPTURE_BOUNDS.idleMax
  ) {
    return `Idle-candidate threshold must be between ${CAPTURE_BOUNDS.idleCandidateMin} and ${CAPTURE_BOUNDS.idleMax} seconds.`;
  }
  if (input.idle_away_seconds <= input.idle_candidate_seconds) {
    return 'Away threshold must be greater than the idle-candidate threshold.';
  }
  if (input.idle_away_seconds > CAPTURE_BOUNDS.idleMax) {
    return `Away threshold must be at most ${CAPTURE_BOUNDS.idleMax} seconds.`;
  }
  if (input.retention_days < CAPTURE_BOUNDS.retentionMin || input.retention_days > CAPTURE_BOUNDS.retentionMax) {
    return `Retention must be between ${CAPTURE_BOUNDS.retentionMin} and ${CAPTURE_BOUNDS.retentionMax} days.`;
  }
  return null;
}

// A single degraded-capture marker rendered as "<state> since <ts> (<detail>)". Every value is a
// bounded coarse code / timestamp (never a title, path, or secret) but is escaped regardless.
export function healthMarkerLabel(m: HealthMarker): string {
  const detail = m.detail ? ` (${m.detail})` : '';
  return `${m.state} since ${m.since_or_start_ts}${detail}`;
}

// The status/health readout — every gap is explained, never a silent empty box (TASK-002 §4):
// macOS-only / off / degraded-with-cause / on-awaiting-first-sample / healthy, plus a collapsed
// recent-health history when present.
export function captureStatusBlock(view: CaptureSettingsView): string {
  if (!view.platform_supported) {
    return `<p class="hint">Active-window capture is available on macOS only.</p>`;
  }
  const recent = view.status.recent_health.length
    ? `<details class="capture-recent"><summary>Recent capture health (${view.status.recent_health.length})</summary><ul>${view.status.recent_health
        .map((m) => `<li>${esc(healthMarkerLabel(m))}</li>`)
        .join('')}</ul></details>`
    : '';
  return statusMain(view) + recent;
}

function statusMain(view: CaptureSettingsView): string {
  const s = view.status;
  if (!view.capture_enabled) {
    return `<p>Capture is <b>off</b>. No active-window or idle data is being collected. Enable it above to start.</p>`;
  }
  if (s.open_health.length) {
    const items = s.open_health.map((m) => `<li>${esc(healthMarkerLabel(m))}</li>`).join('');
    return (
      `<p>Capture is enabled but <b>degraded</b> — the following gaps are open:</p><ul>${items}</ul>` +
      `<p>Samples today: <b>${s.samples_today}</b> · Evidence blocks retained: <b>${s.evidence_blocks_retained}</b>.</p>`
    );
  }
  if (s.last_sample_ts == null) {
    return `<p>Capture is <b>on</b> and awaiting the first sample.</p>`;
  }
  return (
    `<p>Capture is <b>healthy</b>. Last sample: <b>${esc(s.last_sample_ts)}</b> · ` +
    `Samples today: <b>${s.samples_today}</b> · Evidence blocks retained: <b>${s.evidence_blocks_retained}</b>.</p>`
  );
}

// Plain-language, source-of-truth privacy table (Captured vs Never captured). Mirrors
// docs/active-window-capture.md §"Privacy posture" so the doc and the UI never drift apart.
export function privacyTable(): string {
  const rows: Array<[string, string]> = [
    ['Frontmost app bundle id + display name', 'Window titles'],
    ['Coarse idle state (active / idle_candidate / away)', 'Accessibility tree, screen pixels'],
    ['Observation timestamp', 'Keystrokes, mouse content, clipboard'],
    ['', 'URLs, file paths, terminal command bodies'],
    ['', 'Prompts / responses, secrets'],
  ];
  const body = rows.map(([c, n]) => `<tr><td>${esc(c)}</td><td>${esc(n)}</td></tr>`).join('');
  return `<table class="privacy-table"><tr><th>Captured</th><th>Never captured</th></tr>${body}</table>`;
}

function numberField(label: string, name: string, value: number, min: number, max: number, dis: string): string {
  return `<label>${esc(label)}<input name="${name}" type="number" min="${min}" max="${max}" step="1" value="${esc(String(value))}"${dis} required></label>`;
}

// The full "Active-window capture" Settings panel. On non-macOS the controls are disabled and the
// panel states capture is macOS-only. Renders the four knobs, the enable toggle, the status/health
// readout, and the privacy table. Pure HTML — the submit handler lives in main.ts.
export function capturePanel(view: CaptureSettingsView): string {
  const dis = view.platform_supported ? '' : ' disabled';
  const macNote = view.platform_supported
    ? ''
    : `<p class="hint">Active-window capture is available on macOS only. The controls below are read-only on this platform.</p>`;
  const checked = view.capture_enabled ? ' checked' : '';
  return (
    `<section class="panel" id="capturePanel"><h2>Active-window capture</h2>` +
    `<p>Optionally sample the frontmost app and a coarse idle state at a fixed cadence, stored only in ` +
    `local SQLite on this Mac. This uses zero macOS permissions, is <b>off by default</b>, and a change ` +
    `applies within a few seconds on the next sample — no app restart.</p>` +
    macNote +
    `<form id="captureForm" class="lf-form">` +
    `<label class="switch"><input type="checkbox" name="capture_enabled"${checked}${dis}><span>Enable active-window capture</span></label>` +
    numberField('Sample interval (seconds)', 'sample_seconds', view.sample_seconds, CAPTURE_BOUNDS.sampleMin, CAPTURE_BOUNDS.sampleMax, dis) +
    numberField('Idle-candidate threshold (seconds)', 'idle_candidate_seconds', view.idle_candidate_seconds, CAPTURE_BOUNDS.idleCandidateMin, CAPTURE_BOUNDS.idleMax, dis) +
    numberField('Away threshold (seconds)', 'idle_away_seconds', view.idle_away_seconds, CAPTURE_BOUNDS.idleCandidateMin + 1, CAPTURE_BOUNDS.idleMax, dis) +
    numberField('Retention (days)', 'retention_days', view.retention_days, CAPTURE_BOUNDS.retentionMin, CAPTURE_BOUNDS.retentionMax, dis) +
    `<button${dis}>Save capture settings</button>` +
    `</form>` +
    `<h3>Capture status</h3><div aria-live="polite">${captureStatusBlock(view)}</div>` +
    `<h3>What is and isn't captured</h3>${privacyTable()}` +
    `<p class="hint">Window titles: <b>never captured</b> (fixed <code>title_mode = ${esc(view.title_mode)}</code>; ` +
    `storing titles would require an Accessibility grant this feature does not request). Zero macOS ` +
    `permissions are requested; data is stored only in local SQLite and never sent over the network.</p>` +
    `</section>`
  );
}

// The Today/Settings capture banner, driven by the real setting (replaces the hard-coded v0.1
// "capture deferred" string). Truthful in every state — when capture is on it says active-app and
// idle state ARE being recorded; it never claims active windows/idle state are not collected.
export function captureBanner(view: CaptureSettingsView | null): string {
  if (view == null) {
    return `<section class="banner"><b>Active-window capture</b><p>Capture status is unavailable right now. Open Settings to view or change it.</p></section>`;
  }
  if (!view.platform_supported) {
    return `<section class="banner"><b>Active-window capture — macOS only</b><p>Automatic active-window capture is available on macOS only; no capture runs on this platform. Screenshots, keystrokes, window titles, URLs, and screen content are never captured.</p></section>`;
  }
  if (!view.capture_enabled) {
    return `<section class="banner"><b>Active-window capture is off</b><p>No active-window or idle data is being collected. You can enable it in Settings. Screenshots, keystrokes, window titles, URLs, and screen content are never captured.</p></section>`;
  }
  return `<section class="banner"><b>Active-window capture is on</b><p>The frontmost app and a coarse idle state are recorded locally on this Mac. Window titles, screenshots, keystrokes, URLs, and screen content are never captured. Manage it in Settings.</p></section>`;
}

// The always-visible left-sidebar capture status line (TASK-056). Replaces the hard-coded
// "Manual Mode / Capture deferred — No automatic activity capture runs in v0.1" box, which asserted
// on EVERY view that no capture runs and therefore contradicted the enabled state the Settings panel
// this task ships lets a user reach. Driven by the same real setting as the banner/panel, so no view
// ever carries an unconditional capture denial. `null` (setting not loaded / read failed) is a
// neutral "see Settings" pointer, never a false claim either way.
export function sidebarCaptureStatus(view: CaptureSettingsView | null): string {
  if (view == null) {
    return `<b>Active-window capture</b><p>Open Settings for capture status.</p>`;
  }
  if (!view.platform_supported) {
    return `<b>Active-window capture</b><p>Available on macOS only.</p>`;
  }
  if (!view.capture_enabled) {
    return `<b>Active-window capture: off</b><p>No active-window or idle data is being collected. Enable it in Settings.</p>`;
  }
  return `<b>Active-window capture: on</b><p>The frontmost app and a coarse idle state are recorded locally on this Mac.</p>`;
}
