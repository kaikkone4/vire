//! Importer-owned persistence (TASK-019's narrow slice of TASK-004). Additive only — it touches
//! no `projects`/`time_entries` table and stores no credential material in any column.

use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

use super::config::ImporterConfig;
use super::model::{AiEvidence, HealthState};

/// Create the importer tables if absent. Called from `init_db` so a single migration entry point
/// keeps the schema consistent across fresh installs and reopens.
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS langfuse_import_runs (
            id TEXT PRIMARY KEY,
            environment TEXT NOT NULL,
            window_from TEXT,
            window_to TEXT,
            cursor_ts TEXT,
            status TEXT NOT NULL,
            pages_walked INTEGER NOT NULL DEFAULT 0,
            traces_seen INTEGER NOT NULL DEFAULT 0,
            duplicates_suppressed INTEGER NOT NULL DEFAULT 0,
            warnings TEXT,
            started_at TEXT NOT NULL,
            finished_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_import_runs_env_finished
            ON langfuse_import_runs(environment, finished_at);
        CREATE TABLE IF NOT EXISTS langfuse_raw_traces (
            environment TEXT NOT NULL,
            trace_id TEXT NOT NULL,
            payload TEXT NOT NULL,
            imported_at TEXT NOT NULL,
            import_run_id TEXT,
            PRIMARY KEY (environment, trace_id)
        );
        CREATE TABLE IF NOT EXISTS langfuse_ai_evidence (
            environment TEXT NOT NULL,
            trace_id TEXT NOT NULL,
            session_id TEXT,
            ai_start_ts TEXT,
            ai_end_ts TEXT,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            total_tokens INTEGER,
            cost_total REAL,
            cost_currency TEXT,
            health TEXT NOT NULL,
            import_run_id TEXT,
            PRIMARY KEY (environment, trace_id)
        );",
    )?;
    // Additive, idempotent: surface `session_id` on installs whose evidence table predates the
    // TASK-022 column. Duplicate-column on a fresh install is expected and ignored; any other
    // error propagates. The column is nullable and privacy-positive (opaque hashed id, never
    // prompt/content), letting the runtime observer match the normalized row not the raw payload.
    add_column_if_absent(conn, "langfuse_ai_evidence", "session_id", "TEXT")
}

/// `ALTER TABLE … ADD COLUMN` that treats an already-present column as success. SQLite has no
/// `ADD COLUMN IF NOT EXISTS`, so a duplicate-column error is the expected "already migrated" path.
fn add_column_if_absent(
    conn: &Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> rusqlite::Result<()> {
    match conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
        [],
    ) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(_, Some(msg))) if msg.contains("duplicate column name") => {
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// A row of `langfuse_import_runs`, used both for writing a run and reading prior cursor/state.
#[derive(Debug, Clone)]
pub struct ImportRunRecord {
    pub id: String,
    pub environment: String,
    pub window_from: Option<String>,
    pub window_to: Option<String>,
    pub cursor_ts: Option<String>,
    pub status: HealthState,
    pub pages_walked: u32,
    pub traces_seen: u32,
    pub duplicates_suppressed: u32,
    pub warnings: Vec<String>,
    pub started_at: String,
    pub finished_at: String,
}

pub fn insert_import_run(conn: &Connection, run: &ImportRunRecord) -> rusqlite::Result<()> {
    let warnings = serde_json::to_string(&run.warnings).unwrap_or_else(|_| "[]".into());
    conn.execute(
        "INSERT INTO langfuse_import_runs
            (id, environment, window_from, window_to, cursor_ts, status,
             pages_walked, traces_seen, duplicates_suppressed, warnings, started_at, finished_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            run.id,
            run.environment,
            run.window_from,
            run.window_to,
            run.cursor_ts,
            run.status.as_str(),
            run.pages_walked,
            run.traces_seen,
            run.duplicates_suppressed,
            warnings,
            run.started_at,
            run.finished_at,
        ],
    )?;
    Ok(())
}

/// Persist one import run's writes **atomically** (S-3): the raw-trace upserts, AI-evidence
/// upserts, and run-record insert all commit together or not at all. The transaction is created
/// from a shared `&Connection` via `unchecked_transaction`; on any error the `Transaction` is
/// dropped without `commit`, so rusqlite rolls the whole run back — no partially-written run can
/// survive a mid-run failure. The atomicity unit is one import run (one `run_id`) per environment.
pub fn persist_import_run(
    conn: &Connection,
    run: &ImportRunRecord,
    raw: &[(String, String)],
    evidence: &[AiEvidence],
    imported_at: &str,
) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    for (trace_id, payload) in raw {
        upsert_raw_trace(&tx, &run.environment, trace_id, payload, imported_at, &run.id)?;
    }
    for ev in evidence {
        upsert_ai_evidence(&tx, ev, &run.id)?;
    }
    insert_import_run(&tx, run)?;
    tx.commit()
}

