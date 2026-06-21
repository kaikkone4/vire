//! Active-window evidence persistence (TASK-046). Three additive tables, typed write/read/prune
//! APIs. The allowlist is the schema — no prohibited column exists in any table. The title
//! redaction gate lives inside the write API so no caller can bypass it.

use rusqlite::{params, Connection};

use super::model::{
    CaptureHealthEvent, EvidenceBlock, EvidenceBlockView, PruneStats, RawObservation, TitleMode,
    title_state,
};

/// Create the three `active_window_*` tables and their indexes if absent.
/// Idempotent — safe to call on every startup. Carries no FK into `time_entries` or `projects`
/// so ordering within `init_db` is unconstrained; placed last.
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS active_window_raw_evidence (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            sample_ts      TEXT NOT NULL,
            day            TEXT NOT NULL,
            app_name       TEXT,
            app_bundle_id  TEXT,
            window_title   TEXT,
            title_state    TEXT NOT NULL,
            idle_state     TEXT NOT NULL,
            source         TEXT NOT NULL,
            capture_health TEXT,
            observed_at    TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_aw_raw_day
            ON active_window_raw_evidence(day);
        CREATE INDEX IF NOT EXISTS idx_aw_raw_samplets
            ON active_window_raw_evidence(sample_ts);

        CREATE TABLE IF NOT EXISTS active_window_evidence (
            id               TEXT PRIMARY KEY,
            day              TEXT NOT NULL,
            start_ts         TEXT NOT NULL,
            end_ts           TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL,
            app_name         TEXT,
            app_bundle_id    TEXT,
            window_title     TEXT,
            title_state      TEXT NOT NULL,
            idle_state       TEXT NOT NULL,
            source           TEXT NOT NULL,
            capture_health   TEXT,
            review_state     TEXT NOT NULL DEFAULT 'unreviewed',
            created_at       TEXT NOT NULL,
            updated_at       TEXT NOT NULL,
            UNIQUE (day, start_ts, app_bundle_id)
        );
        CREATE INDEX IF NOT EXISTS idx_aw_evidence_day
            ON active_window_evidence(day, start_ts);

        CREATE TABLE IF NOT EXISTS active_window_capture_health (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            day         TEXT NOT NULL,
            start_ts    TEXT NOT NULL,
            end_ts      TEXT,
            state       TEXT NOT NULL,
            detail      TEXT,
            source      TEXT NOT NULL,
            observed_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_aw_health_day
            ON active_window_capture_health(day, start_ts);",
    )
}

/// Insert a per-sample raw observation. Title gate is applied before any SQL bind:
/// under `Redacted` mode the raw title is discarded and `title_state` is forced to `'redacted'`.
/// Logs counts/states only — never a title value.
pub fn insert_raw_observation(
    conn: &Connection,
    obs: &RawObservation,
    mode: TitleMode,
    observed_at: &str,
) -> rusqlite::Result<()> {
    let (persisted_title, effective_state) =
        apply_title_gate(obs.window_title.as_deref(), &obs.title_state, mode);
    conn.execute(
        "INSERT INTO active_window_raw_evidence
            (sample_ts, day, app_name, app_bundle_id, window_title, title_state,
             idle_state, source, capture_health, observed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            obs.sample_ts,
            obs.day,
            obs.app_name,
            obs.app_bundle_id,
            persisted_title,
            effective_state,
            obs.idle_state,
            obs.source,
            obs.capture_health,
            observed_at,
        ],
    )?;
    Ok(())
}

/// Upsert a normalized evidence block, keyed on `(day, start_ts, app_bundle_id)`.
/// Re-coalescing a day is idempotent — same key re-resolves in place, no duplicate rows.
/// Title gate applied identically to the raw path.
pub fn upsert_evidence_block(
    conn: &Connection,
    block: &EvidenceBlock,
    mode: TitleMode,
    now: &str,
) -> rusqlite::Result<()> {
    let (persisted_title, effective_state) =
        apply_title_gate(block.window_title.as_deref(), &block.title_state, mode);
    conn.execute(
        "INSERT INTO active_window_evidence
            (id, day, start_ts, end_ts, duration_seconds, app_name, app_bundle_id,
             window_title, title_state, idle_state, source, capture_health, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13)
         ON CONFLICT(day, start_ts, app_bundle_id) DO UPDATE SET
            end_ts           = excluded.end_ts,
            duration_seconds = excluded.duration_seconds,
            app_name         = excluded.app_name,
            window_title     = excluded.window_title,
            title_state      = excluded.title_state,
            idle_state       = excluded.idle_state,
            source           = excluded.source,
            capture_health   = excluded.capture_health,
            updated_at       = excluded.updated_at",
        params![
            block.id,
            block.day,
            block.start_ts,
            block.end_ts,
            block.duration_seconds,
            block.app_name,
            block.app_bundle_id,
            persisted_title,
            effective_state,
            block.idle_state,
            block.source,
            block.capture_health,
            now,
        ],
    )?;
    Ok(())
}

