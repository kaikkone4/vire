//! Active-window evidence persistence (TASK-046). Three additive tables, typed write/read/prune
//! APIs. The allowlist is the schema — no prohibited column exists in any table. The title
//! redaction gate lives inside the write API so no caller can bypass it.

use rusqlite::{params, Connection};

use super::model::{
    health_state, idle_state, source, title_state, CaptureHealthEvent, CaptureStatusView,
    EvidenceBlock, EvidenceBlockView, HealthMarker, PruneStats, RawObservation, TitleMode,
};

// ----- controlled vocabulary constants ---------------------------------------------------------

const TITLE_STATE_VOCAB: &[&str] = &[
    title_state::CAPTURED,
    title_state::REDACTED,
    title_state::ABSENT_NO_PERMISSION,
    title_state::ABSENT_NO_WINDOW,
    title_state::ABSENT_UNSUPPORTED,
    title_state::EMPTY,
];

const IDLE_STATE_VOCAB: &[&str] = &[
    idle_state::ACTIVE,
    idle_state::IDLE_CANDIDATE,
    idle_state::AWAY,
];

const SOURCE_VOCAB: &[&str] = &[source::NSWORKSPACE, source::ACCESSIBILITY, source::QUARTZ];

const HEALTH_STATE_VOCAB: &[&str] = &[
    health_state::AX_PERMISSION_DENIED,
    health_state::SCREEN_RECORDING_ABSENT,
    health_state::NO_FOCUSED_WINDOW,
    health_state::WINDOW_UNAVAILABLE,
    health_state::TITLE_EMPTY,
    health_state::SAMPLING_GAP,
    health_state::SYSTEM_SLEEP,
    health_state::SYSTEM_WAKE,
    health_state::SCREEN_LOCKED,
    health_state::HELPER_CRASH,
    health_state::HELPER_RESTART,
    health_state::NO_GUI_SESSION,
];

/// Maximum byte length for the `detail` field. Coarse reason codes only; never a title or secret.
pub const MAX_DETAIL_BYTES: usize = 200;

fn check_vocab(value: &str, allowed: &[&str], field: &str) -> rusqlite::Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(rusqlite::Error::InvalidParameterName(format!(
            "{field} {value:?} is not in the controlled vocabulary"
        )))
    }
}

fn check_detail(detail: Option<&str>) -> rusqlite::Result<()> {
    if let Some(d) = detail {
        if d.len() > MAX_DETAIL_BYTES {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "detail must be {MAX_DETAIL_BYTES} bytes or fewer (got {})",
                d.len()
            )));
        }
    }
    Ok(())
}

// ----- Sentinel for nullable conflict key -------------------------------------------------------
//
// SQLite treats NULL as distinct in UNIQUE constraints, so two rows with
// (day, start_ts, app_bundle_id=NULL) would satisfy `UNIQUE(day, start_ts, app_bundle_id)`
// without conflict. To preserve idempotent upsert semantics for apps with no bundle ID, we
// normalise None → "" (empty string sentinel) before binding the conflict-key column, and
// map "" → None on read. macOS bundle IDs are never empty, so the sentinel is unambiguous.
const BUNDLE_NULL_SENTINEL: &str = "";

// ----- migrate ---------------------------------------------------------------------------------

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
            app_bundle_id    TEXT NOT NULL DEFAULT '',
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

// ----- write APIs ------------------------------------------------------------------------------