/// Most-recent run for an environment (by `finished_at`), used to seed the next cursor/overlap.
pub fn latest_run_for_env(
    conn: &Connection,
    environment: &str,
) -> rusqlite::Result<Option<ImportRunRecord>> {
    conn.query_row(
        "SELECT id, environment, window_from, window_to, cursor_ts, status,
                pages_walked, traces_seen, duplicates_suppressed, warnings, started_at, finished_at
           FROM langfuse_import_runs
          WHERE environment = ?1
          ORDER BY finished_at DESC
          LIMIT 1",
        params![environment],
        row_to_run,
    )
    .optional()
}

/// Most-recent run across all environments, for the read-only health snapshot.
pub fn latest_run(conn: &Connection) -> rusqlite::Result<Option<ImportRunRecord>> {
    conn.query_row(
        "SELECT id, environment, window_from, window_to, cursor_ts, status,
                pages_walked, traces_seen, duplicates_suppressed, warnings, started_at, finished_at
           FROM langfuse_import_runs
          ORDER BY finished_at DESC
          LIMIT 1",
        [],
        row_to_run,
    )
    .optional()
}

fn row_to_run(r: &rusqlite::Row) -> rusqlite::Result<ImportRunRecord> {
    let status: String = r.get(5)?;
    let warnings: String = r.get(9)?;
    Ok(ImportRunRecord {
        id: r.get(0)?,
        environment: r.get(1)?,
        window_from: r.get(2)?,
        window_to: r.get(3)?,
        cursor_ts: r.get(4)?,
        status: HealthState::parse(&status),
        pages_walked: r.get(6)?,
        traces_seen: r.get(7)?,
        duplicates_suppressed: r.get(8)?,
        warnings: serde_json::from_str(&warnings).unwrap_or_default(),
        started_at: r.get(10)?,
        finished_at: r.get(11)?,
    })
}

/// Trace ids already persisted for an environment — the durable half of the dedup key so
/// re-imports and overlap windows are idempotent.
pub fn seen_trace_ids(conn: &Connection, environment: &str) -> rusqlite::Result<HashSet<String>> {
    let mut stmt =
        conn.prepare("SELECT trace_id FROM langfuse_raw_traces WHERE environment = ?1")?;
    let ids = stmt
        .query_map(params![environment], |r| r.get::<_, String>(0))?
        .collect::<Result<HashSet<String>, _>>()?;
    Ok(ids)
}

/// Persist a raw trace payload (local-only evidence; may include prompt/session/metadata per the
/// DEC-020 MVP relaxation). Idempotent on `(environment, trace_id)`. No credential material.
pub fn upsert_raw_trace(
    conn: &Connection,
    environment: &str,
    trace_id: &str,
    payload: &str,
    imported_at: &str,
    import_run_id: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO langfuse_raw_traces (environment, trace_id, payload, imported_at, import_run_id)
         VALUES (?1,?2,?3,?4,?5)
         ON CONFLICT(environment, trace_id) DO UPDATE SET
            payload = excluded.payload,
            imported_at = excluded.imported_at,
            import_run_id = excluded.import_run_id",
        params![environment, trace_id, payload, imported_at, import_run_id],
    )?;
    Ok(())
}