/// Record a degraded capture state as a first-class row (never dropped silently).
/// `detail` must hold only a bounded coarse reason code — never a title, path, command, or secret.
pub fn record_capture_health(
    conn: &Connection,
    ev: &CaptureHealthEvent,
    observed_at: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO active_window_capture_health
            (day, start_ts, end_ts, state, detail, source, observed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            ev.day,
            ev.start_ts,
            ev.end_ts,
            ev.state,
            ev.detail,
            ev.source,
            observed_at,
        ],
    )?;
    Ok(())
}

/// Return normalized evidence blocks whose `day` falls in `[from, to]` (inclusive).
/// Under `Redacted` mode, `window_title` is always `None` in the returned views — the caller
/// never receives a raw title.
pub fn evidence_blocks_in_range(
    conn: &Connection,
    from: &str,
    to: &str,
    mode: TitleMode,
) -> rusqlite::Result<Vec<EvidenceBlockView>> {
    let mut stmt = conn.prepare(
        "SELECT id, day, start_ts, end_ts, duration_seconds, app_name, app_bundle_id,
                window_title, title_state, idle_state, source, capture_health, review_state
           FROM active_window_evidence
          WHERE day >= ?1 AND day <= ?2
          ORDER BY day, start_ts",
    )?;
    let rows = stmt
        .query_map(params![from, to], |row| {
            let raw_title: Option<String> = row.get(7)?;
            let title_st: String = row.get(8)?;
            let projected_title = match mode {
                TitleMode::Stored => raw_title,
                TitleMode::Redacted => None,
            };
            Ok(EvidenceBlockView {
                id: row.get(0)?,
                day: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: row.get(3)?,
                duration_seconds: row.get(4)?,
                app_name: row.get(5)?,
                app_bundle_id: row.get(6)?,
                window_title: projected_title,
                title_state: title_st,
                idle_state: row.get(9)?,
                source: row.get(10)?,
                capture_health: row.get(11)?,
                review_state: row.get(12)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Return capture health events whose `day` falls in `[from, to]` (inclusive).
pub fn capture_health_in_range(
    conn: &Connection,
    from: &str,
    to: &str,
) -> rusqlite::Result<Vec<CaptureHealthEvent>> {
    let mut stmt = conn.prepare(
        "SELECT day, start_ts, end_ts, state, detail, source
           FROM active_window_capture_health
          WHERE day >= ?1 AND day <= ?2
          ORDER BY day, start_ts",
    )?;
    let rows = stmt
        .query_map(params![from, to], |row| {
            Ok(CaptureHealthEvent {
                day: row.get(0)?,
                start_ts: row.get(1)?,
                end_ts: row.get(2)?,
                state: row.get(3)?,
                detail: row.get(4)?,
                source: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Delete only `active_window_*` rows whose `day` is older than `now − retention_days`.
/// Touches no other table (C6: approved human time is never mutated by retention).
/// Returns per-table deleted-row counts.
pub fn prune_expired(
    conn: &Connection,
    now: &str,
    retention_days: i64,
) -> rusqlite::Result<PruneStats> {
    let interval = format!("-{} days", retention_days);
    let raw_deleted = conn.execute(
        "DELETE FROM active_window_raw_evidence WHERE day < date(?1, ?2)",
        params![now, interval],
    )?;
    let evidence_deleted = conn.execute(
        "DELETE FROM active_window_evidence WHERE day < date(?1, ?2)",
        params![now, interval],
    )?;
    let health_deleted = conn.execute(
        "DELETE FROM active_window_capture_health WHERE day < date(?1, ?2)",
        params![now, interval],
    )?;
    Ok(PruneStats {
        raw_evidence_deleted: raw_deleted,
        evidence_deleted,
        capture_health_deleted: health_deleted,
    })
}

/// Apply the title redaction gate. Returns `(persisted_title, effective_title_state)`.
/// Under `Redacted` mode: if the caller observed a title (`title_state='captured'`), the value
/// is discarded and the state becomes `'redacted'`. All non-`captured` states pass through
/// unchanged (absence states, empty) because there is no title to discard.
fn apply_title_gate(
    title: Option<&str>,
    state: &str,
    mode: TitleMode,
) -> (Option<String>, String) {
    match mode {
        TitleMode::Stored => (title.map(str::to_owned), state.to_owned()),
        TitleMode::Redacted => {
            if state == title_state::CAPTURED {
                (None, title_state::REDACTED.to_owned())
            } else {
                (None, state.to_owned())
            }
        }
    }
}
