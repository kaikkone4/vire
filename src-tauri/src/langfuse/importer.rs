//! The import engine: availability probe → paginate → dedup → normalize usage/cost from
//! observations → classify a health state → persist. Pure orchestration over the `LangfuseApi`
//! trait, so the whole flow is unit-tested against an in-memory mock (no network, no credentials).

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::Connection;
use serde_json::Value;
use uuid::Uuid;

use super::api::LangfuseApi;
use super::config::ImporterConfig;
use super::model::{
    self, AiEvidence, ApiError, ApiErrorKind, HealthState, ImportWindow, Observation, SkipReason,
    SkipReasonCount, SkipSample, Trace,
};
use super::store::{self, ImportRunRecord};

/// Default page size for the traces list.
pub const PAGE_LIMIT: u32 = 50;
/// A latest-trace/cursor older than this (relative to the window end) reads as `stale`.
pub const STALE_AFTER_HOURS: i64 = 24;
/// Hard pagination backstop so a wrong `totalPages` can never spin forever.
pub(crate) const MAX_PAGES: u32 = 1000;

/// Maximum bounded structural samples kept per skip reason (SEC-011 — diagnostics, not a log).
const MAX_SAMPLES_PER_REASON: usize = 3;

/// Incremental reconciliation lookback (seconds): a normal import resumes from `cursor − OVERLAP` so
/// late/delayed traces inside the overlap are re-seen. Durable `(env, trace_id)` dedup makes the
/// overlap free, and the cursor never regresses (TASK-029 C2).
const OVERLAP_SECS: i64 = 3600;

/// Minimum width of one backfill sub-window. A backfill commits durably chunk-by-chunk; 30 days is a
/// reasonable per-commit unit (TASK-029 C4).
const BACKFILL_MIN_CHUNK_DAYS: i64 = 30;

/// Upper bound on the number of backfill sub-windows. The chunk width widens above
/// [`BACKFILL_MIN_CHUNK_DAYS`] when needed so even an `all`-history backfill stays a bounded sequence
/// of atomic runs rather than thousands of tiny windows.
const MAX_BACKFILL_CHUNKS: i64 = 24;

/// Per-environment outcome of one import run, returned for reporting/IPC.
#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub environment: String,
    pub health: HealthState,
    pub pages: u32,
    pub traces_seen: usize,
    pub unique: usize,
    pub duplicates: usize,
    /// Traces genuinely dropped or degraded for a shape reason (no usable id, or a generation with
    /// no usage/cost in any supported location, or a failed observations fetch). Surfaced as
    /// `skipped` in the import report. Informational anomalies the importer tolerates (the v3
    /// identifier-list `observations` shape, a tolerated peripheral type mismatch) are NOT counted
    /// here — they appear in `skip_reasons` for diagnostics but never inflate the skip count or
    /// force `schema_changed` (TASK-029 A/B).
    pub skipped_schema: usize,
    /// Grouped, secret-free per-reason breakdown of skips/degrades for this environment (TASK-029 A).
    /// Replaces the old per-trace repeated warning string.
    pub skip_reasons: Vec<SkipReasonCount>,
    /// Bounded structural samples (≤ [`MAX_SAMPLES_PER_REASON`] per reason): JSON key names + the
    /// offending field's JSON type name only — never a value or any payload content (SEC-011).
    pub skip_samples: Vec<SkipSample>,
    /// True when at least one window for this environment hit the [`MAX_PAGES`] pagination backstop,
    /// so the run was bounded and the user should re-run to continue (no silent truncation, TASK-029 C4).
    pub reached_page_limit: bool,
    /// When [`reached_page_limit`](Self::reached_page_limit) is set, the durable **continuation
    /// boundary**: the SECOND-oldest *distinct* instant the page-limited window reached (or the single
    /// oldest instant when the window collapsed to one instant). A backfill persists this so the next
    /// "Backfill now" re-scans `[range_floor, this)` with an exclusive `toTimestamp`, re-reading the
    /// oldest (possibly partially-read) instant via durable dedup and reaching strictly-older history —
    /// monotonic progress without skipping the cut instant (TASK-029 C4 / SW-4 Blocker 1). `None` for any
    /// run that did not hit the page backstop. Internal to the importer — never serialized into the
    /// secret-free report.
    pub page_limit_floor_ts: Option<String>,
    pub cursor_ts: Option<String>,
    pub evidence: Vec<AiEvidence>,
    pub warnings: Vec<String>,
}

/// Secret-free, serializable summary of one import run for the Settings panel (SEC-010). It is
/// built **only** from counts, health enums (as fixed strings), environment names, and the
/// importer's existing secret-free warning strings — never a credential, `Authorization` header,
/// raw API response body, or trace prompt/session content. The per-trace `evidence` (token/cost
/// values) is deliberately excluded; only aggregate counts cross this boundary.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportReport {
    pub total_traces_seen: usize,
    pub total_unique: usize,
    pub total_duplicates: usize,
    pub total_skipped_schema: usize,
    /// Grouped skip/degrade reasons aggregated across every environment (TASK-029 A). Fixed reason
    /// labels + counts only — secret-free (SEC-011).
    pub total_skip_reasons: Vec<SkipReasonCount>,
    /// True when any environment's run was bounded by the pagination backstop, so the import was
    /// incomplete and re-running will continue it (no silent truncation, TASK-029 C4).
    pub reached_page_limit: bool,
    pub environment_count: usize,
    pub environments: Vec<EnvImportLine>,
}

/// One environment's line in the [`ImportReport`]. Counts + health + grouped secret-free skip
/// breakdown + secret-free warnings only.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnvImportLine {
    pub environment: String,
    pub health: String,
    pub pages: u32,
    pub traces_seen: usize,
    pub unique: usize,
    pub duplicates: usize,
    pub skipped_schema: usize,
    /// Grouped per-reason skip/degrade breakdown for this environment (TASK-029 A / D).
    pub skip_reasons: Vec<SkipReasonCount>,
    /// Bounded structural samples for this environment (SEC-011): key names + JSON type names only.
    pub skip_samples: Vec<SkipSample>,
    pub reached_page_limit: bool,
    pub warnings: Vec<String>,
}

