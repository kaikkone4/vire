//! The reconciliation engine: match each coarse runtime session against imported evidence
//! (`session_id` first, then `environment` + time-window overlap), then resolve the TASK-022
//! reconciliation state under the load-bearing **import-health gate** — `observed_no_trace` is
//! asserted only beneath a genuinely `healthy` import; a missing trace under a down/uncertain
//! import is `reconciliation_unknown`, never a real gap and never zero (NFR-008, "absence ≠ zero").

use std::collections::HashSet;

use chrono::{DateTime, Duration as ChronoDuration, Utc};

use super::config::RuntimeConfig;
use super::model::{EvidenceRef, ReconciledSession, ReconciliationState, RuntimeSession};
use super::store::ImportRunHealth;
use crate::langfuse::model::HealthState;

/// Reconcile every session against the imported evidence + run-health, in input order.
pub fn reconcile_sessions(
    sessions: &[RuntimeSession],
    evidence: &[EvidenceRef],
    import_runs: &[ImportRunHealth],
    config: &RuntimeConfig,
) -> Vec<ReconciledSession> {
    sessions
        .iter()
        .map(|s| resolve_one(s, evidence, import_runs, config))
        .collect()
}

fn resolve_one(
    session: &RuntimeSession,
    evidence: &[EvidenceRef],
    import_runs: &[ImportRunHealth],
    config: &RuntimeConfig,
) -> ReconciledSession {
    let environment = session.project_key.as_str().trim();
    let env = config.resolve_environment(environment);

    // 1. session_id exact — works across environments, the strongest signal.
    if let Some(sid) = nonempty(&session.session_id) {
        if let Some(ev) = evidence
            .iter()
            .find(|e| e.session_id.as_deref() == Some(sid.as_str()))
        {
            return matched(session, env, &ev.trace_id);
        }
    }

    // 2. environment + time-window overlap — only when the session maps to an environment.
    if let Some(env_name) = &env {
        if let Some(ev) = evidence
            .iter()
            .find(|e| &e.environment == env_name && overlaps(session, e, config.slop_secs))
        {
            return matched(session, Some(env_name.clone()), &ev.trace_id);
        }
    }

    // No environment mapping and no session_id match ⇒ cannot be evaluated against any import.
    let Some(env_name) = env else {
        return ReconciledSession {
            session: session.clone(),
            environment: None,
            reconciliation: ReconciliationState::UnmatchedRuntime,
            matched_trace_id: None,
        };
    };

    // 3. Import-health gate (load-bearing). Only a `healthy` covering import licenses a real gap.
    let health = import_health_for(import_runs, &env_name, session);
    let state = match health {
        HealthState::Unavailable | HealthState::Unknown | HealthState::AuthOrNetworkError => {
            ReconciliationState::ReconciliationUnknown
        }
        HealthState::Healthy => ReconciliationState::ObservedNoTrace,
        // stale / missing / wrong_env / delayed / duplicate / schema_changed: the import is not a
        // clean positive, so a missing trace cannot be asserted as a real gap — absence ≠ zero.
        _ => ReconciliationState::ReconciliationUnknown,
    };
    ReconciledSession {
        session: session.clone(),
        environment: Some(env_name),
        reconciliation: state,
        matched_trace_id: None,
    }
}

fn matched(session: &RuntimeSession, env: Option<String>, trace_id: &str) -> ReconciledSession {
    ReconciledSession {
        session: session.clone(),
        environment: env,
        reconciliation: ReconciliationState::Matched,
        matched_trace_id: Some(trace_id.to_string()),
    }
}

/// Count imported traces (evidence rows) that no runtime session matched. This is the
/// `unmatched_trace` signal, computed per evidence row (it has no session row of its own).
pub fn count_unmatched_traces(reconciled: &[ReconciledSession], evidence: &[EvidenceRef]) -> u32 {
    let matched_ids: HashSet<&str> = reconciled
        .iter()
        .filter_map(|r| r.matched_trace_id.as_deref())
        .collect();
    evidence
        .iter()
        .filter(|e| !matched_ids.contains(e.trace_id.as_str()))
        .count() as u32
}

/// The importer's recorded health for the run *covering* this session's window+environment: the
/// latest-finished run for that environment whose window overlaps the session. When no run covers
/// it (none for the env, or none overlapping), health is `Unknown` — never silently `healthy`.
fn import_health_for(
    import_runs: &[ImportRunHealth],
    env: &str,
    session: &RuntimeSession,
) -> HealthState {
    let (s_start, s_end) = session_bounds(session);
    let mut best: Option<&ImportRunHealth> = None;
    for run in import_runs.iter().filter(|r| r.environment == env) {
        if !window_covers(run, s_start, s_end) {
            continue;
        }
        match best {
            Some(b) if b.finished_at >= run.finished_at => {}
            _ => best = Some(run),
        }
    }
    best.map(|r| r.status).unwrap_or(HealthState::Unknown)
}

/// A run window covers the session when the two intervals overlap. A `NULL` window bound is treated
/// as open (−∞ / +∞), so a run with unrecorded bounds is considered covering rather than excluded.
fn window_covers(
    run: &ImportRunHealth,
    s_start: Option<DateTime<Utc>>,
    s_end: Option<DateTime<Utc>>,
) -> bool {
    let w_from = run.window_from.as_deref().and_then(parse_ts);
    let w_to = run.window_to.as_deref().and_then(parse_ts);
    // Session with no parseable timestamps: fall back to env-level coverage (any run for the env).
    let (Some(ss), Some(se)) = (s_start.or(s_end), s_end.or(s_start)) else {
        return true;
    };
    let after_start = w_to.is_none_or(|to| ss <= to);
    let before_end = w_from.is_none_or(|from| se >= from);
    after_start && before_end
}

/// True when the session interval overlaps the evidence interval, each expanded by `slop` seconds.
/// Missing end falls back to the start (a point); a session with no timestamps cannot time-match.
fn overlaps(session: &RuntimeSession, ev: &EvidenceRef, slop_secs: i64) -> bool {
    let (Some(ss), Some(se)) = session_bounds(session) else {
        return false;
    };
    let es = ev.ai_start_ts.as_deref().and_then(parse_ts);
    let ee = ev.ai_end_ts.as_deref().and_then(parse_ts);
    let (Some(es), Some(ee)) = (es.or(ee), ee.or(es)) else {
        return false;
    };
    let slop = ChronoDuration::seconds(slop_secs.max(0));
    // [ss, se] ∩ [es-slop, ee+slop] ≠ ∅
    ss <= ee + slop && se + slop >= es
}

/// Parse the session's [start, end] as a closed interval. A session with only one bound is a point
/// at that bound; a session with neither yields `(None, None)`.
fn session_bounds(session: &RuntimeSession) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let start = session.started_at.as_deref().and_then(parse_ts);
    let end = session.ended_at.as_deref().and_then(parse_ts);
    (start.or(end), end.or(start))
}

/// Tolerant timestamp parse: RFC3339 first (the importer's canonical form), then a couple of common
/// fallbacks so a slightly different emitter format still reconciles. Returns `None`, never a
/// fabricated epoch, when unparseable.
fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(v) {
        return Some(dt.with_timezone(&Utc));
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
    ] {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(v, fmt) {
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }
    None
}

fn nonempty(v: &Option<String>) -> Option<String> {
    v.as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
