// Pure date-window helpers for the Reports quick-range presets (TASK-033). No DOM or IPC here so the
// relative-window arithmetic stays unit-testable without a webview (mirrors src/langfuse-settings.ts).
// A preset only produces a {start, end} pair the user could already type into #rStart/#rEnd by hand.

import { localDateInputValue } from './date';

// Ordered, read-only preset list. `days` is the inclusive window length; `label` is the button text.
export const REPORT_RANGE_PRESETS: ReadonlyArray<{ key: string; label: string; days: number }> = [
  { key: 'last_7d', label: 'Last 7 days', days: 7 },
  { key: 'last_14d', label: 'Last 14 days', days: 14 },
  { key: 'last_30d', label: 'Last 30 days', days: 30 },
  { key: 'last_90d', label: 'Last 90 days', days: 90 },
];

// Inclusive window of `days` calendar days ending today: end = today, start = today − (days − 1).
// The subtraction is done on a LOCAL Date (new Date(y, m, d - (days-1))) and both ends are formatted via
// localDateInputValue — the same local path today() uses — so the dates never shift by a day through UTC
// conversion the way toISOString().slice(0,10) would near midnight or across time zones. For days ≥ 1
// start ≤ end always holds (all presets are ≥ 7).
export function reportRange(days: number, today = new Date()): { start: string; end: string } {
  const startDate = new Date(today.getFullYear(), today.getMonth(), today.getDate() - (days - 1));
  return { start: localDateInputValue(startDate), end: localDateInputValue(today) };
}
