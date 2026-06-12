//! The import engine: availability probe → paginate → dedup → normalize usage/cost from
//! observations → classify a health state → persist. Pure orchestration over the `LangfuseApi`
//! trait, so the whole flow is unit-tested against an in-memory mock (no network, no credentials).

use std::collections::HashSet;

use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use super::api::LangfuseApi;
use super::config::ImporterConfig;
use super::model::{
    AiEvidence, ApiError, ApiErrorKind, HealthState, ImportWindow, Observation, Trace,
};
use super::store::{self, ImportRunRecord};

/// Default page size for the traces list.
pub const PAGE_LIMIT: u32 = 50;
/// A latest-trace/cursor older than this (relative to the window end) reads as `stale`.
pub const STALE_AFTER_HOURS: i64 = 24;
/// Hard pagination backstop so a wrong `totalPages` can never spin forever.
const MAX_PAGES: u32 = 1000;

/// Per-environment outcome of one import run, returned for reporting/IPC.
#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub environment: String,
    pub health: HealthState,
    pub pages: u32,
    pub traces_seen: usize,
    pub unique: usize,
    pub duplicates: usize,
    pub cursor_ts: Option<String>,
    pub evidence: Vec<AiEvidence>,
    pub warnings: Vec<String>,
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

/// Run an import across the configured environments. Always probes availability first; a down or
/// unreachable stack records `unavailable` for each environment and never reports zero usage/cost.
/// The non-configured `default` environment is additionally checked so wrong-environment traffic
/// is surfaced (`wrong_env`) rather than silently lost.
pub fn run_import(
    api: &dyn LangfuseApi,
    conn: &Connection,
    config: &ImporterConfig,
    window: &ImportWindow,
) -> Vec<ImportSummary> {
    // 1. Availability gate — absence of a stack is a health state, never zero.
    if let Err(err) = api.probe() {
        return config
            .allowed_environments
            .iter()
            .map(|env| {
                let mut summary = unavailable_summary(conn, env, &err);
                persist_run(conn, &mut summary, &[], window);
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
        let mut imported = import_environment(api, conn, config, window, &env, is_allowed);
        // Don't record the synthetic `default` probe when it found nothing to surface.
        if !imported.is_allowed && !should_surface(&imported.summary) {
            continue;
        }
        persist_run(conn, &mut imported.summary, &imported.raw, window);
        out.push(imported.summary);
    }
    out
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
    let mut schema_issues = 0usize;
    let mut delayed = 0usize;
    let mut max_ts: Option<String> = None;
    let mut api_error: Option<ApiErrorKind> = None;

    let mut page = 1u32;
    loop {
        match api.get_traces(env, &window.from, &window.to, page, PAGE_LIMIT) {
            Ok(tp) => {
                pages = page;
                for value in &tp.data {
                    traces_seen += 1;
                    let trace: Trace = match serde_json::from_value(value.clone()) {
                        Ok(t) => t,
                        Err(_) => {
                            schema_issues += 1;
                            warnings.push("a trace did not match the expected shape".into());
                            continue;
                        }
                    };
                    // Dedup by (environment, trace_id) across pages, re-imports, and overlap.
                    if !seen.insert(trace.id.clone()) {
                        duplicates += 1;
                        continue;
                    }
                    unique_ids.insert(trace.id.clone());

                    if let (Some(ts), Some(cur)) = (&trace.timestamp, &prior_cursor) {
                        if ts.as_str() < cur.as_str() {
                            delayed += 1;
                        }
                    }
                    if let Some(ts) = &trace.timestamp {
                        if max_ts.as_deref().map_or(true, |m| ts.as_str() > m) {
                            max_ts = Some(ts.clone());
                        }
                    }

                    let (mut norm, schema_issue) = normalize_trace(api, env, &trace);
                    if schema_issue {
                        schema_issues += 1;
                    }
                    if !is_allowed {
                        // A trace in `default`/unexpected env is surfaced for review, never folded
                        // into the project's authoritative total.
                        norm.health = HealthState::WrongEnv;
                    }
                    evidence.push(norm);
                    raw.push((trace.id.clone(), value.to_string()));
                }

                let total_pages = tp.meta.total_pages;
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
    // (late-arriving, older) trace is reconciled without regressing the cursor.
    let cursor_ts = match (max_ts, prior_cursor.clone()) {
        (Some(a), Some(b)) => Some(if a.as_str() >= b.as_str() { a } else { b }),
        (Some(a), None) => Some(a),
        (None, b) => b,
    };
    let stale = is_stale(&cursor_ts, &window.to);

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

    EnvImport {
        summary: ImportSummary {
            environment: env.to_string(),
            health,
            pages,
            traces_seen,
            unique: unique_ids.len(),
            duplicates,
            cursor_ts,
            evidence,
            warnings,
        },
        raw,
        is_allowed,
    }
}

/// Aggregate token usage and cost for a trace from its generation **observations** (not the trace
/// body). Returns the evidence row and whether a schema/usage issue was detected. Token/cost stay
/// `None` when genuinely absent — never `0`.
fn normalize_trace(api: &dyn LangfuseApi, env: &str, trace: &Trace) -> (AiEvidence, bool) {
    let mut observations = trace.observations.clone();
    let mut fetch_failed = false;
    if observations.is_empty() {
        match api.get_observations(&trace.id) {
            Ok(obs) => observations = obs,
            Err(_) => fetch_failed = true,
        }
    }
    let generations: Vec<&Observation> = observations.iter().filter(|o| o.is_generation()).collect();

    let prompt_tokens = sum_opt_i64(generations.iter().map(|o| o.prompt()));
    let completion_tokens = sum_opt_i64(generations.iter().map(|o| o.completion()));
    let total_tokens = sum_opt_i64(generations.iter().map(|o| o.total()));
    let mut cost_total = sum_opt_f64(generations.iter().map(|o| o.cost()));
    if cost_total.is_none() {
        // Fall back to the trace-level aggregate convenience only if observations had no cost.
        cost_total = trace.total_cost;
    }
    let ai_start_ts = generations.iter().filter_map(|o| o.start_time.clone()).min();
    let ai_end_ts = generations.iter().filter_map(|o| o.end_time.clone()).max();

    let schema_issue =
        fetch_failed || generations.iter().any(|o| o.lacks_usage_and_cost());
    let health = if schema_issue {
        HealthState::SchemaChanged
    } else {
        HealthState::Healthy
    };

    (
        AiEvidence {
            trace_id: trace.id.clone(),
            environment: env.to_string(),
            ai_start_ts,
            ai_end_ts,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            cost_total,
            health,
        },
        schema_issue,
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
/// credential text (SEC-003).
const PERSIST_FAILURE_MSG: &str =
    "importer could not persist this run to the local store; recorded state is unknown";

/// Persist one import run **atomically** (S-3): the raw-trace rows, AI-evidence rows, and run record
/// commit together or not at all (`store::persist_import_run` wraps them in one transaction).
///
/// On a persistence failure the error is **surfaced, never swallowed** (S-4): the in-memory summary
/// degrades to a non-healthy `unknown` state with a secret-free warning, and a separate marker run
/// (its own id, so the failed run's id stays fully rolled back) records that non-healthy state into
/// the snapshot. A persistence failure therefore never reads as `healthy` and never contributes a
/// zero usage/cost total.
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
        let _ = store::insert_import_run(conn, &marker);
    }
}
