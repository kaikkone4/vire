// Pure HTML builders for the AI time-suggestion review surface (TASK-032 Workstream C). No DOM or IPC
// here so the markup stays unit-testable; the click handlers that invoke list/accept/dismiss live in
// main.ts bindSuggestions(). Every value rendered is a project reference, a local date/time, an
// aggregate number, a count, or a fixed health/confidence/reason label — never a raw session id,
// payload byte, prompt, metadata, or credential (SEC-012). All caller-supplied text is escaped.
//
// Three guarantees are load-bearing and asserted in tests/suggestionsUi.test.mjs:
// - absence ≠ zero (DEC-004): an unknown duration renders "needs manual time" (never "0m"); unknown
//   tokens/cost render "—" (never "0"); unmapped evidence is surfaced, never dropped or zeroed.
// - no auto-posting (DEC-006): this module only renders Accept/Edit/Dismiss affordances. Accept is the
//   only action that posts a time entry, and an untimed block cannot be accepted without an edited span.
// - no secrets (SEC-012): only the secret-free fields below are rendered.

import { escapeHtml as esc } from './html';

// Mirrors the backend `Suggestion` (suggestions/mod.rs) — Tauri serializes the Rust field names
// verbatim (snake_case). `Option` fields arrive as `null` for genuinely-unknown values (absence ≠ zero).
export type Suggestion = {
  id: string;
  project_id: string;
  project_name: string;
  date: string;
  block_start_ts: string | null;
  block_end_ts: string | null;
  duration_minutes: number | null;
  trace_count: number;
  session_count: number;
  total_tokens: number | null;
  cost_total: number | null;
  cost_currency: string | null;
  health: string;
  confidence: string;
  source: string;
  reason: string;
  status: string;
  accepted_entry_id: string | null;
  created_at: string;
  updated_at: string;
};

// Mirrors the backend `UnmappedEnv` / `SuggestionList`.
export type UnmappedEnv = { environment: string; trace_count: number };
export type SuggestionList = { suggestions: Suggestion[]; unmapped: UnmappedEnv[] };

const plural = (n: number): string => (n === 1 ? '' : 's');
const hm = (min: number): string => `${Math.floor(min / 60)}h ${min % 60}m`;
// Thousands grouping without a locale dependency (deterministic render across environments).
const grouped = (n: number): string => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ',');

// Duration cell: a known duration → "Xh Ym"; unknown (null) → the absence ≠ zero copy. Never "0m".
export function durationLabel(min: number | null): string {
  return min == null ? 'needs manual time' : hm(min);
}

// Time-of-day ("HH:MM") from a 'YYYY-MM-DD HH:MM:SS' local timestamp; '' when absent.
function timeOfDay(ts: string | null): string {
  if (!ts) return '';
  const m = ts.match(/(\d{2}:\d{2})(?::\d{2})?\s*$/);
  return m ? m[1] : '';
}

// The block's local span "HH:MM–HH:MM"; "—" when the block carries no usable timestamps.
export function spanLabel(start: string | null, end: string | null): string {
  const s = timeOfDay(start);
  const e = timeOfDay(end);
  if (!s && !e) return '—';
  return `${s || '—'}–${e || '—'}`;
}

// Tokens are NULL when unknown → "—", never "0" (absence ≠ zero).
export function tokensLabel(total: number | null): string {
  return total == null ? '—' : grouped(total);
}

// Cost is NULL when unknown or mixed-currency → "—", never "0". Currency suffixed when present.
export function costLabel(total: number | null, currency: string | null): string {
  if (total == null) return '—';
  return `${total.toFixed(2)}${currency ? ` ${currency}` : ''}`;
}

