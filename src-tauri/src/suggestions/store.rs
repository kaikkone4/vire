//! Persistence for `time_entry_suggestions` (TASK-032 A2). Additive, idempotent migration mirroring
//! `env_mapping::migrate`'s placement after `projects`/`time_entries` so the `project_id` foreign key
//! resolves. Stores only secret-free aggregates (SEC-012) — never a session id, payload, or
//! credential. Evidence is associated to its project at read time via a LEFT JOIN (DEC-001), so no
//! evidence row is ever rewritten.

use rusqlite::{params, Connection, OptionalExtension};

use super::Suggestion;

/// Create `time_entry_suggestions` if absent (design §1.1). Called from `init_db` **after** the
/// `projects`/`time_entries`/`env_mapping` tables exist (FK ordering). The natural key
/// `(project_id, date, block_start_ts)` plus the decided-row guard in [`insert_if_not_decided`] make
/// regeneration idempotent without resurrecting an accepted/dismissed block as a fresh pending one.
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS time_entry_suggestions (
            id                TEXT PRIMARY KEY,
            project_id        TEXT NOT NULL REFERENCES projects(id),
            date              TEXT NOT NULL,
            block_start_ts    TEXT,
            block_end_ts      TEXT,
            duration_minutes  INTEGER,
            trace_count       INTEGER NOT NULL,
            session_count     INTEGER NOT NULL,
            total_tokens      INTEGER,
            cost_total        REAL,
            cost_currency     TEXT,
            health            TEXT NOT NULL,
            confidence        TEXT NOT NULL,
            source            TEXT NOT NULL,
            reason            TEXT NOT NULL,
            status            TEXT NOT NULL DEFAULT 'pending',
            accepted_entry_id TEXT,
            created_at        TEXT NOT NULL,
            updated_at        TEXT NOT NULL,
            UNIQUE (project_id, date, block_start_ts)
        );
        CREATE INDEX IF NOT EXISTS idx_suggestions_status_date
            ON time_entry_suggestions(status, date);",
    )
}

/// One evidence row joined to its mapped project at read time (DEC-001 — no rewrite). `project_id` is
/// `None` when the environment is unmapped; those rows feed the unmapped summary, never a block. Token
/// and cost fields stay `Option` so absence survives as `None`, never a fabricated `0`.
#[derive(Debug, Clone)]
pub struct EvidenceRow {
    pub environment: String,
    pub session_id: Option<String>,
    pub ai_start_ts: Option<String>,
    pub ai_end_ts: Option<String>,
    pub total_tokens: Option<i64>,
    pub cost_total: Option<f64>,
    pub cost_currency: Option<String>,
    pub health: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
}

