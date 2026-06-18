// Pure helpers for the in-app Langfuse settings panel (TASK-026 A4). No DOM or IPC here so the
// logic stays unit-testable; the secret value is never an input or output of any of these.

export function environmentsToCsv(environments: string[]): string {
  return environments.join(', ');
}

export function parseEnvironmentsCsv(csv: string): string[] {
  return csv
    .split(',')
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
}

// Presence flag → label. We only ever know whether a key is set, never its value (SEC-009).
export function secretStateLabel(present: boolean): string {
  return present ? 'set' : 'not set';
}

// Test connection is gated on the SAVED enable switch. The Rust core authoritatively
// short-circuits a disabled integration before any Keychain/network read; the UI mirrors that so a
// disabled integration cannot trigger a probe. Empty string = allowed; otherwise the tooltip reason.
export function testConnectionDisabledReason(enabled: boolean): string {
  return enabled ? '' : 'Enable the Langfuse integration above to test the connection.';
}

// ---- Import range control (TASK-029 D3) -------------------------------------
// Pure canonicalization between the range picker and the backend `langfuse_import_range` setting.
// The backend (`langfuse/mod.rs` ImportRange) is the source of truth: it parses/validates/normalizes
// and returns the canonical string. These helpers only mirror that canonical vocabulary so the UI can
// seed the control and send a value the backend accepts — they never touch a credential or secret.

// The default range when none is configured (mirrors `ImportRange::default_range`).
export const DEFAULT_IMPORT_RANGE = 'last_30d';

// The sentinel preset for "custom since date" — NOT a backend value; it pairs with a date input that
// canonicalizes to `since:<RFC3339>`.
export const CUSTOM_RANGE_PRESET = 'custom';

// Picker options. The first four values are canonical backend keywords; `custom` reveals a date input.
export const IMPORT_RANGE_PRESETS: ReadonlyArray<{ value: string; label: string }> = [
  { value: 'last_7d', label: 'Last 7 days' },
  { value: 'last_30d', label: 'Last 30 days' },
  { value: 'last_90d', label: 'Last 90 days' },
  { value: 'all', label: 'All history' },
  { value: CUSTOM_RANGE_PRESET, label: 'Custom (since date)' },
];

const KEYWORD_RANGES = new Set(['last_7d', 'last_30d', 'last_90d', 'all']);
const DATE_ONLY = /^\d{4}-\d{2}-\d{2}$/;

// Turn the picker selection into the canonical string to persist. A keyword preset passes through; a
// custom date (the `YYYY-MM-DD` value of a date input) becomes `since:<date>T00:00:00Z`, the UTC
// RFC3339 floor the backend normalizes to. An empty/malformed custom date falls back to the default
// rather than sending an invalid value. The backend re-validates and returns the canonical form.
export function canonicalImportRange(preset: string, sinceDate: string): string {
  if (KEYWORD_RANGES.has(preset)) return preset;
  if (preset === CUSTOM_RANGE_PRESET && DATE_ONLY.test(sinceDate.trim())) {
    return `since:${sinceDate.trim()}T00:00:00Z`;
  }
  return DEFAULT_IMPORT_RANGE;
}

// Inverse of `canonicalImportRange` for seeding the control from the stored value: which preset is
// selected and (for a `since:` value) the `YYYY-MM-DD` to seed the date input. An unrecognized value
// resolves to the default preset, mirroring the backend's malformed-value fallback.
export function parseImportRangeControl(canonical: string): { preset: string; sinceDate: string } {
  const value = canonical.trim();
  if (KEYWORD_RANGES.has(value)) return { preset: value, sinceDate: '' };
  if (value.toLowerCase().startsWith('since:')) {
    const ts = value.slice(6).trim();
    const date = ts.slice(0, 10);
    return { preset: CUSTOM_RANGE_PRESET, sinceDate: DATE_ONLY.test(date) ? date : '' };
  }
  return { preset: DEFAULT_IMPORT_RANGE, sinceDate: '' };
}

// Human-readable label for the currently configured range (for the "Current range:" line).
export function importRangeLabel(canonical: string): string {
  const value = canonical.trim();
  const preset = IMPORT_RANGE_PRESETS.find((p) => p.value === value);
  if (preset) return preset.label;
  if (value.toLowerCase().startsWith('since:')) {
    const date = value.slice(6).trim().slice(0, 10);
    return DATE_ONLY.test(date) ? `Since ${date}` : 'Custom';
  }
  return value || importRangeLabel(DEFAULT_IMPORT_RANGE);
}