// One suggestion as a summary row plus an inline edit row. Timed blocks expose Accept / Edit / Dismiss
// with the edit panel hidden; an untimed block exposes only Dismiss and an always-visible edit panel —
// its Accept-with-edits is the sole way to accept it, so a span must be supplied (the backend never
// invents a duration). Returns two <tr> siblings for an 8-column table.
export function suggestionRow(s: Suggestion): string {
  const id = esc(s.id);
  const untimed = s.duration_minutes == null;
  const evidence = `${s.trace_count} trace${plural(s.trace_count)} · ${s.session_count} session${plural(s.session_count)}`;
  const usage = `${tokensLabel(s.total_tokens)} tok · ${costLabel(s.cost_total, s.cost_currency)}`;
  const quality = `${esc(s.health)} · ${esc(s.confidence)}`;
  const actions = untimed
    ? `<button class="danger" data-dismiss="${id}">Dismiss</button>`
    : `<button data-accept="${id}">Accept</button> <button data-edit="${id}">Edit</button> <button class="danger" data-dismiss="${id}">Dismiss</button>`;
  const summary =
    `<tr data-sug-row="${id}">` +
    `<td>${esc(s.date)}</td>` +
    `<td>${esc(spanLabel(s.block_start_ts, s.block_end_ts))}</td>` +
    `<td>${esc(durationLabel(s.duration_minutes))}</td>` +
    `<td>${esc(evidence)}</td>` +
    `<td>${esc(usage)}</td>` +
    `<td>${quality}</td>` +
    `<td>${esc(s.reason)}</td>` +
    `<td>${actions}</td>` +
    `</tr>`;
  const startVal = timeOfDay(s.block_start_ts);
  const endVal = timeOfDay(s.block_end_ts);
  const hint = untimed
    ? `<p class="hint">This block has no usable timestamps. Set a start and end time to accept it — Vire never invents a duration.</p>`
    : '';
  const reqAttr = untimed ? ' required' : '';
  const edit =
    `<tr class="sug-edit" data-edit-panel="${id}"${untimed ? '' : ' hidden'}><td colspan="8">${hint}` +
    `<div class="lf-form">` +
    `<label>Date<input data-edit-field="date" type="date" value="${esc(s.date)}"></label>` +
    `<label>Start<input data-edit-field="start_time" type="time" value="${esc(startVal)}"${reqAttr}></label>` +
    `<label>End<input data-edit-field="end_time" type="time" value="${esc(endVal)}"${reqAttr}></label>` +
    `<label>Note<input data-edit-field="note" maxlength="2000" placeholder="Optional — a secret-free AI-provenance note is recorded by default"></label>` +
    `<div class="row"><button data-accept-edited="${id}">Accept with edits</button></div>` +
    `</div></td></tr>`;
  return summary + edit;
}

// Suggestions grouped by project → date. One panel per project (first-seen order); rows sorted by date
// then block start (untimed blocks, which sort last, surface at the end of their day).
export function suggestionGroups(suggestions: Suggestion[]): string {
  const byProject = new Map<string, Suggestion[]>();
  for (const s of suggestions) {
    const arr = byProject.get(s.project_id) ?? [];
    arr.push(s);
    byProject.set(s.project_id, arr);
  }
  return [...byProject.values()]
    .map((rows) => {
      const name = rows[0].project_name;
      const sorted = [...rows].sort(
        (a, b) =>
          a.date.localeCompare(b.date) ||
          (a.block_start_ts ?? '￿').localeCompare(b.block_start_ts ?? '￿'),
      );
      const body = sorted.map(suggestionRow).join('');
      return (
        `<section class="panel"><h2>${esc(name)}</h2>` +
        `<table class="sug-table"><tr><th>Date</th><th>Span</th><th>Duration</th><th>Evidence</th>` +
        `<th>Usage</th><th>Health · Confidence</th><th>Reason</th><th></th></tr>${body}</table></section>`
      );
    })
    .join('');
}

// Notice for evidence whose environment maps to no project — never dropped, never zeroed (absence ≠
// zero). Links back to Settings where mapping happens. Empty when nothing is unmapped.
export function unmappedNotice(unmapped: UnmappedEnv[]): string {
  if (!unmapped.length) return '';
  const items = unmapped
    .map(
      (u) =>
        `<li><b>${esc(u.environment)}</b> — ${u.trace_count} trace${plural(u.trace_count)} with no project mapping</li>`,
    )
    .join('');
  return (
    `<section class="banner"><b>Unmapped AI evidence</b><p>These Langfuse environments have imported evidence ` +
    `but no Vire project, so they produce no suggestions. Map them in Settings to get suggestions.</p>` +
    `<ul>${items}</ul><div class="row"><button data-goto-view="Settings">Open Settings to map</button></div></section>`
  );
}

// Empty/absence state — never a bare empty table (design §4). The pending list alone cannot distinguish
// "no evidence" from "all decided", so the candidate causes are named explicitly.
function emptyState(hasUnmapped: boolean): string {
  const causes = hasUnmapped
    ? 'Evidence exists but its environments are not mapped to a project yet — map them above to get suggestions.'
    : 'No pending suggestions. Either no AI evidence has been imported yet, or every suggestion has already been accepted or dismissed.';
  return `<div class="empty">${causes} Use <b>Refresh suggestions</b> to recompute from the latest imported evidence, or import from Langfuse in Settings.</div>`;
}

// The full Suggestions view body (header + refresh control + unmapped notice + grouped list or empty
// state). main.ts wraps this with the error/source banners and the shell chrome and binds the actions.
export function suggestionsBody(list: SuggestionList): string {
  const header = `<header><h1>Suggestions</h1><p>AI-evidence time blocks awaiting your review. Nothing is posted until you accept.</p></header>`;
  const controls = `<div class="filters"><button id="refreshSuggestions">Refresh suggestions</button></div>`;
  const body = list.suggestions.length
    ? suggestionGroups(list.suggestions)
    : emptyState(list.unmapped.length > 0);
  return `${header}${controls}${unmappedNotice(list.unmapped)}${body}`;
}
