//! AI Runtime Observer (TASK-022). A read-only Rust-core component that ingests a local coarse
//! session log, reconciles it against the importer's normalized evidence, and persists a coarse
//! `ai_runtime_sessions` row per session. It makes **zero** network calls, scans **no** processes,
//! reads **no** command-lines, and is **never** a cost/time authority (DEC-003/DEC-017): the
//! session table has no token/cost/duration column. Absence of the log or a down/uncertain import
//! resolves to `reconciliation_unknown` — never zero AI usage/cost.

pub mod config;
pub mod ingest;
pub mod model;
pub mod reconcile;
pub mod store;

use chrono::{SecondsFormat, Utc};
use rusqlite::Connection;

use config::RuntimeConfig;
use model::{ReconciledSession, ReconciliationState, RuntimeReconciliationSnapshot};

/// Observe → reconcile → persist → snapshot, all local. Reads the configured session log and the
/// importer's evidence/run tables read-only, writes only the observer-owned `ai_runtime_sessions`
/// table, and returns coarse counts. A persistence failure surfaces as a secret-free `Err`.
pub fn observe_and_reconcile(
    conn: &Connection,
) -> Result<RuntimeReconciliationSnapshot, String> {
    let config = RuntimeConfig::from_env();
    observe_with_config(conn, &config)
}

/// The config-injectable core, so tests drive a deterministic path/environment without touching
/// process-global env vars (which would race across parallel tests).
pub(crate) fn observe_with_config(
    conn: &Connection,
    config: &RuntimeConfig,
) -> Result<RuntimeReconciliationSnapshot, String> {
    let ingested = ingest::ingest(config);
    let evidence = store::evidence_refs(conn).map_err(|e| e.to_string())?;
    let import_runs = store::import_runs(conn).map_err(|e| e.to_string())?;

    let reconciled =
        reconcile::reconcile_sessions(&ingested.sessions, &evidence, &import_runs, config);

    let observed_at = now();
    for r in &reconciled {
        store::upsert_session(conn, r, &observed_at).map_err(|_| PERSIST_FAILURE_MSG.to_string())?;
    }

    let unmatched_trace = reconcile::count_unmatched_traces(&reconciled, &evidence);
    Ok(build_snapshot(&reconciled, unmatched_trace, ingested.log_present))
}

/// Secret-free message used if the observer cannot persist a reconciled session. rusqlite driver
/// errors can carry no credential material here, but we still surface only this stable string.
pub const PERSIST_FAILURE_MSG: &str =
    "runtime observer could not persist reconciliation to the local store; state is unknown";

fn build_snapshot(
    reconciled: &[ReconciledSession],
    unmatched_trace: u32,
    log_present: bool,
) -> RuntimeReconciliationSnapshot {
    let count = |state: ReconciliationState| {
        reconciled.iter().filter(|r| r.reconciliation == state).count() as u32
    };
    let observed_no_trace = count(ReconciliationState::ObservedNoTrace);
    let reconciliation_unknown = count(ReconciliationState::ReconciliationUnknown);
    let observed_runs = reconciled.len() as u32;

    let message = if !log_present {
        "No runtime session log found — runtime reconciliation is unknown, never zero AI usage or cost."
            .to_string()
    } else {
        format!(
            "Observed {observed_runs} agent run(s); {observed_no_trace} healthy-import gap(s), \
             {reconciliation_unknown} unknown. A down or absent import is reported as unknown, never zero."
        )
    };

    RuntimeReconciliationSnapshot {
        runtime_log_present: log_present,
        source: "local".to_string(),
        observed_runs,
        matched: count(ReconciliationState::Matched),
        observed_no_trace,
        unmatched_runtime: count(ReconciliationState::UnmatchedRuntime),
        reconciliation_unknown,
        unmatched_trace,
        message,
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests;