impl ImportReport {
    /// Fold the per-environment summaries into the secret-free report. Pure aggregation of the
    /// counts the importer already computed — nothing new is read from the network or the store,
    /// and no token/cost value or raw payload is copied in.
    pub fn from_summaries(summaries: &[ImportSummary]) -> Self {
        let environments: Vec<EnvImportLine> = summaries
            .iter()
            .map(|s| EnvImportLine {
                environment: s.environment.clone(),
                health: s.health.as_str().to_string(),
                pages: s.pages,
                traces_seen: s.traces_seen,
                unique: s.unique,
                duplicates: s.duplicates,
                skipped_schema: s.skipped_schema,
                skip_reasons: s.skip_reasons.clone(),
                skip_samples: s.skip_samples.clone(),
                reached_page_limit: s.reached_page_limit,
                warnings: s.warnings.clone(),
            })
            .collect();
        ImportReport {
            total_traces_seen: environments.iter().map(|e| e.traces_seen).sum(),
            total_unique: environments.iter().map(|e| e.unique).sum(),
            total_duplicates: environments.iter().map(|e| e.duplicates).sum(),
            total_skipped_schema: environments.iter().map(|e| e.skipped_schema).sum(),
            total_skip_reasons: aggregate_skip_reasons(&environments),
            reached_page_limit: environments.iter().any(|e| e.reached_page_limit),
            environment_count: environments.len(),
            environments,
        }
    }
}

/// Sum each [`SkipReason`] across environments, preserving the fixed [`SkipReason::ALL`] order so the
/// aggregated breakdown is deterministic. Only reasons that actually occurred are emitted.
fn aggregate_skip_reasons(environments: &[EnvImportLine]) -> Vec<SkipReasonCount> {
    SkipReason::ALL
        .iter()
        .filter_map(|reason| {
            let label = reason.as_str();
            let count: usize = environments
                .iter()
                .flat_map(|e| &e.skip_reasons)
                .filter(|rc| rc.reason == label)
                .map(|rc| rc.count)
                .sum();
            (count > 0).then(|| SkipReasonCount {
                reason: label.to_string(),
                count,
            })
        })
        .collect()
}

/// Aggregates skip/degrade classifications for one environment's import: per-reason counts plus a
/// small bounded set of secret-free structural samples (TASK-029 A / SEC-011). This replaces the
/// per-trace repeated warning string with a grouped, secret-free breakdown — the classifier inspects
/// the raw `serde_json::Value` **structurally** and never copies a field value or a `serde` error.
#[derive(Default)]
struct SkipClassifier {
    counts: HashMap<SkipReason, usize>,
    samples: Vec<SkipSample>,
    samples_per_reason: HashMap<SkipReason, usize>,
}

impl SkipClassifier {
    /// Record one occurrence of `reason`, derived structurally from the raw entry `value`. Keeps at
    /// most [`MAX_SAMPLES_PER_REASON`] bounded structural samples per reason. `field` names the
    /// offending field (for the field-scoped reasons) so the sample can carry its JSON type name —
    /// never its value.
    fn record(&mut self, reason: SkipReason, value: &Value, field: Option<&str>) {
        *self.counts.entry(reason).or_insert(0) += 1;
        let kept = self.samples_per_reason.entry(reason).or_insert(0);
        if *kept < MAX_SAMPLES_PER_REASON {
            *kept += 1;
            self.samples.push(structural_sample(reason, value, field));
        }
    }

    /// Count of genuine drops/degrades (the reasons that surface as `schema_changed`); excludes the
    /// informational reasons (`observations_not_embedded`, `field_type_mismatch`).
    fn schema_issue_count(&self) -> usize {
        self.counts
            .iter()
            .filter(|(reason, _)| reason.is_schema_issue())
            .map(|(_, count)| *count)
            .sum()
    }

    /// Per-reason counts in fixed [`SkipReason::ALL`] order (only reasons that occurred), and the
    /// bounded structural samples.
    fn finish(self) -> (Vec<SkipReasonCount>, Vec<SkipSample>) {
        let counts = SkipReason::ALL
            .iter()
            .filter_map(|reason| {
                self.counts
                    .get(reason)
                    .copied()
                    .filter(|count| *count > 0)
                    .map(|count| SkipReasonCount {
                        reason: reason.as_str().to_string(),
                        count,
                    })
            })
            .collect();
        (counts, self.samples)
    }
}

/// Build one bounded, **secret-free** structural sample (SEC-011). Carries only the entry's top-level
/// JSON key **names**, the offending field **name**, that field's JSON **type name**, and — for an
/// array field — its first element's type name. No field value, nested content, raw bytes, or `serde`
/// error string is ever read.
fn structural_sample(reason: SkipReason, value: &Value, field: Option<&str>) -> SkipSample {
    let keys = value
        .as_object()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default();
    let field_value = field.and_then(|name| value.get(name));
    let field_type = field_value.map(model::json_type_name).map(str::to_string);
    let element_type = field_value
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .map(model::json_type_name)
        .map(str::to_string);
    SkipSample {
        reason: reason.as_str().to_string(),
        keys,
        field: field.map(str::to_string),
        field_type,
        element_type,
    }
}

struct EnvImport {
    summary: ImportSummary,
    raw: Vec<(String, String)>,
    is_allowed: bool,
}

/// Every importer-emitted timestamp (`started_at`/`finished_at`/`imported_at`) is UTC RFC3339, so a
/// run record is internally comparable with its RFC3339 `cursor_ts`/window/observation timestamps.
/// `finished_at` is the `ORDER BY … DESC` sort key for "latest run": an RFC3339 `…T…Z` value always
/// out-sorts a legacy space-separated row because `'T'` (0x54) > `' '` (0x20), so new rows stay
/// latest across the format transition with no data migration.
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Resolve the import window for one environment given its persisted cursor (TASK-029 C2). The
/// fixed-window engine passes a constant window for every environment; incremental/backfill pass a
/// per-environment resolver that reads from the range floor + cursor.
pub type WindowResolver<'a> = dyn Fn(&str, Option<&str>) -> ImportWindow + 'a;

/// Run an import across the configured environments using a fixed window for every environment. This
/// is the single-window engine the incremental and backfill paths build on; a backfill drives it
/// once per bounded sub-window (TASK-029 C4). Always probes availability first; a down or unreachable
/// stack records `unavailable` for each environment and never reports zero usage/cost. The
/// non-configured `default` environment is additionally checked so wrong-environment traffic is
/// surfaced (`wrong_env`) rather than silently lost.
pub fn run_import(
    api: &dyn LangfuseApi,
    conn: &Connection,
    config: &ImporterConfig,
    window: &ImportWindow,
) -> Vec<ImportSummary> {
    run_import_with(api, conn, config, &|_env, _cursor| window.clone())
}