/// Persist a normalized AI-evidence row. Token/cost columns stay `NULL` when absent — absence is
/// never written as `0`.
pub fn upsert_ai_evidence(
    conn: &Connection,
    evidence: &AiEvidence,
    import_run_id: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO langfuse_ai_evidence
            (environment, trace_id, session_id, ai_start_ts, ai_end_ts, prompt_tokens,
             completion_tokens, total_tokens, cost_total, cost_currency, health, import_run_id)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT(environment, trace_id) DO UPDATE SET
            session_id = excluded.session_id,
            ai_start_ts = excluded.ai_start_ts,
            ai_end_ts = excluded.ai_end_ts,
            prompt_tokens = excluded.prompt_tokens,
            completion_tokens = excluded.completion_tokens,
            total_tokens = excluded.total_tokens,
            cost_total = excluded.cost_total,
            cost_currency = excluded.cost_currency,
            health = excluded.health,
            import_run_id = excluded.import_run_id",
        params![
            evidence.environment,
            evidence.trace_id,
            evidence.session_id,
            evidence.ai_start_ts,
            evidence.ai_end_ts,
            evidence.prompt_tokens,
            evidence.completion_tokens,
            evidence.total_tokens,
            evidence.cost_total,
            Option::<String>::None, // cost_currency: reserved; not yet populated
            evidence.health.as_str(),
            import_run_id,
        ],
    )?;
    Ok(())
}

/// Read-only health snapshot for the renderer (`get_langfuse_source_health`). Contains no
/// secrets — only the public base URL, source posture, environments, and the latest run state.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceHealthSnapshot {
    pub base_url: String,
    pub source: String,
    pub environments: Vec<String>,
    pub last_import_at: Option<String>,
    pub latest_trace_ts: Option<String>,
    pub health: String,
    pub message: String,
}

pub fn source_health_snapshot(
    conn: &Connection,
    config: &ImporterConfig,
) -> Result<SourceHealthSnapshot, String> {
    let latest = latest_run(conn).map_err(|e| e.to_string())?;
    let (health, last_import_at, latest_trace_ts) = match &latest {
        Some(run) => (
            run.status,
            Some(run.finished_at.clone()),
            run.cursor_ts.clone(),
        ),
        // No import has run yet: currency/completeness is indeterminate, not zero.
        None => (HealthState::Unknown, None, None),
    };
    Ok(SourceHealthSnapshot {
        base_url: config.base_url.clone(),
        source: config.source.as_str().to_string(),
        environments: config.allowed_environments.clone(),
        last_import_at,
        latest_trace_ts,
        message: snapshot_message(health),
        health: health.as_str().to_string(),
    })
}

/// Snapshot for a disabled integration (TASK-026): no import runs and no probe fires when
/// `langfuse_enabled = false`. Carries the non-secret config for display and an explicit `disabled`
/// state — never a zero AI usage/cost (absence-≠-zero invariant). Built from public config only;
/// no credentials are read.
pub fn disabled_snapshot(config: &ImporterConfig) -> SourceHealthSnapshot {
    SourceHealthSnapshot {
        base_url: config.base_url.clone(),
        source: config.source.as_str().to_string(),
        environments: config.allowed_environments.clone(),
        last_import_at: None,
        latest_trace_ts: None,
        health: "disabled".to_string(),
        message: "Langfuse integration is turned off — enable it in Settings to import AI evidence. \
                  Disabled is not zero AI usage or cost."
            .to_string(),
    }
}

fn snapshot_message(health: HealthState) -> String {
    match health {
        HealthState::Healthy => "Langfuse AI evidence is current.",
        HealthState::Unavailable => {
            "Local Docker/Langfuse is unavailable — AI usage and cost are unknown, not zero."
        }
        HealthState::Stale => "Latest Langfuse import is stale — totals may be out of date.",
        HealthState::Unknown => "Langfuse source state is unknown — run an import to refresh.",
        HealthState::Missing => "No matching Langfuse traces were found for the window.",
        HealthState::WrongEnv => {
            "Traces were found in an unexpected environment — surfaced for review."
        }
        HealthState::AuthOrNetworkError => {
            "Could not reach Langfuse (auth or network) — AI totals are unknown, not zero."
        }
        HealthState::SchemaChanged => "Langfuse trace shape changed — totals withheld for review.",
        HealthState::Delayed => "Late-arriving Langfuse traces were reconciled.",
        HealthState::Duplicate => "Duplicate Langfuse traces were de-duplicated.",
    }
    .to_string()
}
