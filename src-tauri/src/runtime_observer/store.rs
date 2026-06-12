//! Observer-owned persistence: the additive `ai_runtime_sessions` table (TASK-022's narrow slice
//! of TASK-004) plus read-only projections of the importer's evidence/run tables. By construction
//! the session table has **no** token/cost/duration/prompt/command/secret column, so it can never
//! become a competing AI cost/time authority (DEC-003/DEC-017).

use rusqlite::{params, Connection};

use super::model::{EvidenceRef, ReconciledSession};
use crate::langfuse::model::HealthState;

/// Create `ai_runtime_sessions` if absent. Called from the single `init_db` entry point. Coarse
/// boundary + reconciliation state only — note the deliberate absence of any cost/token/duration
/// column.
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ai_runtime_sessions (
            session_key      TEXT PRIMARY KEY,
            project_key      TEXT NOT NULL,
            tool             TEXT,
            session_id       TEXT,
            environment      TEXT,
            started_at       TEXT,
            ended_at         TEXT,
            status           TEXT,
            reconciliation   TEXT NOT NULL,
            matched_trace_id TEXT,
            observed_at      TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_runtime_sessions_recon
            ON ai_runtime_sessions(reconciliation);",
    )
}

/// Idempotent upsert on `session_key`: re-ingesting the same log re-resolves reconciliation in
/// place without duplicating rows (the absence-preserving discipline the importer uses for its
/// `(environment, trace_id)` dedup).
pub fn upsert_session(
    conn: &Connection,
    r: &ReconciledSession,
    observed_at: &str,
) -> rusqlite::Result<()> {
    let s = &r.session;
    conn.execute(
        "INSERT INTO ai_runtime_sessions
            (session_key, project_key, tool, session_id, environment, started_at, ended_at,
             status, reconciliation, matched_trace_id, observed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(session_key) DO UPDATE SET
            project_key = excluded.project_key,
            tool = excluded.tool,
            session_id = excluded.session_id,
            environment = excluded.environment,
            started_at = excluded.started_at,
            ended_at = excluded.ended_at,
            status = excluded.status,
            reconciliation = excluded.reconciliation,
            matched_trace_id = excluded.matched_trace_id,
            observed_at = excluded.observed_at",
        params![
            s.session_key,
            s.project_key,
            s.tool,
            s.session_id,
            r.environment,
            s.started_at,
            s.ended_at,
            s.status,
            r.reconciliation.as_str(),
            r.matched_trace_id,
            observed_at,
        ],
    )?;
    Ok(())
}

/// Read-only projection of `langfuse_ai_evidence` for matching — the normalized row only, never the
/// prompt-bearing `langfuse_raw_traces.payload`. No token/cost column is selected.
pub fn evidence_refs(conn: &Connection) -> rusqlite::Result<Vec<EvidenceRef>> {
    let mut stmt = conn.prepare(
        "SELECT trace_id, environment, session_id, ai_start_ts, ai_end_ts
           FROM langfuse_ai_evidence",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(EvidenceRef {
                trace_id: row.get(0)?,
                environment: row.get(1)?,
                session_id: row.get(2)?,
                ai_start_ts: row.get(3)?,
                ai_end_ts: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// The importer's recorded run health for a window+environment, read-only. Used solely to *gate*
/// `observed_no_trace`; the observer never recomputes trace-side health.
#[derive(Debug, Clone)]
pub struct ImportRunHealth {
    pub environment: String,
    pub status: HealthState,
    pub window_from: Option<String>,
    pub window_to: Option<String>,
    pub finished_at: String,
}

pub fn import_runs(conn: &Connection) -> rusqlite::Result<Vec<ImportRunHealth>> {
    let mut stmt = conn.prepare(
        "SELECT environment, status, window_from, window_to, finished_at
           FROM langfuse_import_runs",
    )?;
    let rows = stmt
        .query_map([], |row| {
            let status: String = row.get(1)?;
            Ok(ImportRunHealth {
                environment: row.get(0)?,
                status: HealthState::parse(&status),
                window_from: row.get(2)?,
                window_to: row.get(3)?,
                finished_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