/// Like [`run_import`] but resolves a **per-environment** window from `resolve(env, cursor_ts)`
/// (TASK-029 C2). Used by the incremental path so each environment resumes from its own persisted
/// cursor (less a reconciliation overlap). The cursor passed to the resolver is the environment's
/// last persisted `cursor_ts`, or `None` for a never-imported environment.
pub fn run_import_with(
    api: &dyn LangfuseApi,
    conn: &Connection,
    config: &ImporterConfig,
    resolve: &WindowResolver,
) -> Vec<ImportSummary> {
    // 1. Availability gate — absence of a stack is a health state, never zero.
    if let Err(err) = api.probe() {
        return config
            .allowed_environments
            .iter()
            .map(|env| {
                let window = resolve(env, prior_cursor_for(conn, env).as_deref());
                let mut summary = unavailable_summary(conn, env, &err);
                persist_run(conn, &mut summary, &[], &window);
                summary
            })
            .collect();
    }

    // 2. Import each configured environment, then surface `default` for wrong_env.
    let mut targets: Vec<(String, bool)> = config
        .allowed_environments
        .iter()
        .map(|e| (e.clone(), true))
        .collect();
    if !config.allowed_environments.iter().any(|e| e == "default") {
        targets.push(("default".to_string(), false));
    }

    let mut out = Vec::new();
    for (env, is_allowed) in targets {
        let window = resolve(&env, prior_cursor_for(conn, &env).as_deref());
        let mut imported = import_environment(api, conn, config, &window, &env, is_allowed);
        // Don't record the synthetic `default` probe when it found nothing to surface.
        if !imported.is_allowed && !should_surface(&imported.summary) {
            continue;
        }
        persist_run(conn, &mut imported.summary, &imported.raw, &window);
        out.push(imported.summary);
    }
    out
}

/// The persisted `cursor_ts` for an environment (the last successfully synced timestamp), or `None`
/// when the environment has never been imported.
fn prior_cursor_for(conn: &Connection, env: &str) -> Option<String> {
    store::latest_run_for_env(conn, env)
        .ok()
        .flatten()
        .and_then(|r| r.cursor_ts)
}