/// Read all imported AI evidence, LEFT-JOINed to the env→project map and the project name. The join
/// mirrors `env_mapping::list_evidence_projects_repo` (D3): association is computed at read time, so
/// changing or clearing a mapping changes only this result — the evidence rows are untouched.
pub fn load_evidence(conn: &Connection) -> rusqlite::Result<Vec<EvidenceRow>> {
    // `trace_id` is not projected (it feeds no block field), but it stays in `ORDER BY` so rows with
    // equal `(environment)` keep a deterministic, stable order for clustering.
    let mut stmt = conn.prepare(
        "SELECT e.environment, e.session_id, e.ai_start_ts, e.ai_end_ts,
                e.total_tokens, e.cost_total, e.cost_currency, e.health,
                m.project_id, p.name
           FROM langfuse_ai_evidence e
           LEFT JOIN langfuse_env_project_map m ON m.environment = e.environment
           LEFT JOIN projects p ON p.id = m.project_id
          ORDER BY e.environment, e.trace_id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(EvidenceRow {
                environment: r.get(0)?,
                session_id: r.get(1)?,
                ai_start_ts: r.get(2)?,
                ai_end_ts: r.get(3)?,
                total_tokens: r.get(4)?,
                cost_total: r.get(5)?,
                cost_currency: r.get(6)?,
                health: r.get(7)?,
                project_id: r.get(8)?,
                project_name: r.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Drop every `pending` suggestion ahead of a regeneration. `accepted`/`dismissed` rows are left
/// untouched — only the recomputed pending set is replaced (design §2 idempotent generation).
pub fn delete_pending(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM time_entry_suggestions WHERE status = 'pending'",
        [],
    )
}

/// Insert a freshly-computed pending suggestion **unless** a decided (`accepted`/`dismissed`) row
/// already exists for the same natural key `(project_id, date, block_start_ts)`. This is the
/// idempotency guard: after `delete_pending`, the only surviving rows are decided ones, so the
/// `NOT EXISTS` check keeps a decided block from being resurrected as a new pending duplicate.
///
/// The explicit `block_start_ts IS NULL` branch is deliberate: SQLite treats `NULL` values in a
/// `UNIQUE` index as distinct, so the table constraint alone would *not* dedupe an all-untimed
/// ("needs manual time") suggestion against an already-dismissed one — this guard does. Returns
/// `true` when a row was inserted.
pub fn insert_if_not_decided(conn: &Connection, s: &Suggestion) -> rusqlite::Result<bool> {
    let inserted = conn.execute(
        "INSERT INTO time_entry_suggestions
            (id, project_id, date, block_start_ts, block_end_ts, duration_minutes,
             trace_count, session_count, total_tokens, cost_total, cost_currency,
             health, confidence, source, reason, status, accepted_entry_id, created_at, updated_at)
         SELECT ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19
          WHERE NOT EXISTS (
              SELECT 1 FROM time_entry_suggestions t
               WHERE t.project_id = ?2
                 AND t.date = ?3
                 AND ((t.block_start_ts IS NULL AND ?4 IS NULL) OR t.block_start_ts = ?4)
                 AND t.status IN ('accepted', 'dismissed')
          )",
        params![
            s.id,
            s.project_id,
            s.date,
            s.block_start_ts,
            s.block_end_ts,
            s.duration_minutes,
            s.trace_count,
            s.session_count,
            s.total_tokens,
            s.cost_total,
            s.cost_currency,
            s.health,
            s.confidence,
            s.source,
            s.reason,
            s.status,
            s.accepted_entry_id,
            s.created_at,
            s.updated_at,
        ],
    )?;
    Ok(inserted > 0)
}

/// Read the current `pending` suggestions, joined to their project name for display, grouped by
/// project → date with untimed ("needs manual time") suggestions sorted last within a day.
pub fn list_pending(conn: &Connection) -> rusqlite::Result<Vec<Suggestion>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.project_id, p.name, s.date, s.block_start_ts, s.block_end_ts,
                s.duration_minutes, s.trace_count, s.session_count, s.total_tokens,
                s.cost_total, s.cost_currency, s.health, s.confidence, s.source, s.reason,
                s.status, s.accepted_entry_id, s.created_at, s.updated_at
           FROM time_entry_suggestions s
           JOIN projects p ON p.id = s.project_id
          WHERE s.status = 'pending'
          ORDER BY p.name, s.date, s.block_start_ts IS NULL, s.block_start_ts",
    )?;
    let rows = stmt
        .query_map([], row_to_suggestion)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Read a single suggestion by id regardless of status (Workstream B accept/dismiss read the current
/// `status`/`accepted_entry_id` to enforce the decided-once rule). Returns `None` when no such row
/// exists. Joined to the project name for a consistent [`Suggestion`] shape.
pub fn get_by_id(conn: &Connection, id: &str) -> rusqlite::Result<Option<Suggestion>> {
    conn.query_row(
        "SELECT s.id, s.project_id, p.name, s.date, s.block_start_ts, s.block_end_ts,
                s.duration_minutes, s.trace_count, s.session_count, s.total_tokens,
                s.cost_total, s.cost_currency, s.health, s.confidence, s.source, s.reason,
                s.status, s.accepted_entry_id, s.created_at, s.updated_at
           FROM time_entry_suggestions s
           JOIN projects p ON p.id = s.project_id
          WHERE s.id = ?1",
        params![id],
        row_to_suggestion,
    )
    .optional()
}

fn row_to_suggestion(r: &rusqlite::Row<'_>) -> rusqlite::Result<Suggestion> {
    Ok(Suggestion {
        id: r.get(0)?,
        project_id: r.get(1)?,
        project_name: r.get(2)?,
        date: r.get(3)?,
        block_start_ts: r.get(4)?,
        block_end_ts: r.get(5)?,
        duration_minutes: r.get(6)?,
        trace_count: r.get(7)?,
        session_count: r.get(8)?,
        total_tokens: r.get(9)?,
        cost_total: r.get(10)?,
        cost_currency: r.get(11)?,
        health: r.get(12)?,
        confidence: r.get(13)?,
        source: r.get(14)?,
        reason: r.get(15)?,
        status: r.get(16)?,
        accepted_entry_id: r.get(17)?,
        created_at: r.get(18)?,
        updated_at: r.get(19)?,
    })
}