/// Insert a per-sample raw observation. Vocabulary-validates `title_state`, `idle_state`, and
/// `source` before any SQL bind. Title gate is applied: under `Redacted` mode the raw title is
/// discarded and `title_state` is forced to `'redacted'`. Logs counts/states only — never a title.
pub fn insert_raw_observation(
    conn: &Connection,
    obs: &RawObservation,
    mode: TitleMode,
    observed_at: &str,
) -> rusqlite::Result<()> {
    check_vocab(&obs.title_state, TITLE_STATE_VOCAB, "title_state")?;
    check_vocab(&obs.idle_state, IDLE_STATE_VOCAB, "idle_state")?;
    check_vocab(&obs.source, SOURCE_VOCAB, "source")?;
    if let Some(h) = obs.capture_health.as_deref() {
        check_vocab(h, HEALTH_STATE_VOCAB, "capture_health")?;
    }
    let (persisted_title, effective_state) =
        apply_title_gate(obs.window_title.as_deref(), &obs.title_state, mode)?;
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
/// Vocabulary-validates `title_state`, `idle_state`, and `source` before any SQL bind.
/// `app_bundle_id = None` is normalised to the empty-string sentinel so that the UNIQUE
/// constraint detects duplicate no-bundle blocks (SQLite NULLs are distinct in UNIQUE indexes).
/// Title gate applied identically to the raw path.
pub fn upsert_evidence_block(
    conn: &Connection,
    block: &EvidenceBlock,
    mode: TitleMode,
    now: &str,
) -> rusqlite::Result<()> {
    check_vocab(&block.title_state, TITLE_STATE_VOCAB, "title_state")?;
    check_vocab(&block.idle_state, IDLE_STATE_VOCAB, "idle_state")?;
    check_vocab(&block.source, SOURCE_VOCAB, "source")?;
    if let Some(h) = block.capture_health.as_deref() {
        check_vocab(h, HEALTH_STATE_VOCAB, "capture_health")?;
    }
    let (persisted_title, effective_state) =
        apply_title_gate(block.window_title.as_deref(), &block.title_state, mode)?;
    let bundle_key = block
        .app_bundle_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(BUNDLE_NULL_SENTINEL);
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
            bundle_key,
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
/// Vocabulary-validates `state` and `source`. `detail` must be a bounded coarse reason code
/// (≤ `MAX_DETAIL_BYTES` bytes) — never a title, path, command, or secret.
pub fn record_capture_health(
    conn: &Connection,
    ev: &CaptureHealthEvent,
    observed_at: &str,
) -> rusqlite::Result<()> {
    check_vocab(&ev.state, HEALTH_STATE_VOCAB, "state")?;
    check_vocab(&ev.source, SOURCE_VOCAB, "source")?;
    check_detail(ev.detail.as_deref())?;
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

// ----- read APIs -------------------------------------------------------------------------------

/// Return normalized evidence blocks whose `day` falls in `[from, to]` (inclusive).
/// Under `Redacted` mode, `window_title` is always `None` in the returned views — the caller
/// never receives a raw title. The empty-string bundle sentinel is mapped back to `None`.
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
            let raw_bundle: Option<String> = row.get(6)?;
            // Map the empty-string sentinel back to None for API consumers.
            let app_bundle_id = raw_bundle.filter(|s| !s.is_empty());
            Ok(EvidenceBlockView {
                id: row.get(0)?,
                day: row.get(1)?,
                start_ts: row.get(2)?,
                end_ts: row.get(3)?,
                duration_seconds: row.get(4)?,
                app_name: row.get(5)?,
                app_bundle_id,
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

// ----- status projection (read-only, TASK-056 B) -----------------------------------------------

/// How many most-recent `capture_health` rows the status snapshot returns in `recent_health`.
pub const RECENT_HEALTH_LIMIT: i64 = 10;

/// Read-only capture status/health snapshot over the three existing tables. No schema change and no
/// write path: pure `MAX`/`COUNT`/state-code reads over allowlisted columns, so the result can carry
/// only counts, timestamps, and bounded coarse state/detail codes — never a title, path, URL,
/// command, or secret. `now_day` and `retention_from_day` are `YYYY-MM-DD` day keys computed by the
/// caller so the projection stays a pure function of `(rows, day bounds)` and is trivially testable.
pub fn capture_status_snapshot(
    conn: &Connection,
    now_day: &str,
    retention_from_day: &str,
) -> rusqlite::Result<CaptureStatusView> {
    let last_sample_ts: Option<String> = conn.query_row(
        "SELECT MAX(sample_ts) FROM active_window_raw_evidence",
        [],
        |r| r.get::<_, Option<String>>(0),
    )?;

    let samples_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM active_window_raw_evidence WHERE day = ?1",
        params![now_day],
        |r| r.get(0),
    )?;

    let evidence_blocks_retained: i64 = conn.query_row(
        "SELECT COUNT(*) FROM active_window_evidence WHERE day >= ?1",
        params![retention_from_day],
        |r| r.get(0),
    )?;

    let open_health = {
        let mut stmt = conn.prepare(
            "SELECT state, start_ts, detail FROM active_window_capture_health
              WHERE end_ts IS NULL
              ORDER BY start_ts DESC, id DESC",
        )?;
        let rows = stmt
            .query_map([], map_health_marker)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let recent_health = {
        let mut stmt = conn.prepare(
            "SELECT state, start_ts, detail FROM active_window_capture_health
              ORDER BY start_ts DESC, id DESC
              LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![RECENT_HEALTH_LIMIT], map_health_marker)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    Ok(CaptureStatusView {
        last_sample_ts,
        samples_today,
        evidence_blocks_retained,
        open_health,
        recent_health,
    })
}

fn map_health_marker(row: &rusqlite::Row) -> rusqlite::Result<HealthMarker> {
    Ok(HealthMarker {
        state: row.get(0)?,
        since_or_start_ts: row.get(1)?,
        detail: row.get(2)?,
    })
}

// ----- retention -------------------------------------------------------------------------------

/// Delete only `active_window_*` rows whose per-table timestamp is strictly before
/// `datetime(now, -retention_days days)`. Uses each table's own precise timestamp column
/// (`sample_ts` / `end_ts` / `start_ts`) rather than the coarser `day` column so that the
/// retention boundary is exact and no row on the cutoff day is incorrectly retained or removed.
/// Wraps all three deletes in a single transaction so a failure cannot leave a partial prune.
/// Touches no other table (C6: approved human time is never mutated by retention).
/// Returns per-table deleted-row counts.
pub fn prune_expired(
    conn: &Connection,
    now: &str,
    retention_days: i64,
) -> rusqlite::Result<PruneStats> {
    let interval = format!("-{} days", retention_days);
    let tx = conn.unchecked_transaction()?;
    let raw_deleted = tx.execute(
        "DELETE FROM active_window_raw_evidence
          WHERE datetime(sample_ts) < datetime(?1, ?2)",
        params![now, interval],
    )?;
    let evidence_deleted = tx.execute(
        "DELETE FROM active_window_evidence
          WHERE datetime(end_ts) < datetime(?1, ?2)",
        params![now, interval],
    )?;
    let health_deleted = tx.execute(
        "DELETE FROM active_window_capture_health
          WHERE datetime(start_ts) < datetime(?1, ?2)",
        params![now, interval],
    )?;
    tx.commit()?;
    Ok(PruneStats {
        raw_evidence_deleted: raw_deleted,
        evidence_deleted,
        capture_health_deleted: health_deleted,
    })
}

// ----- title gate ------------------------------------------------------------------------------

/// Apply the title redaction gate. Returns `(persisted_title, effective_title_state)`.
///
/// Classifies the observed title into three cases — `Present` = `Some(non-empty)`, `Empty` =
/// `Some("")`, `Absent` = `None` — then applies the §8.2 total truth table (arch-review).
/// Returns `Err(InvalidParameterName)` for any contradictory (caller-state, title) pair,
/// surfacing a capture-side bug fail-closed. Both callers propagate with `?`.
///
/// Invariant: a stored `window_title IS NOT NULL` ⟺ `title_state='captured'` under Stored mode;
/// every other state stores `window_title = NULL`.
fn apply_title_gate(
    title: Option<&str>,
    state: &str,
    mode: TitleMode,
) -> rusqlite::Result<(Option<String>, String)> {
    let reject = |msg: &str| Err(rusqlite::Error::InvalidParameterName(msg.to_owned()));

    let is_present = matches!(title, Some(t) if !t.is_empty());
    let is_empty_str = matches!(title, Some(t) if t.is_empty());

    if state == title_state::CAPTURED {
        if is_present {
            return match mode {
                TitleMode::Stored => {
                    Ok((title.map(str::to_owned), title_state::CAPTURED.to_owned()))
                }
                TitleMode::Redacted => Ok((None, title_state::REDACTED.to_owned())),
            };
        }
        if is_empty_str {
            // Some("") reclassified as Empty — not a stored title.
            return Ok((None, title_state::EMPTY.to_owned()));
        }
        return reject(
            "title_state='captured' requires a non-empty title (capture-side invariant violated)",
        );
    }
    if state == title_state::EMPTY {
        return if is_empty_str {
            Ok((None, title_state::EMPTY.to_owned()))
        } else {
            reject("title_state='empty' requires Some(\"\") as the title value")
        };
    }
    if state == title_state::ABSENT_NO_PERMISSION
        || state == title_state::ABSENT_NO_WINDOW
        || state == title_state::ABSENT_UNSUPPORTED
    {
        return if title.is_none() {
            Ok((None, state.to_owned()))
        } else {
            reject("absence title_state requires title=None")
        };
    }
    if state == title_state::REDACTED {
        return if title.is_none() {
            Ok((None, title_state::REDACTED.to_owned()))
        } else {
            reject("title_state='redacted' requires title=None (idempotent re-upsert only)")
        };
    }
    // Unreachable when check_vocab ran first; fail-closed for defensive depth.
    reject(&format!("unhandled title_state: {state:?}"))
}
