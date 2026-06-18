// Pure rendering of the secret-free Langfuse import report (TASK-029 D). No DOM/IPC here so the
// markup stays unit-testable. Every value rendered is a count, a fixed health/reason label, an
// environment name, a JSON key name, or a JSON type name — never a credential, raw payload byte,
// prompt/session/metadata content, or a field value (SEC-011). All caller-supplied text is escaped.

import { escapeHtml as esc } from './html';

// Mirrors the backend report structs (`langfuse/importer.rs`, `langfuse/model.rs`). Tauri serializes
// the Rust field names verbatim (snake_case). `field`/`field_type`/`element_type` are omitted by the
// backend when absent (`skip_serializing_if`), so they are optional here.
export type SkipReasonCount = { reason: string; count: number };
export type SkipSample = {
  reason: string;
  keys: string[];
  field?: string | null;
  field_type?: string | null;
  element_type?: string | null;
};
export type EnvImportLine = {
  environment: string;
  health: string;
  pages: number;
  traces_seen: number;
  unique: number;
  duplicates: number;
  skipped_schema: number;
  skip_reasons: SkipReasonCount[];
  skip_samples: SkipSample[];
  reached_page_limit: boolean;
  warnings: string[];
};
export type ImportReport = {
  total_traces_seen: number;
  total_unique: number;
  total_duplicates: number;
  total_skipped_schema: number;
  total_skip_reasons: SkipReasonCount[];
  reached_page_limit: boolean;
  environment_count: number;
  environments: EnvImportLine[];
};

// Whether the report came from an incremental import (resumes per-env cursor) or an explicit
// backfill (re-scans the whole configured range). Only changes the headline wording.
export type ImportMode = 'incremental' | 'backfill';

// Friendly label for each fixed `SkipReason` (backend `model.rs`). The keys are the only reasons the
// backend emits; an unknown key falls back to its raw (escaped) form so a future variant still renders.
const SKIP_REASON_LABELS: Record<string, string> = {
  missing_trace_id: 'missing trace id',
  observations_not_embedded: 'observations not embedded (v3 id-list shape)',
  field_type_mismatch: 'field type mismatch',
  generation_lacks_usage_and_cost: 'generation lacks usage and cost',
  observations_fetch_failed: 'observations fetch failed',
};

export function skipReasonLabel(reason: string): string {
  return SKIP_REASON_LABELS[reason] ?? reason;
}

const plural = (n: number): string => (n === 1 ? '' : 's');

// One grouped line summarising every classified reason as "<count> <label>" joined by " · ".
// Grouped — one entry per reason, never the same line repeated per trace.
function groupedReasons(reasons: SkipReasonCount[]): string {
  return reasons
    .map((r) => `${r.count} ${esc(skipReasonLabel(r.reason))}`)
    .join(' · ');
}

function headline(report: ImportReport, mode: ImportMode): string {
  const prefix = mode === 'backfill' ? 'Backfill — ' : '';
  const envs = `${report.environment_count} environment${plural(report.environment_count)}`;
  const dup = `${report.total_duplicates} duplicate${plural(report.total_duplicates)}`;
  if (report.total_unique > 0) {
    return `${prefix}Imported ${report.total_unique} new trace${plural(report.total_unique)} across ${envs} — ${dup}, ${report.total_skipped_schema} skipped.`;
  }
  return `${prefix}No new traces imported across ${envs} (${report.total_traces_seen} seen, ${dup}, ${report.total_skipped_schema} skipped). See per-environment detail below.`;
}

// A bounded structural sample line: reason label + the entry's top-level key names + (for the
// field-scoped reasons) the offending field name and its JSON type name. Key/type NAMES only —
// never a field value or any payload content (SEC-011).
function sampleLine(sample: SkipSample): string {
  const keys = sample.keys.map((k) => esc(k)).join(', ');
  let fieldPart = '';
  if (sample.field) {
    const type = sample.element_type
      ? `${esc(sample.field_type ?? 'array')} of ${esc(sample.element_type)}`
      : esc(sample.field_type ?? 'unknown');
    fieldPart = ` · <code>${esc(sample.field)}</code>: ${type}`;
  }
  return `<li>${esc(skipReasonLabel(sample.reason))} — keys [${keys}]${fieldPart}</li>`;
}

function envLine(env: EnvImportLine): string {
  const counts = `${env.traces_seen} seen, ${env.unique} new, ${env.duplicates} duplicate, ${env.skipped_schema} skipped`;
  const pages = `${env.pages} page${plural(env.pages)}`;
  const limit = env.reached_page_limit
    ? ' · <b>reached page limit — re-run to continue</b>'
    : '';
  const reasons = env.skip_reasons.length
    ? ` · diagnostics: ${groupedReasons(env.skip_reasons)}`
    : '';
  // Backend warnings are fixed, secret-free transport/auth/persist strings; escaped defensively.
  const warnings = env.warnings.length
    ? ` · ${env.warnings.map((w) => esc(w)).join('; ')}`
    : '';
  return `<li><b>${esc(env.environment)}</b>: ${esc(env.health)} — ${counts} (${pages})${limit}${reasons}${warnings}</li>`;
}

// Render the whole import report as a secret-free, grouped, `aria-live` block. Returns '' when there
// is no report (e.g. the integration is disabled, so the backend returned `report: null`).
export function renderImportReport(report: ImportReport | null, mode: ImportMode = 'incremental'): string {
  if (!report) return '';
  const limitNote = report.reached_page_limit
    ? `<p class="import-warn">Some environments reached the pagination limit, so this run is incomplete. Re-run to continue — no data was truncated silently.</p>`
    : '';
  const diag = report.total_skip_reasons.length
    ? `<p class="import-diag"><b>Diagnostics (grouped):</b> ${groupedReasons(report.total_skip_reasons)}</p>`
    : '';
  const envRows = report.environments.map(envLine).join('');
  const samples = report.environments.flatMap((e) => e.skip_samples);
  const sampleBlock = samples.length
    ? `<details class="import-samples"><summary>Shape samples (key &amp; type names only — no values)</summary><ul>${samples.map(sampleLine).join('')}</ul></details>`
    : '';
  return `<div class="import-report" aria-live="polite"><p><b>Last import result:</b> ${esc(headline(report, mode))}</p>${limitNote}${diag}${envRows ? `<ul class="import-envs">${envRows}</ul>` : ''}${sampleBlock}</div>`;
}