fn fmt_ts(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Resolve a single environment's **incremental** window (TASK-029 C2): `to = now`, and
/// `from = max(range_floor, cursor − OVERLAP)`. A never-imported environment (no cursor) starts at
/// the range floor. Timestamps are compared chronologically (parsed), not lexicographically, so
/// mixed RFC3339 precisions never mis-order the floor against the overlapped cursor.
pub fn incremental_window(range_floor: &str, cursor: Option<&str>, now: &str) -> ImportWindow {
    let from = match cursor.and_then(parse_ts) {
        Some(cur) => {
            let overlapped = cur - ChronoDuration::seconds(OVERLAP_SECS);
            let chosen = match parse_ts(range_floor) {
                Some(floor) if floor > overlapped => floor,
                _ => overlapped,
            };
            fmt_ts(chosen)
        }
        None => range_floor.to_string(),
    };
    ImportWindow {
        from,
        to: now.to_string(),
    }
}

/// Split `[floor, now]` into an ordered, bounded sequence of sub-windows, **newest → oldest**, so a
/// wide backfill commits durably chunk-by-chunk (each chunk is its own atomic run) instead of one
/// giant transaction (TASK-029 C4). Newest-first establishes currency immediately; the cursor never
/// regresses as older chunks follow. The chunk width is at least [`BACKFILL_MIN_CHUNK_DAYS`] and
/// widens as needed to keep the count ≤ [`MAX_BACKFILL_CHUNKS`], so even an `all`-history backfill is
/// a bounded sequence.
fn backfill_chunks(
    floor: DateTime<Utc>,
    ceiling: DateTime<Utc>,
    ceiling_str: &str,
) -> Vec<ImportWindow> {
    if ceiling <= floor {
        return Vec::new();
    }
    let span_days = (ceiling - floor).num_days().max(1);
    let chunk_days = std::cmp::max(
        BACKFILL_MIN_CHUNK_DAYS,
        (span_days + MAX_BACKFILL_CHUNKS - 1) / MAX_BACKFILL_CHUNKS,
    );
    let chunk = ChronoDuration::days(chunk_days);
    let mut windows = Vec::new();
    let mut end = ceiling;
    let mut first = true;
    while end > floor {
        let start = std::cmp::max(floor, end - chunk);
        windows.push(ImportWindow {
            from: fmt_ts(start),
            // The newest chunk's `to` is the resume ceiling verbatim, at its source precision. A
            // continuation boundary can carry sub-second precision; re-formatting it through
            // second-granularity `fmt_ts` would move the exclusive `toTimestamp` earlier and skip
            // traces in the truncated sub-second remainder (TASK-029 SW-4 Blocker 1).
            to: if first {
                ceiling_str.to_string()
            } else {
                fmt_ts(end)
            },
        });
        first = false;
        end = start;
    }
    windows
}

/// Run a **chunked, resumable backfill** over `[range_floor, now]` (TASK-029 C4 / DEC-030). Each
/// bounded sub-window runs through the single-window [`run_import`] engine and is persisted
/// atomically as its own run (S-3 preserved) advancing the per-env cursor — so an interruption loses
/// at most the in-flight chunk and a re-run converges via durable `(env, trace_id)` dedup and the
/// non-regressing cursor. Per-environment summaries are merged across chunks into one report. Honors
/// the same boundaries as a normal import (the caller probes/serializes/bounds identically). If the
/// stack is hard-down on a chunk, the run stops early rather than probing every remaining chunk.
///
/// **Page-limit continuation (TASK-029 C4 / SW-4 Blocker 1).** Because pagination always restarts at
/// page 1 and the source returns traces newest-first, a window dense enough to hit the [`MAX_PAGES`]
/// backstop cannot reach the history below the backstop in one run. To make re-runs **monotonic
/// without skipping**, a page-limited run persists a durable **continuation boundary**: the
/// *second-oldest distinct instant* it reached (`min_ts2`). The next backfill resumes at
/// `[range_floor, boundary)` — Langfuse's `toTimestamp` is **exclusive** ("traces *before* the
/// datetime") — so the scan re-reads the oldest, possibly partially-read instant (durable
/// `(env, trace_id)` dedup suppresses the overlap) and then reaches strictly-older history. Using the
/// second-oldest instant is what prevents a skip: the oldest reached instant may have been cut
/// mid-instant, so it must be re-included, while every instant at/above the boundary was fully drained.
///
/// When a page-limited window collapsed to a **single instant** (`min_ts2` is `None`), that instant
/// holds more traces than the page backstop can drain — genuinely unreachable through a
/// timestamp-filtered API. The boundary falls back to that instant; the exclusive resume scans strictly
/// below it, so the run still advances (no infinite loop) and the unreachable same-instant excess is
/// surfaced via `reached_page_limit` rather than silently truncated. A page-limited run that produced
/// **no usable timestamp at all** preserves any existing boundary (it never clears → never falsely
/// restarts from `now`). A clean, fully-covered run clears the boundary so a later "Backfill now"
/// starts fresh.
///
/// **Persistence faults are surfaced in-band (SW-4 Blocker 2).** A continuation-store read, write, or
/// clear failure is not swallowed: it injects the secret-free [`PERSIST_FAILURE_MSG`] sentinel via
/// [`flag_continuation_failure`], so `run_blocking`'s `import_result` collapses the run to an `Err` and
/// the UI/report can never claim durable resumability after a boundary operation failed.
pub fn run_backfill(
    api: &dyn LangfuseApi,
    conn: &Connection,
    config: &ImporterConfig,
    range_floor: &str,
    now: &str,
) -> Vec<ImportSummary> {
    let mut continuation_failed = false;
    // Read the persisted boundary. A store-read failure is NOT silently treated as "no boundary"
    // (which would falsely restart from `now`): it is flagged so the run surfaces in-band (Blocker 2).
    let stored = match store::backfill_resume_to(conn) {
        Ok(value) => value,
        Err(_) => {
            continuation_failed = true;
            None
        }
    };
    // Resume strictly below a previously-persisted boundary if it is older than `now`; otherwise scan
    // from `now`. A boundary at/after `now` (or unparseable) is ignored.
    let ceiling = match &stored {
        Some(resume_to) if cmp_ts(resume_to, now) == Some(Ordering::Less) => resume_to.clone(),
        _ => now.to_string(),
    };

    let chunks = match (parse_ts(range_floor), parse_ts(&ceiling)) {
        (Some(floor), Some(top)) => backfill_chunks(floor, top, &ceiling),
        // Unparseable bounds: fall back to one window over the literal range rather than nothing.
        _ => vec![ImportWindow {
            from: range_floor.to_string(),
            to: ceiling.clone(),
        }],
    };
    if chunks.is_empty() {
        // The boundary has reached the range floor (or the floor is already at/after now): the
        // configured range is fully covered. Clear any boundary and run one incremental-style pass so
        // the environments still surface an explicit health state rather than a blank result.
        if store::clear_backfill_resume_to(conn).is_err() {
            continuation_failed = true;
        }
        let mut summaries = run_import(
            api,
            conn,
            config,
            &ImportWindow {
                from: range_floor.to_string(),
                to: now.to_string(),
            },
        );
        if continuation_failed {
            flag_continuation_failure(&mut summaries);
        }
        return summaries;
    }
    let mut merged: Vec<ImportSummary> = Vec::new();
    // The newest continuation boundary across any environment that hit the page backstop this run.
    // Re-scanning `[range_floor, that point)` next run covers every page-limit gap (each lies below it)
    // without skipping a sparser environment's older history.
    let mut limited_floor: Option<String> = None;
    // Whether ANY chunk hit the page backstop, independent of whether it yielded a usable boundary
    // timestamp — distinguishes a clean run (clear the boundary) from a page-limited run with no usable
    // timestamp (preserve the boundary, never clear/restart).
    let mut any_page_limited = false;
    let mut hard_down_stop = false;
    for window in &chunks {
        let chunk = run_import(api, conn, config, window);
        let hard_down = !chunk.is_empty()
            && chunk.iter().all(|s| {
                matches!(
                    s.health,
                    HealthState::Unavailable | HealthState::AuthOrNetworkError
                )
            });
        for summary in &chunk {
            if summary.reached_page_limit {
                any_page_limited = true;
                limited_floor = later_ts(limited_floor.take(), summary.page_limit_floor_ts.clone());
            }
        }
        merge_summaries(&mut merged, chunk);
        if hard_down {
            // Stack is down/unreachable — re-probing every remaining chunk would just repeat the
            // failure. The committed chunks (if any) persist; the existing boundary is preserved so a
            // re-run resumes rather than restarting.
            hard_down_stop = true;
            break;
        }
    }
    // Advance, preserve, or clear the continuation boundary (Blocker 1), surfacing any store
    // write/clear fault in-band (Blocker 2):
    //  * page-limited with a usable boundary → advance strictly older (the second-oldest distinct
    //    instant guarantees monotonic progress without skipping the cut instant);
    //  * page-limited with no usable timestamp, OR a hard-down stop → preserve any existing boundary
    //    (never clear, so a re-run resumes rather than falsely restarting from `now`);
    //  * a clean, fully-covered run → clear the boundary.
    let store_result = if let Some(floor_ts) = limited_floor {
        store::set_backfill_resume_to(conn, &floor_ts, now)
    } else if any_page_limited || hard_down_stop {
        Ok(())
    } else {
        store::clear_backfill_resume_to(conn)
    };
    if store_result.is_err() {
        continuation_failed = true;
    }
    if continuation_failed {
        flag_continuation_failure(&mut merged);
    }
    merged
}

/// Merge a chunk's per-environment summaries into the running accumulator, keyed by environment, so
/// a multi-chunk backfill reports one aggregated line per environment (TASK-029 C4).
fn merge_summaries(acc: &mut Vec<ImportSummary>, chunk: Vec<ImportSummary>) {
    for summary in chunk {
        match acc
            .iter_mut()
            .find(|e| e.environment == summary.environment)
        {
            Some(existing) => merge_into(existing, summary),
            None => acc.push(summary),
        }
    }
}

/// Fold one later chunk's summary into an earlier one for the same environment. Counts sum; the
/// `reached_page_limit` flag OR-s; the cursor takes the chronologically-latest value (never
/// regressing); skip reasons sum per reason and samples stay bounded; health takes the more
/// significant state so a genuine `schema_changed`/error across any chunk is not masked by a later
/// empty chunk.
fn merge_into(acc: &mut ImportSummary, next: ImportSummary) {
    acc.pages = acc.pages.max(next.pages);
    acc.traces_seen += next.traces_seen;
    acc.unique += next.unique;
    acc.duplicates += next.duplicates;
    acc.skipped_schema += next.skipped_schema;
    acc.reached_page_limit |= next.reached_page_limit;
    // Keep the newest stopping point across chunks as a representative boundary (the per-chunk value
    // the backfill driver actually persists is recomputed there; this just keeps the merged summary
    // self-consistent).
    acc.page_limit_floor_ts = later_ts(acc.page_limit_floor_ts.take(), next.page_limit_floor_ts);
    acc.health = merge_health(acc.health, next.health);
    acc.cursor_ts = later_ts(acc.cursor_ts.take(), next.cursor_ts);
    acc.skip_reasons = merge_reason_counts(&acc.skip_reasons, &next.skip_reasons);
    acc.skip_samples = merge_samples(std::mem::take(&mut acc.skip_samples), next.skip_samples);
    acc.evidence.extend(next.evidence);
    for warning in next.warnings {
        if !acc.warnings.contains(&warning) {
            acc.warnings.push(warning);
        }
    }
}

/// The chronologically-later of two optional RFC3339 timestamps, compared by instant (`cmp_ts`) so the
/// cursor never regresses across mixed offsets/precisions.
fn later_ts(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => match cmp_ts(&b, &a) {
            Some(Ordering::Greater) => Some(b),
            Some(_) => Some(a),
            // One side unparseable: keep a parseable side, never lexically rank (SW-4 Blocker 3).
            None => {
                if parse_ts(&a).is_some() {
                    Some(a)
                } else {
                    Some(b)
                }
            }
        },
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

/// Pick the more significant health state across two chunks. Hard infrastructure/transport failures
/// and shape problems dominate a successful import; `healthy` dominates the benign empty/duplicate
/// states (so importing some traces is not masked by a later empty chunk).
fn merge_health(a: HealthState, b: HealthState) -> HealthState {
    fn rank(h: HealthState) -> u8 {
        match h {
            HealthState::Unavailable => 9,
            HealthState::AuthOrNetworkError => 8,
            HealthState::Unknown => 7,
            HealthState::SchemaChanged => 6,
            HealthState::WrongEnv => 5,
            HealthState::Healthy => 4,
            HealthState::Delayed => 3,
            HealthState::Duplicate => 2,
            HealthState::Stale => 1,
            HealthState::Missing => 0,
        }
    }
    if rank(a) >= rank(b) {
        a
    } else {
        b
    }
}

/// Sum two per-reason breakdowns, preserving the fixed [`SkipReason::ALL`] order.
fn merge_reason_counts(a: &[SkipReasonCount], b: &[SkipReasonCount]) -> Vec<SkipReasonCount> {
    SkipReason::ALL
        .iter()
        .filter_map(|reason| {
            let label = reason.as_str();
            let sum: usize = a
                .iter()
                .chain(b)
                .filter(|rc| rc.reason == label)
                .map(|rc| rc.count)
                .sum();
            (sum > 0).then(|| SkipReasonCount {
                reason: label.to_string(),
                count: sum,
            })
        })
        .collect()
}

/// Concatenate two sample lists while keeping at most [`MAX_SAMPLES_PER_REASON`] per reason.
fn merge_samples(mut acc: Vec<SkipSample>, next: Vec<SkipSample>) -> Vec<SkipSample> {
    for sample in next {
        let kept = acc.iter().filter(|s| s.reason == sample.reason).count();
        if kept < MAX_SAMPLES_PER_REASON {
            acc.push(sample);
        }
    }
    acc
}

/// Surface a continuation-store read/write/clear failure through the existing in-band persist-failure
/// channel (TASK-029 SW-4 Blocker 2 / TASK-021 S-4): inject the secret-free [`PERSIST_FAILURE_MSG`]
/// sentinel and degrade health to `unknown` so `run_blocking`'s `import_result` collapses the run to an
/// `Err`. The UI/report can then never claim durable resumability when the boundary could not be read,
/// written, or cleared. The sentinel string carries no boundary value, count, or credential.
fn flag_continuation_failure(summaries: &mut Vec<ImportSummary>) {
    if summaries.is_empty() {
        summaries.push(continuation_failure_summary());
        return;
    }
    for summary in summaries.iter_mut() {
        summary.health = HealthState::Unknown;
        if !summary.warnings.iter().any(|w| w == PERSIST_FAILURE_MSG) {
            summary.warnings.push(PERSIST_FAILURE_MSG.to_string());
        }
    }
}

/// A minimal summary carrying only the persist-failure sentinel, for the rare case a continuation-store
/// failure occurs with no per-environment summary to attach it to (e.g. a read failure before any chunk
/// produced a summary). Keeps the in-band `Err` channel reliable without inventing trace counts.
fn continuation_failure_summary() -> ImportSummary {
    ImportSummary {
        environment: String::new(),
        health: HealthState::Unknown,
        pages: 0,
        traces_seen: 0,
        unique: 0,
        duplicates: 0,
        skipped_schema: 0,
        skip_reasons: Vec::new(),
        skip_samples: Vec::new(),
        reached_page_limit: false,
        page_limit_floor_ts: None,
        cursor_ts: None,
        evidence: Vec::new(),
        warnings: vec![PERSIST_FAILURE_MSG.to_string()],
    }
}

fn should_surface(summary: &ImportSummary) -> bool {
    summary.unique > 0
        || matches!(
            summary.health,
            HealthState::Unavailable
                | HealthState::AuthOrNetworkError
                | HealthState::Unknown
                | HealthState::SchemaChanged
                | HealthState::WrongEnv
        )
}

fn unavailable_summary(conn: &Connection, env: &str, err: &ApiError) -> ImportSummary {
    let cursor_ts = store::latest_run_for_env(conn, env)
        .ok()
        .flatten()
        .and_then(|r| r.cursor_ts);
    ImportSummary {
        environment: env.to_string(),
        health: err.health(),
        pages: 0,
        traces_seen: 0,
        unique: 0,
        duplicates: 0,
        skipped_schema: 0,
        skip_reasons: Vec::new(),
        skip_samples: Vec::new(),
        reached_page_limit: false,
        page_limit_floor_ts: None,
        cursor_ts,
        evidence: Vec::new(),
        warnings: vec![err.message.clone()],
    }
}

fn import_environment(
    api: &dyn LangfuseApi,
    conn: &Connection,
    _config: &ImporterConfig,
    window: &ImportWindow,
    env: &str,
    is_allowed: bool,
) -> EnvImport {
    let prior = store::latest_run_for_env(conn, env).ok().flatten();
    let prior_cursor = prior.and_then(|r| r.cursor_ts);
    let mut seen: HashSet<String> = store::seen_trace_ids(conn, env).unwrap_or_default();

    let mut pages = 0u32;
    let mut traces_seen = 0usize;
    let mut duplicates = 0usize;
    let mut unique_ids: HashSet<String> = HashSet::new();
    let mut evidence: Vec<AiEvidence> = Vec::new();
    let mut raw: Vec<(String, String)> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut classifier = SkipClassifier::default();
    let mut delayed = 0usize;
    // Cursor candidate: the chronologically-NEWEST parseable instant this run saw (original source
    // string kept for fidelity). Unparseable timestamps are ignored — never lexically ranked (Blocker 3).
    let mut max_ts: Option<String> = None;
    // The two chronologically-OLDEST *distinct* parseable instants this window reached, over every
    // id-bearing entry the source returned (duplicates included, before the dedup short-circuit), so a
    // page-limited run knows how far back it got AND whether the cut fell inside a single instant. The
    // continuation boundary is the SECOND-oldest distinct instant (`min_ts2`); see
    // [`note_oldest_instants`] and the [`run_backfill`] page-limit-continuation docs. Tracked even for
    // duplicates because a re-run re-walks the already-imported newest pages and the boundary must still
    // advance. Unparseable timestamps are ignored so source garbage never becomes the boundary (Blocker 3).
    let mut min_ts: Option<(DateTime<Utc>, String)> = None;
    let mut min_ts2: Option<(DateTime<Utc>, String)> = None;
    let mut api_error: Option<ApiErrorKind> = None;
    let mut reached_page_limit = false;

    let mut page = 1u32;
    loop {
        match api.get_traces(env, &window.from, &window.to, page, PAGE_LIMIT) {
            Ok(tp) => {
                pages = page;
                for value in &tp.data {
                    traces_seen += 1;
                    // Identification is decoupled from usage parsing (TASK-029 B1 / DEC-031): an
                    // entry with a usable `id` is ALWAYS imported; the only identification failure
                    // that still skips is a missing/blank id.
                    let Some(id) = model::trace_id(value) else {
                        classifier.record(SkipReason::MissingTraceId, value, None);
                        continue;
                    };
                    // Track how far back the PAGES reached (the two oldest distinct instants), over every
                    // id-bearing entry before the dedup short-circuit, so the page-limit continuation
                    // boundary is accurate even when a re-run re-walks already-imported pages. An
                    // unparseable timestamp is ignored — never lexically ordered into the boundary (Blocker 3).
                    if let Some((dt, ts)) = value
                        .get("timestamp")
                        .and_then(|v| v.as_str())
                        .and_then(|s| parse_ts(s).map(|dt| (dt, s)))
                    {
                        note_oldest_instants(&mut min_ts, &mut min_ts2, dt, ts);
                    }
                    // Dedup by (environment, trace_id) across pages, re-imports, and overlap.
                    if !seen.insert(id.clone()) {
                        duplicates += 1;
                        continue;
                    }
                    unique_ids.insert(id.clone());

                    // Informational: the current v3 list shape carries observation ID strings, not
                    // embedded objects. The trace is still imported (usage comes from the fetch); we
                    // record the reason so the report explains it rather than dropping the trace.
                    if model::observations_is_id_list(value) {
                        classifier.record(
                            SkipReason::ObservationsNotEmbedded,
                            value,
                            Some("observations"),
                        );
                    }

                    // Fast path: the strict typed parse (now tolerant of the v3 observations shape).
                    // If a *peripheral* identification field has an unexpected JSON type the strict
                    // parse fails — record the field name and fall back to the tolerant reader so the
                    // identifiable trace is still imported, never dropped.
                    let trace = match serde_json::from_value::<Trace>(value.clone()) {
                        Ok(trace) => trace,
                        Err(_) => {
                            let field = model::offending_identification_field(value);
                            classifier.record(SkipReason::FieldTypeMismatch, value, field);
                            match Trace::from_value_tolerant(value) {
                                Some(trace) => trace,
                                // Unreachable (id is present), but never drop silently if it happens.
                                None => continue,
                            }
                        }
                    };

                    // Delayed = a NEW trace whose instant precedes the prior cursor. An unparseable
                    // trace timestamp or cursor yields `None` and is NOT classified delayed (ignored),
                    // never lexically compared (Blocker 3).
                    if let (Some(ts), Some(cur)) = (&trace.timestamp, &prior_cursor) {
                        if cmp_ts(ts, cur) == Some(Ordering::Less) {
                            delayed += 1;
                        }
                    }
                    if let Some(ts) = &trace.timestamp {
                        if parse_ts(ts).is_some()
                            && max_ts
                                .as_deref()
                                .is_none_or(|m| cmp_ts(ts, m) == Some(Ordering::Greater))
                        {
                            max_ts = Some(ts.clone());
                        }
                    }

                    let (mut norm, degrade) = normalize_trace(api, env, &trace);
                    if let Some(reason) = degrade {
                        classifier.record(reason, value, None);
                    }
                    if !is_allowed {
                        // A trace in `default`/unexpected env is surfaced for review, never folded
                        // into the project's authoritative total.
                        norm.health = HealthState::WrongEnv;
                    }
                    evidence.push(norm);
                    raw.push((id, value.to_string()));
                }

                let total_pages = tp.meta.total_pages;
                if page >= MAX_PAGES && page < total_pages && !tp.data.is_empty() {
                    // Bounded by the pagination backstop with more pages still indicated: say so
                    // rather than silently truncating — the user re-runs to continue (TASK-029 C4).
                    reached_page_limit = true;
                    break;
                }
                if tp.data.is_empty() || page >= total_pages || page >= MAX_PAGES {
                    break;
                }
                page += 1;
            }
            Err(e) => {
                warnings.push(e.message.clone());
                api_error = Some(e.kind);
                break;
            }
        }
    }

    // Advance the checkpoint to the latest trace seen, but never move it backward — a delayed
    // (late-arriving, older) trace is reconciled without regressing the cursor. Compared by instant, so
    // a late trace carrying a different offset/precision can never lexically out-sort and regress the
    // cursor; an unparseable prior cursor (legacy data → `cmp_ts` None) is replaced by this run's
    // known-good newest instant rather than lexically ranked (Blocker 3).
    let cursor_ts = match (max_ts, prior_cursor.clone()) {
        (Some(a), Some(b)) => Some(match cmp_ts(&a, &b) {
            Some(Ordering::Less) => b,
            _ => a,
        }),
        (Some(a), None) => Some(a),
        (None, b) => b,
    };
    // The continuation boundary is only meaningful when the run was actually bounded by the page
    // backstop; otherwise the window was fully drained and there is nothing to resume. Use the
    // SECOND-oldest distinct instant so the exclusive resume re-reads the oldest (possibly
    // partially-read) instant without skipping it; fall back to the single oldest instant when the whole
    // page-limited window collapsed to one instant (genuinely saturated — see [`run_backfill`] docs).
    let page_limit_floor_ts = if reached_page_limit {
        min_ts2.map(|(_, s)| s).or_else(|| min_ts.map(|(_, s)| s))
    } else {
        None
    };
    let stale = is_stale(&cursor_ts, &window.to);

    let schema_issues = classifier.schema_issue_count();
    let outcome = Outcome {
        available: true,
        api_error,
        schema_issues,
        wrong_env: !is_allowed && !unique_ids.is_empty(),
        traces_seen,
        unique: unique_ids.len(),
        delayed,
        stale,
    };
    let health = classify_health(&outcome);
    let (skip_reasons, skip_samples) = classifier.finish();

    EnvImport {
        summary: ImportSummary {
            environment: env.to_string(),
            health,
            pages,
            traces_seen,
            unique: unique_ids.len(),
            duplicates,
            skipped_schema: schema_issues,
            skip_reasons,
            skip_samples,
            reached_page_limit,
            page_limit_floor_ts,
            cursor_ts,
            evidence,
            warnings,
        },
        raw,
        is_allowed,
    }
}

/// Aggregate token usage and cost for a trace from its generation **observations** (not the trace
/// body). Returns the evidence row and, when usage/cost are genuinely unreadable, the specific
/// secret-free [`SkipReason`] the trace is degraded under (`schema_changed`). Token/cost stay `None`
/// when genuinely absent — never `0`. Usage extraction is fully decoupled from identification: the
/// observations fetch (the authoritative source for the v3 list shape) supplies usage/cost when the
/// list payload embedded only observation IDs.
fn normalize_trace(
    api: &dyn LangfuseApi,
    env: &str,
    trace: &Trace,
) -> (AiEvidence, Option<SkipReason>) {
    let mut observations = trace.observations.clone();
    let mut degrade: Option<SkipReason> = None;
    if observations.is_empty() {
        // N+1 ACKNOWLEDGED, NOT HIDDEN (TASK-029 C6, design §4.4): one observations fetch per trace.
        // For the v3 list shape (and thus every backfilled trace) this is one round-trip per trace —
        // the dominant cost at backfill scale. The windowed-observations-scan optimization (one
        // paginated `GET /api/public/observations` per chunk, joined by `traceId`) is a deliberate
        // deferred follow-up; the per-trace path is correct, just slower, and stays read-only on the
        // already-allowlisted observations endpoint. The chunked-resumable backfill bounds the blast
        // radius (a timeout is non-destructive), so deferring the optimization is safe, not silent.
        match api.get_observations(&trace.id) {
            Ok(obs) => observations = obs,
            Err(_) => degrade = Some(SkipReason::ObservationsFetchFailed),
        }
    }
    let generations: Vec<&Observation> =
        observations.iter().filter(|o| o.is_generation()).collect();

    let prompt_tokens = sum_opt_i64(generations.iter().map(|o| o.prompt()));
    let completion_tokens = sum_opt_i64(generations.iter().map(|o| o.completion()));
    let total_tokens = sum_opt_i64(generations.iter().map(|o| o.total()));
    let mut cost_total = sum_opt_f64(generations.iter().map(|o| o.cost()));
    if cost_total.is_none() {
        // Fall back to the trace-level aggregate convenience only if observations had no cost.
        cost_total = trace.total_cost;
    }
    let ai_start_ts = generations
        .iter()
        .filter_map(|o| o.start_time.clone())
        .min();
    let ai_end_ts = generations.iter().filter_map(|o| o.end_time.clone()).max();

    // A genuinely unsupported usage shape degrades to `schema_changed` (reserved for exactly this,
    // after the v3 widening — never the normal ID-list shape). A fetch failure already won.
    if degrade.is_none() && generations.iter().any(|o| o.lacks_usage_and_cost()) {
        degrade = Some(SkipReason::GenerationLacksUsageAndCost);
    }
    let health = if degrade.is_some() {
        HealthState::SchemaChanged
    } else {
        HealthState::Healthy
    };

    (
        AiEvidence {
            trace_id: trace.id.clone(),
            environment: env.to_string(),
            session_id: trace.session_id.clone(),
            ai_start_ts,
            ai_end_ts,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            cost_total,
            health,
        },
        degrade,
    )
}

/// Sum that preserves absence: `None` only when every contributor is `None`. A present value of
/// `0` is distinct from no value at all — this is the absence-≠-zero invariant in one place.
fn sum_opt_i64<I: Iterator<Item = Option<i64>>>(iter: I) -> Option<i64> {
    let mut acc: Option<i64> = None;
    for v in iter {
        if let Some(x) = v {
            acc = Some(acc.unwrap_or(0) + x);
        }
    }
    acc
}

fn sum_opt_f64<I: Iterator<Item = Option<f64>>>(iter: I) -> Option<f64> {
    let mut acc: Option<f64> = None;
    for v in iter {
        if let Some(x) = v {
            acc = Some(acc.unwrap_or(0.0) + x);
        }
    }
    acc
}

struct Outcome {
    available: bool,
    api_error: Option<ApiErrorKind>,
    schema_issues: usize,
    wrong_env: bool,
    traces_seen: usize,
    unique: usize,
    delayed: usize,
    stale: bool,
}

/// Resolve the BA §7 health state by fixed precedence so each of the ten states is deterministically
/// reachable. Order matters: hard infrastructure/transport failures dominate, then data-shape
/// problems, then content signals, and only a clean positive result yields `healthy`.
fn classify_health(o: &Outcome) -> HealthState {
    if !o.available {
        return HealthState::Unavailable;
    }
    if let Some(kind) = o.api_error {
        return match kind {
            ApiErrorKind::Unavailable => HealthState::Unavailable,
            ApiErrorKind::Auth | ApiErrorKind::RateLimited | ApiErrorKind::Network => {
                HealthState::AuthOrNetworkError
            }
            ApiErrorKind::Indeterminate => HealthState::Unknown,
        };
    }
    if o.schema_issues > 0 {
        return HealthState::SchemaChanged;
    }
    if o.wrong_env {
        return HealthState::WrongEnv;
    }
    // A re-import/overlap that suppressed every trace it saw is the `duplicate` signal.
    if o.traces_seen > 0 && o.unique == 0 {
        return HealthState::Duplicate;
    }
    if o.delayed > 0 {
        return HealthState::Delayed;
    }
    if o.unique == 0 {
        return if o.stale {
            HealthState::Stale
        } else {
            HealthState::Missing
        };
    }
    HealthState::Healthy
}

fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Chronological comparison of two RFC3339 timestamps by real **instant**. Both operands are parsed to
/// `DateTime<Utc>` so values with different UTC offsets or fractional-second precision order by instant,
/// not by byte sequence — a lexicographic compare mis-orders e.g. `…T10:00:00+02:00` (08:00Z) against
/// `…T09:30:00Z`, or `…05Z` against `…05.250Z` (`'Z'` 0x5A sorts after `'.'` 0x2E). Returns `None` when
/// EITHER value is unparseable: there is **no lexical fallback** (TASK-029 SW-4). A malformed timestamp
/// must be classified or ignored by the caller for every cursor/continuation/delayed decision — never
/// silently byte-ordered, which would let arbitrary source garbage win a `max`/`min`/cursor selection.
/// This is the single instant-comparison primitive behind those decisions.
fn cmp_ts(a: &str, b: &str) -> Option<Ordering> {
    Some(parse_ts(a)?.cmp(&parse_ts(b)?))
}

/// Maintain the two chronologically-**oldest distinct** instants seen so far: `min1` is the oldest,
/// `min2` is the smallest instant strictly greater than `min1` (the second-oldest distinct value).
/// Repeated traces at the same instant never collapse `min2` onto `min1`. The page-limit continuation
/// boundary is `min2` (the oldest *fully-drained* instant) so the next backfill — which scans with an
/// exclusive `toTimestamp` — re-reads the oldest, possibly partially-read instant (durable dedup
/// suppresses the overlap) without skipping any trace at the cut instant (TASK-029 C4 / SW-4 Blocker 1).
fn note_oldest_instants(
    min1: &mut Option<(DateTime<Utc>, String)>,
    min2: &mut Option<(DateTime<Utc>, String)>,
    dt: DateTime<Utc>,
    s: &str,
) {
    match min1 {
        None => *min1 = Some((dt, s.to_string())),
        Some((m1, _)) => {
            let m1 = *m1;
            if dt < m1 {
                // New oldest; the previous oldest becomes the second-oldest distinct instant.
                *min2 = min1.take();
                *min1 = Some((dt, s.to_string()));
            } else if dt > m1 {
                // Candidate for the smallest instant strictly greater than the oldest.
                match min2 {
                    None => *min2 = Some((dt, s.to_string())),
                    Some((m2, _)) if dt < *m2 => *min2 = Some((dt, s.to_string())),
                    _ => {}
                }
            }
            // dt == m1: another trace at the current oldest instant — no distinct second value.
        }
    }
}

fn is_stale(cursor_ts: &Option<String>, window_to: &str) -> bool {
    let Some(cursor) = cursor_ts else {
        return false;
    };
    let (Some(cur), Some(to)) = (parse_ts(cursor), parse_ts(window_to)) else {
        return false;
    };
    cur < to - ChronoDuration::hours(STALE_AFTER_HOURS)
}

/// Fixed, secret-free message recorded when a run cannot be persisted. rusqlite driver errors carry
/// no credential material; we surface only this stable string and never interpolate config or
/// credential text (SEC-003). `run_blocking_import` (mod.rs) keys on this exact string to surface a
/// persist failure to the import IPC **in-band**, so correctness does not depend on a durable DB
/// write that the same fault may make impossible (TASK-021 S-4).
pub const PERSIST_FAILURE_MSG: &str =
    "importer could not persist this run to the local store; recorded state is unknown";

/// Persist one import run **atomically** (S-3): the raw-trace rows, AI-evidence rows, and run record
/// commit together or not at all (`store::persist_import_run` wraps them in one transaction).
///
/// On a persistence failure the error is **surfaced, never swallowed** (S-4): the in-memory summary
/// degrades to a non-healthy `unknown` state with the secret-free `PERSIST_FAILURE_MSG` sentinel in
/// its warnings. That sentinel is the authoritative signal — `run_blocking_import` keys on it to
/// return an in-band `Err`, so the manual-import IPC surfaces the failure even when the store is
/// broadly unwritable and never reads a stale `healthy` snapshot (TASK-021). The separate marker run
/// below (its own id, so the failed run's id stays fully rolled back) is **best-effort**
/// defense-in-depth for the on-demand `get_langfuse_source_health` snapshot when the small insert can
/// still succeed; it is no longer load-bearing for correctness, so its error stays discarded
/// (`let _`). A persistence failure therefore never reads as `healthy` and never contributes a zero
/// usage/cost total.
fn persist_run(
    conn: &Connection,
    summary: &mut ImportSummary,
    raw: &[(String, String)],
    window: &ImportWindow,
) {
    let stamp = now();
    let record = ImportRunRecord {
        id: Uuid::new_v4().to_string(),
        environment: summary.environment.clone(),
        window_from: Some(window.from.clone()),
        window_to: Some(window.to.clone()),
        cursor_ts: summary.cursor_ts.clone(),
        status: summary.health,
        pages_walked: summary.pages,
        traces_seen: summary.traces_seen as u32,
        duplicates_suppressed: summary.duplicates as u32,
        warnings: summary.warnings.clone(),
        started_at: stamp.clone(),
        finished_at: stamp.clone(),
    };
    if store::persist_import_run(conn, &record, raw, &summary.evidence, &stamp).is_err() {
        summary.health = HealthState::Unknown;
        summary.warnings.push(PERSIST_FAILURE_MSG.to_string());
        let marker = ImportRunRecord {
            id: Uuid::new_v4().to_string(),
            status: HealthState::Unknown,
            // Nothing was persisted, so do not advance currency on the marker.
            cursor_ts: None,
            warnings: vec![PERSIST_FAILURE_MSG.to_string()],
            ..record
        };
        // Best-effort durable marker (defense-in-depth for the on-demand snapshot). Correctness is
        // guaranteed by the in-band `Err` in `run_blocking_import`, so a marker insert that fails
        // under the same fault that broke the run transaction is fine — error stays discarded.
        let _ = store::insert_import_run(conn, &marker);
    }
}
