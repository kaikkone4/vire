mod active_window;
mod env_mapping;
mod langfuse;
mod runtime_observer;
mod settings;
mod suggestions;

use chrono::{Local, NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex, MutexGuard, TryLockError},
    time::Duration,
};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use langfuse::store::SourceHealthSnapshot;
use runtime_observer::model::RuntimeReconciliationSnapshot;

struct AppState {
    db: Mutex<Connection>,
    db_path: PathBuf,
    import_lock: Arc<Mutex<()>>,
}

type CmdResult<T> = Result<T, String>;

fn db_conn<'a>(state: &'a State<'a, AppState>) -> CmdResult<MutexGuard<'a, Connection>> {
    state.db.lock().map_err(|_| {
        "Database lock is unavailable after an internal error; please restart Vire".to_string()
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub notes: Option<String>,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_minutes: i64,
    pub note: Option<String>,
    /// Provenance tag (DEC-003): `'manual'` for hand-entered time, `'ai_suggested'` for an accepted
    /// suggestion. Manual entries never expose it on input; it defaults to `'manual'` at the DB layer.
    pub origin: String,
    /// AI cost carried over from the accepted suggestion (TASK-034 B, DEC-003). `None` for manual
    /// entries and for AI entries whose source reported no cost — absence ≠ zero (DEC-004), so it
    /// renders "—", never 0. `cost_currency` is usually `None` (not source-derivable — see design §4).
    pub cost_total: Option<f64>,
    pub cost_currency: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInput {
    pub name: String,
    pub notes: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntryInput {
    pub project_id: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
    pub note: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRow {
    pub project_id: String,
    pub project_name: String,
    /// Human-entered time only (origin='manual'). Meaning is unchanged from before TASK-032 so prior
    /// numbers never silently shift: AI-suggested time is reported separately in `ai_minutes` (DEC-003).
    pub duration_minutes: i64,
    /// Accepted AI-suggested time (origin='ai_suggested'), kept distinct so it is never tallied into the
    /// billable human total.
    pub ai_minutes: i64,
    /// Accepted AI-suggested cost (origin='ai_suggested'), summed separately and never folded into a
    /// human total (TASK-034 B, DEC-003). `None` (→ "—") when no AI entry in range carries a cost —
    /// absence ≠ zero (DEC-004). `ai_cost_currency` is usually `None` (not source-derivable — design §4).
    pub ai_cost_total: Option<f64>,
    pub ai_cost_currency: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatus {
    pub mode: String,
    pub message: String,
}

const MAX_PROJECT_NAME_LEN: usize = 120;
const MAX_PROJECT_NOTES_LEN: usize = 5000;
const MAX_ENTRY_NOTE_LEN: usize = 2000;

fn now() -> String {
    Local::now()
        .naive_local()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn init_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS projects (
          id TEXT PRIMARY KEY, name TEXT NOT NULL, notes TEXT, archived INTEGER NOT NULL DEFAULT 0,
          created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS time_entries (
          id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id), date TEXT NOT NULL,
          start_time TEXT NOT NULL, end_time TEXT NOT NULL, duration_minutes INTEGER NOT NULL,
          note TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        INSERT OR IGNORE INTO settings(key, value) VALUES ('capture_status', 'manual_mode_deferred');
        CREATE INDEX IF NOT EXISTS idx_entries_date_project ON time_entries(date, project_id);")?;
    // Additive provenance tag (TASK-032 B1, DEC-003). Idempotent via the shared duplicate-column-tolerant
    // helper; existing rows backfill to 'manual' so prior human totals are unchanged. Accepted AI
    // suggestions are the only writer of 'ai_suggested'.
    langfuse::store::add_column_if_absent(
        conn,
        "time_entries",
        "origin",
        "TEXT NOT NULL DEFAULT 'manual'",
    )?;
    // Additive AI cost provenance (TASK-034 B1, DEC-003 completion). Nullable: a manual entry has no AI
    // cost and existing rows backfill to NULL — absence ≠ zero (DEC-004), so NULL renders "—", never 0.
    // Currency is not source-derivable (Langfuse carries no per-call currency) and is usually NULL.
    langfuse::store::add_column_if_absent(conn, "time_entries", "cost_total", "REAL")?;
    langfuse::store::add_column_if_absent(conn, "time_entries", "cost_currency", "TEXT")?;
    langfuse::store::migrate(conn)?;
    // Additive env→project map; created after `projects` exists so its FK resolves (TASK-027 D1).
    env_mapping::migrate(conn)?;
    runtime_observer::store::migrate(conn)?;
    // Additive suggestions table; created after `projects`/`time_entries`/`env_mapping` so its
    // `project_id` FK resolves and the read-time evidence join has its inputs (TASK-032 A2).
    suggestions::store::migrate(conn)?;
    // Additive active-window evidence tables (TASK-046). No FK into existing tables so ordering
    // is unconstrained; placed last. App behavior stays identical — tables start empty.
    active_window::store::migrate(conn)?;
    // Startup housekeeping: prune expired active-window rows once per launch (C6 — touches only
    // active_window_* tables, never time_entries or any approved summary).
    let aw_cfg = active_window::config::ActiveWindowConfig::from_settings(conn)?;
    active_window::store::prune_expired(conn, &now(), aw_cfg.retention_days)?;
    Ok(())
}

fn clean_opt(s: Option<String>) -> Option<String> {
    s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })
}
fn validate_len(value: &str, max: usize, label: &str) -> Result<(), String> {
    if value.chars().count() > max {
        Err(format!("{label} must be {max} characters or fewer"))
    } else {
        Ok(())
    }
}
fn validate_project_input(input: ProjectInput) -> Result<(String, Option<String>), String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("Project name cannot be empty".into());
    }
    validate_len(&name, MAX_PROJECT_NAME_LEN, "Project name")?;
    let notes = clean_opt(input.notes);
    if let Some(n) = &notes {
        validate_len(n, MAX_PROJECT_NOTES_LEN, "Project notes")?;
    }
    Ok((name, notes))
}
fn validate_entry_note(note: Option<String>) -> Result<Option<String>, String> {
    let note = clean_opt(note);
    if let Some(n) = &note {
        validate_len(n, MAX_ENTRY_NOTE_LEN, "Entry note")?;
    }
    Ok(note)
}
fn parse_duration(date: &str, start: &str, end: &str) -> Result<i64, String> {
    let s = NaiveDateTime::parse_from_str(&format!("{} {}", date, start), "%Y-%m-%d %H:%M")
        .map_err(|_| "Start date/time must be valid".to_string())?;
    let e = NaiveDateTime::parse_from_str(&format!("{} {}", date, end), "%Y-%m-%d %H:%M")
        .map_err(|_| "End date/time must be valid".to_string())?;
    let mins = (e - s).num_minutes();
    if mins <= 0 {
        return Err("End time must be after start time".into());
    }
    Ok(mins)
}
/// DEC-034/035: keep an accepted suggestion from ever storing a zero/negative span, staying inside the
/// same-day `date + HH:MM` model. The engine records block bounds at second precision and a
/// `max(1, round)` duration, but accept re-derives start/end at `HH:MM` (`hhmm_from_block_ts`); a block
/// contained in a single clock minute therefore collapses to `start == end`, which `parse_duration`
/// would reject. When (and only when) the derived `end` is not strictly after `start`, normalize the
/// span to `minutes` (the engine's already-valid `duration_minutes` ≥ 1, or 1 when absent):
/// - **non-boundary** → anchor on the start: `(start, start + minutes)` (forward bump);
/// - **day's last minute** (`start + minutes` would cross midnight, i.e. `start == 23:59`) → anchor on
///   the end instead, where no later same-day end exists: `(23:59 - minutes, 23:59)`, flooring the
///   derived start at `00:00`. The only realistic case is a 1-minute span from `23:59` → `23:58 → 23:59`.
///
/// A positive span (including an explicit user edit) is returned untouched, and a malformed `start`/`end`
/// still surfaces its validation error rather than being silently rewritten. The manual-entry path is
/// unaffected and still rejects `start == end` as a human typo.
fn normalize_same_minute_span(
    date: &str,
    start: &str,
    end: &str,
    duration_minutes: Option<i64>,
) -> Result<(String, String), String> {
    let s = NaiveDateTime::parse_from_str(&format!("{} {}", date, start), "%Y-%m-%d %H:%M")
        .map_err(|_| "Start date/time must be valid".to_string())?;
    let e = NaiveDateTime::parse_from_str(&format!("{} {}", date, end), "%Y-%m-%d %H:%M")
        .map_err(|_| "End date/time must be valid".to_string())?;
    if (e - s).num_minutes() > 0 {
        return Ok((start.to_string(), end.to_string()));
    }
    let minutes = duration_minutes.filter(|m| *m > 0).unwrap_or(1);
    let bumped = s + chrono::Duration::minutes(minutes);
    if bumped.date() == s.date() {
        // Non-boundary: anchor on the start, end = start + minutes (same day).
        return Ok((start.to_string(), bumped.format("%H:%M").to_string()));
    }
    // Day's last minute: no later same-day end exists, so anchor on the end at 23:59 and derive the
    // start backward (floor at 00:00 for the unrealistic case of minutes spanning the whole day).
    let end_dt = s.date().and_hms_opt(23, 59, 0).unwrap();
    let start_dt = end_dt - chrono::Duration::minutes(minutes);
    let start_dt = if start_dt.date() == s.date() {
        start_dt
    } else {
        s.date().and_hms_opt(0, 0, 0).unwrap()
    };
    Ok((
        start_dt.format("%H:%M").to_string(),
        end_dt.format("%H:%M").to_string(),
    ))
}
fn parse_date(date: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| "Date must use YYYY-MM-DD".into())
}
fn validate_date(date: &str) -> Result<(), String> {
    parse_date(date).map(|_| ())
}
fn validate_date_range(start: &str, end: &str) -> Result<(), String> {
    let s = parse_date(start)?;
    let e = parse_date(end)?;
    if s > e {
        Err("Start date must be on or before end date".into())
    } else {
        Ok(())
    }
}
fn project_exists_active(conn: &Connection, id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT archived FROM projects WHERE id=?1",
        params![id],
        |r| r.get::<_, i64>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
    .map(|v| v == Some(0))
}
fn existing_entry_project(conn: &Connection, id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT project_id FROM time_entries WHERE id=?1",
        params![id],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn create_project_repo(conn: &Connection, input: ProjectInput) -> Result<Project, String> {
    let (name, notes) = validate_project_input(input)?;
    let id = Uuid::new_v4().to_string();
    let ts = now();
    conn.execute("INSERT INTO projects(id,name,notes,archived,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![id, name, notes, ts]).map_err(|e| e.to_string())?;
    get_project(conn, &id)
}
fn get_project(conn: &Connection, id: &str) -> Result<Project, String> {
    conn.query_row(
        "SELECT id,name,notes,archived,created_at,updated_at FROM projects WHERE id=?1",
        params![id],
        |r| {
            Ok(Project {
                id: r.get(0)?,
                name: r.get(1)?,
                notes: r.get(2)?,
                archived: r.get::<_, i64>(3)? != 0,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}
pub fn update_project_repo(
    conn: &Connection,
    id: String,
    input: ProjectInput,
) -> Result<Project, String> {
    let (name, notes) = validate_project_input(input)?;
    conn.execute(
        "UPDATE projects SET name=?1, notes=?2, updated_at=?3 WHERE id=?4",
        params![name, notes, now(), id],
    )
    .map_err(|e| e.to_string())?;
    get_project(conn, &id)
}
pub fn archive_project_repo(conn: &Connection, id: String) -> Result<Project, String> {
    conn.execute(
        "UPDATE projects SET archived=1, updated_at=?1 WHERE id=?2",
        params![now(), id],
    )
    .map_err(|e| e.to_string())?;
    get_project(conn, &id)
}
pub fn list_projects_repo(
    conn: &Connection,
    include_archived: bool,
) -> Result<Vec<Project>, String> {
    let sql = if include_archived {
        "SELECT id,name,notes,archived,created_at,updated_at FROM projects ORDER BY archived,name"
    } else {
        "SELECT id,name,notes,archived,created_at,updated_at FROM projects WHERE archived=0 ORDER BY name"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Project {
                id: r.get(0)?,
                name: r.get(1)?,
                notes: r.get(2)?,
                archived: r.get::<_, i64>(3)? != 0,
                created_at: r.get(4)?,
                updated_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    rows
}

pub fn create_entry_repo(conn: &Connection, input: TimeEntryInput) -> Result<TimeEntry, String> {
    validate_date(&input.date)?;
    let duration = parse_duration(&input.date, &input.start_time, &input.end_time)?;
    if !project_exists_active(conn, &input.project_id)? {
        return Err("New time entries must reference an active project".into());
    }
    let id = Uuid::new_v4().to_string();
    let ts = now();
    let note = validate_entry_note(input.note)?;
    conn.execute("INSERT INTO time_entries(id,project_id,date,start_time,end_time,duration_minutes,note,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)", params![id,input.project_id,input.date,input.start_time,input.end_time,duration,note,ts]).map_err(|e| e.to_string())?;
    get_entry(conn, &id)
}
fn get_entry(conn: &Connection, id: &str) -> Result<TimeEntry, String> {
    conn.query_row("SELECT e.id,e.project_id,p.name,e.date,e.start_time,e.end_time,e.duration_minutes,e.note,e.origin,e.cost_total,e.cost_currency,e.created_at,e.updated_at FROM time_entries e JOIN projects p ON p.id=e.project_id WHERE e.id=?1", params![id], row_to_entry).map_err(|e| e.to_string())
}
fn row_to_entry(r: &rusqlite::Row) -> rusqlite::Result<TimeEntry> {
    Ok(TimeEntry {
        id: r.get(0)?,
        project_id: r.get(1)?,
        project_name: r.get(2)?,
        date: r.get(3)?,
        start_time: r.get(4)?,
        end_time: r.get(5)?,
        duration_minutes: r.get(6)?,
        note: r.get(7)?,
        origin: r.get(8)?,
        cost_total: r.get(9)?,
        cost_currency: r.get(10)?,
        created_at: r.get(11)?,
        updated_at: r.get(12)?,
    })
}
pub fn update_entry_repo(
    conn: &Connection,
    id: String,
    input: TimeEntryInput,
) -> Result<TimeEntry, String> {
    validate_date(&input.date)?;
    let duration = parse_duration(&input.date, &input.start_time, &input.end_time)?;
    let existing_project =
        existing_entry_project(conn, &id)?.ok_or_else(|| "Time entry not found".to_string())?;
    if input.project_id != existing_project && !project_exists_active(conn, &input.project_id)? {
        return Err(
            "Changing a time entry to an archived or missing project is not allowed".into(),
        );
    }
    let note = validate_entry_note(input.note)?;
    conn.execute("UPDATE time_entries SET project_id=?1,date=?2,start_time=?3,end_time=?4,duration_minutes=?5,note=?6,updated_at=?7 WHERE id=?8", params![input.project_id,input.date,input.start_time,input.end_time,duration,note,now(),id]).map_err(|e| e.to_string())?;
    get_entry(conn, &id)
}
pub fn delete_entry_repo(conn: &Connection, id: String) -> Result<(), String> {
    let affected = conn
        .execute("DELETE FROM time_entries WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err("Time entry not found".into());
    }
    Ok(())
}
pub fn list_entries_repo(
    conn: &Connection,
    start: String,
    end: String,
    project_id: Option<String>,
) -> Result<Vec<TimeEntry>, String> {
    validate_date_range(&start, &end)?;
    let mut sql = "SELECT e.id,e.project_id,p.name,e.date,e.start_time,e.end_time,e.duration_minutes,e.note,e.origin,e.cost_total,e.cost_currency,e.created_at,e.updated_at FROM time_entries e JOIN projects p ON p.id=e.project_id WHERE e.date>=?1 AND e.date<=?2".to_string();
    if project_id.is_some() {
        sql.push_str(" AND e.project_id=?3");
    }
    sql.push_str(" ORDER BY e.date DESC,e.start_time DESC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Result<Vec<_>, _> = if let Some(pid) = project_id {
        stmt.query_map(params![start, end, pid], row_to_entry)
            .map_err(|e| e.to_string())?
            .collect()
    } else {
        stmt.query_map(params![start, end], row_to_entry)
            .map_err(|e| e.to_string())?
            .collect()
    };
    rows.map_err(|e| e.to_string())
}
pub fn summary_repo(
    conn: &Connection,
    start: String,
    end: String,
    project_id: Option<String>,
) -> Result<Vec<SummaryRow>, String> {
    validate_date_range(&start, &end)?;
    // DEC-003: human ('manual') and AI ('ai_suggested') minutes are summed into separate columns so AI
    // time is never tallied into the billable human total. Anything not explicitly 'ai_suggested' counts
    // as human (conservative — a legacy/unknown origin can never silently land in the AI column).
    // AI cost is summed only over `ai_suggested` rows. Bare `SUM` (no COALESCE) returns NULL when no AI
    // row in range carries a cost so the column stays NULL → "—" (absence ≠ zero, DEC-004), never 0.
    // Currency is uniform-or-NULL in practice (design §4); `MAX` picks it without inventing one.
    let mut sql = "SELECT p.id,p.name,\
        COALESCE(SUM(CASE WHEN e.origin='ai_suggested' THEN 0 ELSE e.duration_minutes END),0),\
        COALESCE(SUM(CASE WHEN e.origin='ai_suggested' THEN e.duration_minutes ELSE 0 END),0),\
        SUM(CASE WHEN e.origin='ai_suggested' THEN e.cost_total END),\
        MAX(CASE WHEN e.origin='ai_suggested' THEN e.cost_currency END) \
        FROM time_entries e JOIN projects p ON p.id=e.project_id WHERE e.date>=?1 AND e.date<=?2"
        .to_string();
    if project_id.is_some() {
        sql.push_str(" AND e.project_id=?3");
    }
    sql.push_str(" GROUP BY p.id,p.name ORDER BY p.name");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mapper = |r: &rusqlite::Row| {
        Ok(SummaryRow {
            project_id: r.get(0)?,
            project_name: r.get(1)?,
            duration_minutes: r.get(2)?,
            ai_minutes: r.get(3)?,
            ai_cost_total: r.get(4)?,
            ai_cost_currency: r.get(5)?,
        })
    };
    let rows: Result<Vec<_>, _> = if let Some(pid) = project_id {
        stmt.query_map(params![start, end, pid], mapper)
            .map_err(|e| e.to_string())?
            .collect()
    } else {
        stmt.query_map(params![start, end], mapper)
            .map_err(|e| e.to_string())?
            .collect()
    };
    rows.map_err(|e| e.to_string())
}
fn csv_formula_neutralized(v: &str) -> String {
    let trimmed = v.trim_start_matches(|c: char| c.is_whitespace());
    // Spreadsheet apps can interpret cells as formulas after leading whitespace/control characters.
    if trimmed.starts_with(['=', '+', '-', '@'])
        || matches!(v.chars().next(), Some('\t' | '\r' | '\n'))
    {
        format!("'{v}")
    } else {
        v.to_string()
    }
}
fn csv_escape(v: &str) -> String {
    let safe = csv_formula_neutralized(v);
    if safe.contains([',', '\n', '\r', '"']) {
        format!("\"{}\"", safe.replace('"', "\"\""))
    } else {
        safe
    }
}
pub fn export_csv_repo(
    conn: &Connection,
    start: String,
    end: String,
    project_id: Option<String>,
    path: &Path,
) -> Result<usize, String> {
    let entries = list_entries_repo(conn, start, end, project_id)?;
    // DEC-003: each row carries its `origin` so a consumer can separate AI-suggested time from billable
    // human time; AI minutes are never folded into the human column.
    // DEC-003 completion: AI cost rides alongside `origin` so a consumer can attribute it per row.
    // Empty cells when NULL (absence ≠ zero); the numeric cost needs no escaping, currency reuses it.
    let mut out = String::from(
        "date,project,start_time,end_time,duration_minutes,note,origin,cost_total,cost_currency,total_duration_hours\n",
    );
    for e in &entries {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{:.2}\n",
            e.date,
            csv_escape(&e.project_name),
            e.start_time,
            e.end_time,
            e.duration_minutes,
            csv_escape(e.note.as_deref().unwrap_or("")),
            csv_escape(&e.origin),
            e.cost_total.map(|c| c.to_string()).unwrap_or_default(),
            csv_escape(e.cost_currency.as_deref().unwrap_or("")),
            e.duration_minutes as f64 / 60.0
        ));
    }
    fs::write(path, out).map_err(|e| e.to_string())?;
    Ok(entries.len())
}

// ----- TASK-032 B: accept / dismiss a suggestion (the only writer of an `ai_suggested` entry) ------

/// Extract a `HH:MM` clock time from a suggestion's local `block_start_ts`/`block_end_ts`
/// (`'YYYY-MM-DD HH:MM:SS'`). `None` when the field is absent or malformed, so an untimed block falls
/// through to the "edits required" path rather than fabricating a time.
fn hhmm_from_block_ts(ts: &str) -> Option<String> {
    let time = ts.split(' ').nth(1)?;
    time.get(..5).map(str::to_string)
}

/// Resolve the note for an accepted suggestion. An explicit edit note (validated, same caps as a manual
/// entry) wins; otherwise a secret-free provenance summary is recorded so the entry's AI origin is
/// self-describing. `source`/`reason` are already secret-free (SEC-012); the default is capped
/// defensively in case `reason` ever grows.
fn resolve_ai_note(
    suggestion: &suggestions::Suggestion,
    edit_note: Option<String>,
) -> Result<Option<String>, String> {
    if let Some(note) = clean_opt(edit_note) {
        validate_len(&note, MAX_ENTRY_NOTE_LEN, "Entry note")?;
        return Ok(Some(note));
    }
    let provenance = format!(
        "AI-suggested ({}): {}",
        suggestion.source, suggestion.reason
    );
    Ok(Some(provenance.chars().take(MAX_ENTRY_NOTE_LEN).collect()))
}

/// Accept a suggestion: create the **one** `origin='ai_suggested'` time entry it implies and mark the
/// suggestion `accepted` with its `accepted_entry_id`, atomically (single transaction). An untimed
/// ("needs manual time") suggestion REQUIRES `edits` supplying start/end — accept never invents a
/// duration (absence ≠ zero). Re-deciding an already accepted/dismissed suggestion is rejected. This is
/// the sole writer of an `ai_suggested` entry (DEC-006: generation never posts; only accept does).
pub fn accept_suggestion_repo(
    conn: &mut Connection,
    id: String,
    edits: Option<suggestions::SuggestionEdit>,
) -> Result<TimeEntry, String> {
    let edits = edits.unwrap_or_default();
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let suggestion = suggestions::store::get_by_id(&tx, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Suggestion not found".to_string())?;
    match suggestion.status.as_str() {
        "pending" => {}
        "accepted" => return Err("This suggestion has already been accepted".into()),
        "dismissed" => {
            return Err("This suggestion has been dismissed and cannot be accepted".into())
        }
        _ => return Err("This suggestion cannot be accepted".into()),
    }

    let project_id = edits
        .project_id
        .clone()
        .unwrap_or_else(|| suggestion.project_id.clone());
    let date = edits
        .date
        .clone()
        .unwrap_or_else(|| suggestion.date.clone());
    let start_time = edits.start_time.clone().or_else(|| {
        suggestion
            .block_start_ts
            .as_deref()
            .and_then(hhmm_from_block_ts)
    });
    let end_time = edits.end_time.clone().or_else(|| {
        suggestion
            .block_end_ts
            .as_deref()
            .and_then(hhmm_from_block_ts)
    });
    let (Some(start_time), Some(end_time)) = (start_time, end_time) else {
        return Err(
            "This suggestion has no usable time — add a start and end time before accepting".into(),
        );
    };

    validate_date(&date)?;
    // DEC-034/035: a timed suggestion whose start and end fall in the same clock minute must still be
    // acceptable with a positive same-day span. Normalize to the engine's valid duration — anchored on
    // the start (end = start + minutes) everywhere except the day's last minute, where it anchors on the
    // end (start = 23:59 - minutes). Rebind both ends. The manual `create_time_entry`/`update` path keeps
    // rejecting `start == end` (a human typo).
    let (start_time, end_time) =
        normalize_same_minute_span(&date, &start_time, &end_time, suggestion.duration_minutes)?;
    let duration = parse_duration(&date, &start_time, &end_time)?;
    if !project_exists_active(&tx, &project_id)? {
        return Err("Accepted suggestions must reference an active project".into());
    }
    let note = resolve_ai_note(&suggestion, edits.note)?;

    let entry_id = Uuid::new_v4().to_string();
    let ts = now();
    // DEC-003 completion: copy the suggestion's cost provenance verbatim onto the entry so it reaches
    // Reports/CSV. Cost is not user-editable (no field on `SuggestionEdit`); NULL stays NULL (→ "—").
    tx.execute(
        "INSERT INTO time_entries(id,project_id,date,start_time,end_time,duration_minutes,note,origin,cost_total,cost_currency,created_at,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,'ai_suggested',?8,?9,?10,?10)",
        params![entry_id, project_id, date, start_time, end_time, duration, note, suggestion.cost_total, suggestion.cost_currency, ts],
    )
    .map_err(|e| e.to_string())?;
    // Guarded UPDATE: the `status='pending'` predicate closes the door on a concurrent decision between
    // the read above and here. If it matched no row, the suggestion was decided meanwhile — roll back.
    let marked = tx
        .execute(
            "UPDATE time_entry_suggestions SET status='accepted', accepted_entry_id=?1, updated_at=?2
             WHERE id=?3 AND status='pending'",
            params![entry_id, ts, id],
        )
        .map_err(|e| e.to_string())?;
    if marked != 1 {
        return Err("This suggestion was already decided".into());
    }
    let entry = get_entry(&tx, &entry_id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(entry)
}

/// Dismiss a suggestion: mark it `dismissed` and write **no** time entry. Idempotent — dismissing an
/// already-dismissed suggestion is a no-op success. An already-accepted suggestion cannot be dismissed
/// (its time entry exists), so that is rejected.
pub fn dismiss_suggestion_repo(conn: &Connection, id: String) -> Result<(), String> {
    let suggestion = suggestions::store::get_by_id(conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Suggestion not found".to_string())?;
    match suggestion.status.as_str() {
        "pending" => {
            conn.execute(
                "UPDATE time_entry_suggestions SET status='dismissed', updated_at=?1
                 WHERE id=?2 AND status='pending'",
                params![now(), id],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        }
        "dismissed" => Ok(()),
        "accepted" => Err("An accepted suggestion cannot be dismissed".into()),
        _ => Err("This suggestion cannot be dismissed".into()),
    }
}

#[tauri::command]
fn get_capture_status() -> CaptureStatus {
    CaptureStatus { mode: "Manual Mode / Capture deferred".into(), message: "Automatic activity capture is not active in v0.1. Vire stores only projects and manual time entries locally.".into() }
}
#[tauri::command]
fn list_projects(state: State<AppState>, include_archived: bool) -> CmdResult<Vec<Project>> {
    let db = db_conn(&state)?;
    list_projects_repo(&db, include_archived)
}
#[tauri::command]
fn create_project(state: State<AppState>, input: ProjectInput) -> CmdResult<Project> {
    let db = db_conn(&state)?;
    create_project_repo(&db, input)
}
#[tauri::command]
fn update_project(state: State<AppState>, id: String, input: ProjectInput) -> CmdResult<Project> {
    let db = db_conn(&state)?;
    update_project_repo(&db, id, input)
}
#[tauri::command]
fn archive_project(state: State<AppState>, id: String) -> CmdResult<Project> {
    let db = db_conn(&state)?;
    archive_project_repo(&db, id)
}
#[tauri::command]
fn list_time_entries(
    state: State<AppState>,
    start_date: String,
    end_date: String,
    project_id: Option<String>,
) -> CmdResult<Vec<TimeEntry>> {
    let db = db_conn(&state)?;
    list_entries_repo(&db, start_date, end_date, project_id)
}
#[tauri::command]
fn create_time_entry(state: State<AppState>, input: TimeEntryInput) -> CmdResult<TimeEntry> {
    let db = db_conn(&state)?;
    create_entry_repo(&db, input)
}
#[tauri::command]
fn update_time_entry(
    state: State<AppState>,
    id: String,
    input: TimeEntryInput,
) -> CmdResult<TimeEntry> {
    let db = db_conn(&state)?;
    update_entry_repo(&db, id, input)
}
#[tauri::command]
fn delete_time_entry(state: State<AppState>, id: String) -> CmdResult<()> {
    let db = db_conn(&state)?;
    delete_entry_repo(&db, id)
}
#[tauri::command]
fn get_summary(
    state: State<AppState>,
    start_date: String,
    end_date: String,
    project_id: Option<String>,
) -> CmdResult<Vec<SummaryRow>> {
    let db = db_conn(&state)?;
    summary_repo(&db, start_date, end_date, project_id)
}
#[tauri::command]
async fn export_report_csv(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    start_date: String,
    end_date: String,
    project_id: Option<String>,
) -> CmdResult<Option<usize>> {
    validate_date_range(&start_date, &end_date)?;
    // Present the save dialog off the UI main thread. `blocking_save_file()` parks the calling thread
    // on a channel that its completion callback feeds via the main run loop; called on the main thread
    // (a sync command) that callback can never be delivered and the IPC promise hangs (the macOS
    // deadlock this fix targets). Running it on the blocking pool keeps the main run loop free, so the
    // dialog resolves and the command always returns.
    let dialog_app = app.clone();
    let destination = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .add_filter("CSV", &["csv"])
            .set_file_name("vire-report.csv")
            .blocking_save_file()
    })
    .await
    .map_err(|_| "CSV export save dialog could not be presented".to_string())?;
    let Some(destination) = destination else {
        return Ok(None);
    };
    let path = destination
        .into_path()
        .map_err(|_| "CSV export destination must be a local file path".to_string())?;
    validate_csv_destination(&path)?;
    // Lock the DB only after the dialog resolves — never hold the MutexGuard across the `.await` above.
    let db = db_conn(&state)?;
    export_csv_repo(&db, start_date, end_date, project_id, &path).map(Some)
}

/// List AI time-entry suggestions (TASK-032 B). `regenerate=true` recomputes the pending set from the
/// latest evidence (preserving decided rows); `false` returns the stored pending set. Both also return
/// the unmapped-environment summary so the UI can prompt mapping. Reads SQLite only — no egress.
#[tauri::command]
fn list_time_entry_suggestions(
    state: State<AppState>,
    regenerate: bool,
) -> CmdResult<suggestions::SuggestionList> {
    let db = db_conn(&state)?;
    if regenerate {
        suggestions::generate(&db).map_err(|e| e.to_string())
    } else {
        suggestions::current(&db).map_err(|e| e.to_string())
    }
}

/// Accept a suggestion, creating the single `ai_suggested` time entry it implies (TASK-032 B3).
#[tauri::command]
fn accept_time_entry_suggestion(
    state: State<AppState>,
    id: String,
    edits: Option<suggestions::SuggestionEdit>,
) -> CmdResult<TimeEntry> {
    let mut db = db_conn(&state)?;
    accept_suggestion_repo(&mut db, id, edits)
}

/// Dismiss a suggestion; writes no time entry (TASK-032 B3).
#[tauri::command]
fn dismiss_time_entry_suggestion(state: State<AppState>, id: String) -> CmdResult<()> {
    let db = db_conn(&state)?;
    dismiss_suggestion_repo(&db, id)
}

#[tauri::command]
fn get_langfuse_source_health(state: State<AppState>) -> CmdResult<SourceHealthSnapshot> {
    let db = db_conn(&state)?;
    // Settings-first; returns an explicit disabled snapshot (no probe, no Keychain) when off.
    settings::source_health_snapshot(&db)
}

#[tauri::command]
fn get_langfuse_settings(state: State<AppState>) -> CmdResult<settings::LangfuseSettings> {
    let db = db_conn(&state)?;
    settings::get_langfuse_settings_repo(&db, &settings::secret_store::KeyringSecretStore::new())
}

#[tauri::command]
fn set_langfuse_settings(
    state: State<AppState>,
    input: settings::LangfuseSettingsInput,
) -> CmdResult<settings::LangfuseSettings> {
    let db = db_conn(&state)?;
    settings::set_langfuse_settings_repo(
        &db,
        &settings::secret_store::KeyringSecretStore::new(),
        input,
    )
}

/// Read the configured Langfuse import range as its canonical string (TASK-029 C). Secret-free: the
/// value is only ever a fixed range keyword or a timestamp.
#[tauri::command]
fn get_langfuse_import_range(state: State<AppState>) -> CmdResult<String> {
    let db = db_conn(&state)?;
    settings::get_langfuse_import_range_repo(&db)
}

/// Validate and persist the Langfuse import range (TASK-029 C1). A malformed value is rejected with a
/// fixed, secret-free error; the canonical normalized value is returned for the form to re-render.
#[tauri::command]
fn set_langfuse_import_range(state: State<AppState>, range: String) -> CmdResult<String> {
    let db = db_conn(&state)?;
    settings::set_langfuse_import_range_repo(&db, range)
}

#[tauri::command]
fn set_langfuse_secret(
    state: State<AppState>,
    public_key: String,
    secret_key: String,
) -> CmdResult<()> {
    // The public key now persists in the SQLite `settings` table (TASK-044), so this command needs
    // the DB handle. The JS argument shape (`{publicKey, secretKey}`) is unchanged — `state` is
    // injected by Tauri, not passed by the renderer.
    let db = db_conn(&state)?;
    settings::set_langfuse_secret_repo(
        &db,
        &settings::secret_store::KeyringSecretStore::new(),
        public_key,
        secret_key,
    )
}

#[tauri::command]
fn clear_langfuse_secret(state: State<AppState>) -> CmdResult<()> {
    let db = db_conn(&state)?;
    settings::clear_langfuse_secret_repo(&db, &settings::secret_store::KeyringSecretStore::new())
}

#[tauri::command]
fn test_langfuse_connection(state: State<AppState>) -> CmdResult<langfuse::TestConnectionResult> {
    // A disabled integration short-circuits to an explicit disabled verdict BEFORE any Keychain
    // read or network probe (TASK-026): `test_connection_plan` only touches the secret store on the
    // enabled `Probe` path. Resolve under the lock, then drop it before the bounded probe so the
    // UI's DB is never held across the network call.
    let config = {
        let db = db_conn(&state)?;
        match settings::test_connection_plan(
            &db,
            &settings::secret_store::KeyringSecretStore::new(),
        )? {
            settings::TestConnectionPlan::Disabled => {
                return Ok(langfuse::TestConnectionResult::disabled());
            }
            settings::TestConnectionPlan::Probe(config) => config,
        }
    };
    run_bounded_result(
        Duration::from_secs(TEST_CONNECTION_TIMEOUT_SECS),
        TEST_CONNECTION_TIMEOUT_MSG,
        move || Ok(langfuse::test_connection(config)),
    )
}

/// Read-only runtime-reconciliation surface (TASK-022). Ingests the local coarse session log,
/// reconciles it against the importer's evidence read-only, persists the observer-owned
/// `ai_runtime_sessions` rows, and returns coarse per-state counts only — no secrets, session
/// content, command bodies, or raw log lines. The observer makes no network call.
#[tauri::command]
fn get_runtime_reconciliation(state: State<AppState>) -> CmdResult<RuntimeReconciliationSnapshot> {
    let db = db_conn(&state)?;
    runtime_observer::observe_and_reconcile(&db)
}

// ----- TASK-027 D: environment → project mapping (secret-free surfaces) ----------------------

/// Discovered environments with their mapping state, so the UI can render mapped / unmapped /
/// suggest-create per environment. Secret-free: env names + project refs + state only.
#[tauri::command]
fn list_discovered_environments(
    state: State<AppState>,
) -> CmdResult<Vec<env_mapping::DiscoveredEnvState>> {
    let db = db_conn(&state)?;
    env_mapping::list_discovered_environments_repo(&db)
}

#[tauri::command]
fn list_env_mappings(state: State<AppState>) -> CmdResult<Vec<env_mapping::EnvMapping>> {
    let db = db_conn(&state)?;
    env_mapping::list_env_mappings_repo(&db)
}

/// Map an environment to an **existing** project. Never creates a project (DEC-006): the renderer
/// calls `create_project` first when the user accepts the suggestion, then maps the new project here.
#[tauri::command]
fn set_env_mapping(
    state: State<AppState>,
    environment: String,
    project_id: String,
) -> CmdResult<env_mapping::EnvMapping> {
    let db = db_conn(&state)?;
    env_mapping::set_env_mapping_repo(&db, environment, project_id)
}

#[tauri::command]
fn clear_env_mapping(state: State<AppState>, environment: String) -> CmdResult<()> {
    let db = db_conn(&state)?;
    env_mapping::clear_env_mapping_repo(&db, environment)
}

/// Imported AI evidence associated with its mapped project at read time (D3). The join is performed
/// on read — no evidence row is rewritten. Secret-free: env names, trace ids, project refs only.
#[tauri::command]
fn list_evidence_projects(state: State<AppState>) -> CmdResult<Vec<env_mapping::EvidenceProject>> {
    let db = db_conn(&state)?;
    env_mapping::list_evidence_projects_repo(&db)
}

/// Ceiling for a manual import, comfortably above the reqwest 15s request / 5s connect ceilings so a
/// normal slow import is not cut off, but an indefinite hang (dependency deadlock, lock contention)
/// cannot block the UI forever (S-6).
const IMPORT_TIMEOUT_SECS: u64 = 30;
const IMPORT_TIMEOUT_MSG: &str =
    "Langfuse import did not complete within the time limit — AI usage and cost are unknown, not zero";

/// Ceiling for a **backfill** (TASK-029 C5). Backfill is inherently slower (it re-scans the full
/// configured range in chunks), so it gets a larger bound than an incremental import. Because backfill
/// is chunked-durable-resumable, a timeout is non-destructive: committed chunks persist and the user
/// re-runs "Backfill now" to continue.
const BACKFILL_TIMEOUT_SECS: u64 = 300;
const BACKFILL_TIMEOUT_MSG: &str =
    "Langfuse backfill did not complete within the time limit — already-imported history is saved; \
     re-run Backfill now to continue";

/// Ceiling for the in-app Test connection probe (TASK-026), above the reqwest 15s request / 5s
/// connect ceilings so a normal slow probe is not cut off, but a hang cannot freeze the UI.
const TEST_CONNECTION_TIMEOUT_SECS: u64 = 20;
const TEST_CONNECTION_TIMEOUT_MSG: &str =
    "Langfuse test connection did not complete within the time limit";

// ----- TASK-027 B: automatic import (startup + periodic), serialized with the manual path -------

/// Default interval between background imports. Configurable via `VIRE_LANGFUSE_AUTO_IMPORT_INTERVAL_SECS`
/// (a marked ops/dev override, floored at 30s so it can never hammer the source).
const AUTO_IMPORT_INTERVAL_SECS: u64 = 900;
const AUTO_IMPORT_MIN_INTERVAL_SECS: u64 = 30;

fn auto_import_interval() -> Duration {
    let secs = std::env::var("VIRE_LANGFUSE_AUTO_IMPORT_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(|s| s.max(AUTO_IMPORT_MIN_INTERVAL_SECS))
        .unwrap_or(AUTO_IMPORT_INTERVAL_SECS);
    Duration::from_secs(secs)
}

/// Acquire the single import slot **without blocking**. `None` means another import (manual or a
/// prior automatic tick) holds it, so the caller skips this round — the serialization guard (B3)
/// that keeps any two imports from running concurrently against the local store. A poisoned lock is
/// recovered: the guarded `()` carries no invariant, so a panic in a prior import must not wedge all
/// future imports.
fn try_acquire_import_slot(lock: &Mutex<()>) -> Option<MutexGuard<'_, ()>> {
    match lock.try_lock() {
        Ok(guard) => Some(guard),
        Err(TryLockError::WouldBlock) => None,
        Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
    }
}

/// Acquire the import slot, **waiting** if another import holds it. Used by the manual command's
/// worker so a user click is honored after an in-flight automatic import finishes — never run
/// concurrently with it. The wait is bounded by the manual command's own `run_bounded_result`
/// ceiling, so a stuck prior import surfaces as a secret-free timeout, never a UI freeze.
fn acquire_import_slot(lock: &Mutex<()>) -> MutexGuard<'_, ()> {
    lock.lock().unwrap_or_else(|e| e.into_inner())
}

/// One automatic-import cycle (startup or periodic), run on a dedicated OS thread off the UI/Tauri
/// runtime. Serialized with the manual path and prior ticks via the shared slot (skips when busy),
/// and honors the disabled switch + loopback boundary identically to `import_langfuse_now`: when the
/// integration is disabled it runs nothing and probes nothing (no Keychain read, no socket). A
/// reachable-but-failing import resolves into the health taxonomy via `run_blocking_import`, never to
/// zero. No new capability/CSP — this is pure Rust-core scheduling; the renderer makes no call.
fn run_auto_import_cycle(db_path: &Path, lock: &Mutex<()>) {
    let Some(_slot) = try_acquire_import_slot(lock) else {
        return; // another import is in progress — serialize by skipping this tick.
    };
    // Disabled short-circuit BEFORE any probe/Keychain, mirroring the manual command. Open a
    // throwaway connection only to read the (non-secret) enable switch; a failure to open just skips.
    match Connection::open(db_path) {
        Ok(conn) => {
            if !settings::langfuse_enabled(&conn) {
                return;
            }
        }
        Err(_) => return,
    }
    // Enabled: run the same blocking import the manual button uses. The slot is held for the whole
    // run, so a concurrent manual click or next tick waits/skips. Errors are intentionally dropped
    // here — they are already recorded in the health taxonomy + import report inside the importer.
    let _ = langfuse::run_blocking_import(db_path);
}

/// Run blocking `work` on a dedicated OS thread (off the Tauri runtime) and wait at most `timeout`,
/// returning the work's result or a secret-free `timeout_msg` if it does not finish in time. An
/// orphaned worker that finishes late just fails to send (the receiver is gone) and is bounded by
/// the reqwest ceilings; it persists atomically (S-3) if it completes. No new health state is added.
fn run_bounded_result<T, F>(timeout: Duration, timeout_msg: &str, work: F) -> CmdResult<T>
where
    F: FnOnce() -> CmdResult<T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => Err(timeout_msg.to_string()),
    }
}

/// The manual-import IPC result (TASK-027 A3): the read-only health snapshot **plus** the
/// secret-free [`langfuse::ImportReport`] so the Settings panel can render "Imported N (M skipped,
/// D duplicates) across E environments" instead of nothing. `report` is `None` only on the
/// disabled short-circuit — no import ran — so the UI shows the explicit disabled snapshot rather
/// than fabricated zero counts.
#[derive(Serialize)]
struct ImportOutcome {
    snapshot: SourceHealthSnapshot,
    report: Option<langfuse::ImportReport>,
}

#[tauri::command]
fn import_langfuse_now(state: State<AppState>) -> CmdResult<ImportOutcome> {
    // Short-circuit a disabled integration before any network or Keychain access (TASK-026): no
    // import runs and the snapshot reports an explicit disabled state, never zero. No report is
    // produced because no run occurred.
    {
        let db = db_conn(&state)?;
        if !settings::langfuse_enabled(&db) {
            let snapshot = settings::source_health_snapshot(&db)?;
            return Ok(ImportOutcome {
                snapshot,
                report: None,
            });
        }
    }
    // Run the blocking REST import on a dedicated OS thread (off the Tauri runtime and off the UI's
    // database lock); it uses its own SQLite connection. Bound the wait so a hung import returns a
    // secret-free error instead of blocking the UI. The worker returns the secret-free import
    // report; then read the resulting snapshot.
    let db_path = state.db_path.clone();
    let import_lock = state.import_lock.clone();
    let report = run_bounded_result(
        Duration::from_secs(IMPORT_TIMEOUT_SECS),
        IMPORT_TIMEOUT_MSG,
        move || {
            // Serialize with any in-flight automatic import (B3): wait for the shared slot, then run
            // the same path. The wait is bounded by the ceiling above, so the UI never freezes.
            let _slot = acquire_import_slot(&import_lock);
            langfuse::run_blocking_import(&db_path)
        },
    )?;
    let db = db_conn(&state)?;
    let snapshot = settings::source_health_snapshot(&db)?;
    Ok(ImportOutcome {
        snapshot,
        report: Some(report),
    })
}

/// Explicit "Backfill now" (TASK-029 C5): re-scan the configured range floor → now in bounded,
/// atomically-committed, resumable chunks. Identical posture to [`import_langfuse_now`] — disabled
/// short-circuit before any probe/Keychain, off-UI dedicated thread, `import_lock` serialization,
/// SEC-002 loopback boundary, GET-only, secret-free report — but with a larger time bound. Because
/// backfill is chunked-durable-resumable, a timeout is non-destructive (committed chunks persist).
#[tauri::command]
fn backfill_langfuse_now(state: State<AppState>) -> CmdResult<ImportOutcome> {
    {
        let db = db_conn(&state)?;
        if !settings::langfuse_enabled(&db) {
            let snapshot = settings::source_health_snapshot(&db)?;
            return Ok(ImportOutcome {
                snapshot,
                report: None,
            });
        }
    }
    let db_path = state.db_path.clone();
    let import_lock = state.import_lock.clone();
    let report = run_bounded_result(
        Duration::from_secs(BACKFILL_TIMEOUT_SECS),
        BACKFILL_TIMEOUT_MSG,
        move || {
            // Serialize with any in-flight import (B3): one importer touches the local store at a time.
            let _slot = acquire_import_slot(&import_lock);
            langfuse::run_blocking_backfill(&db_path)
        },
    )?;
    let db = db_conn(&state)?;
    let snapshot = settings::source_health_snapshot(&db)?;
    Ok(ImportOutcome {
        snapshot,
        report: Some(report),
    })
}

fn validate_csv_destination(path: &Path) -> Result<(), String> {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("csv"))
        != Some(true)
    {
        return Err("CSV export destination must use a .csv extension".into());
    }
    if path.is_dir() {
        return Err("CSV export destination must be a file, not a directory".into());
    }
    Ok(())
}
fn db_path(app: &tauri::App) -> Result<PathBuf, std::io::Error> {
    let dir = app.path().app_data_dir().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Could not resolve Vire app data directory: {e}"),
        )
    })?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join("vire.sqlite"))
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = db_path(app)?;
            let conn = Connection::open(&path)?;
            init_db(&conn)?;
            let import_lock = Arc::new(Mutex::new(()));
            app.manage(AppState {
                db: Mutex::new(conn),
                db_path: path.clone(),
                import_lock: import_lock.clone(),
            });
            // TASK-048: zero-permission active-app + idle capture on its own OS thread (mirrors the
            // Langfuse scheduler below). Gated by `active_window_capture_enabled` (default OFF) read
            // inside the loop; while disabled it samples nothing, touches no native API, and writes no
            // rows. No renderer/IPC/network/CSP surface — writes go only through the TASK-046 store API.
            active_window::capture::spawn(path.clone());
            // TASK-027 B: automatic import on a dedicated OS thread (off the UI/Tauri runtime). One import
            // at startup, then a periodic tick. Each cycle serializes with the manual path via the shared
            // slot and honors the disabled switch + loopback boundary inside `run_auto_import_cycle`. The
            // thread sleeps between ticks and is reclaimed on process exit.
            std::thread::spawn(move || {
                let interval = auto_import_interval();
                loop {
                    run_auto_import_cycle(&path, &import_lock);
                    std::thread::sleep(interval);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_capture_status,
            list_projects,
            create_project,
            update_project,
            archive_project,
            list_time_entries,
            create_time_entry,
            update_time_entry,
            delete_time_entry,
            get_summary,
            export_report_csv,
            get_langfuse_source_health,
            import_langfuse_now,
            backfill_langfuse_now,
            get_runtime_reconciliation,
            get_langfuse_settings,
            set_langfuse_settings,
            get_langfuse_import_range,
            set_langfuse_import_range,
            set_langfuse_secret,
            clear_langfuse_secret,
            test_langfuse_connection,
            list_discovered_environments,
            list_env_mappings,
            set_env_mapping,
            clear_env_mapping,
            list_evidence_projects,
            list_time_entry_suggestions,
            accept_time_entry_suggestion,
            dismiss_time_entry_suggestion
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vire");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        init_db(&c).unwrap();
        c
    }
    #[test]
    fn project_crud_archive_and_active_filter() {
        let c = conn();
        let p = create_project_repo(
            &c,
            ProjectInput {
                name: " Alpha ".into(),
                notes: Some("n".into()),
            },
        )
        .unwrap();
        let p = update_project_repo(
            &c,
            p.id.clone(),
            ProjectInput {
                name: "Beta".into(),
                notes: None,
            },
        )
        .unwrap();
        assert_eq!(p.name, "Beta");
        archive_project_repo(&c, p.id).unwrap();
        assert_eq!(list_projects_repo(&c, false).unwrap().len(), 0);
        assert_eq!(list_projects_repo(&c, true).unwrap().len(), 1);
    }
    #[test]
    fn manual_entry_crud_delete_and_validation() {
        let c = conn();
        let p = create_project_repo(
            &c,
            ProjectInput {
                name: "P".into(),
                notes: None,
            },
        )
        .unwrap();
        assert!(create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: p.id.clone(),
                date: "2026-01-01".into(),
                start_time: "10:00".into(),
                end_time: "09:00".into(),
                note: None
            }
        )
        .is_err());
        let e = create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: p.id.clone(),
                date: "2026-01-01".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: None,
            },
        )
        .unwrap();
        let e = update_entry_repo(
            &c,
            e.id.clone(),
            TimeEntryInput {
                project_id: p.id,
                date: "2026-01-01".into(),
                start_time: "09:00".into(),
                end_time: "10:30".into(),
                note: Some("done".into()),
            },
        )
        .unwrap();
        assert_eq!(e.duration_minutes, 90);
        delete_entry_repo(&c, e.id).unwrap();
        assert!(
            list_entries_repo(&c, "2026-01-01".into(), "2026-01-01".into(), None)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn summaries_and_csv_filtering_escape() {
        let c = conn();
        let p1 = create_project_repo(
            &c,
            ProjectInput {
                name: "A, Inc".into(),
                notes: None,
            },
        )
        .unwrap();
        let p2 = create_project_repo(
            &c,
            ProjectInput {
                name: "B".into(),
                notes: None,
            },
        )
        .unwrap();
        create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: p1.id.clone(),
                date: "2026-02-01".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: Some("said \"hi\"".into()),
            },
        )
        .unwrap();
        create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: p2.id,
                date: "2026-02-01".into(),
                start_time: "10:00".into(),
                end_time: "12:00".into(),
                note: None,
            },
        )
        .unwrap();
        let s = summary_repo(
            &c,
            "2026-02-01".into(),
            "2026-02-01".into(),
            Some(p1.id.clone()),
        )
        .unwrap();
        assert_eq!(s[0].duration_minutes, 60);
        let f = NamedTempFile::new().unwrap();
        let n = export_csv_repo(
            &c,
            "2026-02-01".into(),
            "2026-02-01".into(),
            Some(p1.id),
            f.path(),
        )
        .unwrap();
        let csv = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(n, 1);
        assert!(csv.contains("\"A, Inc\""));
        assert!(csv.contains("\"said \"\"hi\"\"\""));
    }
    #[test]
    fn persistence_across_reopen() {
        let f = NamedTempFile::new().unwrap();
        {
            let c = Connection::open(f.path()).unwrap();
            init_db(&c).unwrap();
            create_project_repo(
                &c,
                ProjectInput {
                    name: "Persist".into(),
                    notes: None,
                },
            )
            .unwrap();
        }
        let c = Connection::open(f.path()).unwrap();
        init_db(&c).unwrap();
        assert_eq!(list_projects_repo(&c, false).unwrap()[0].name, "Persist");
    }
    #[test]
    fn rejects_overlong_text_fields() {
        let c = conn();
        assert!(create_project_repo(
            &c,
            ProjectInput {
                name: "x".repeat(MAX_PROJECT_NAME_LEN + 1),
                notes: None
            }
        )
        .unwrap_err()
        .contains("Project name"));
        assert!(create_project_repo(
            &c,
            ProjectInput {
                name: "P".into(),
                notes: Some("x".repeat(MAX_PROJECT_NOTES_LEN + 1))
            }
        )
        .unwrap_err()
        .contains("Project notes"));
        let p = create_project_repo(
            &c,
            ProjectInput {
                name: "P".into(),
                notes: None,
            },
        )
        .unwrap();
        assert!(create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: p.id,
                date: "2026-01-01".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: Some("x".repeat(MAX_ENTRY_NOTE_LEN + 1))
            }
        )
        .unwrap_err()
        .contains("Entry note"));
    }
    #[test]
    fn new_entries_reject_archived_projects_but_existing_can_keep_same_project() {
        let c = conn();
        let p = create_project_repo(
            &c,
            ProjectInput {
                name: "Archived".into(),
                notes: None,
            },
        )
        .unwrap();
        let e = create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: p.id.clone(),
                date: "2026-01-01".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: None,
            },
        )
        .unwrap();
        archive_project_repo(&c, p.id.clone()).unwrap();
        assert!(create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: p.id.clone(),
                date: "2026-01-02".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: None
            }
        )
        .is_err());
        assert!(update_entry_repo(
            &c,
            e.id,
            TimeEntryInput {
                project_id: p.id,
                date: "2026-01-01".into(),
                start_time: "09:00".into(),
                end_time: "10:30".into(),
                note: None
            }
        )
        .is_ok());
    }
    #[test]
    fn delete_missing_entry_returns_error() {
        let c = conn();
        assert!(delete_entry_repo(&c, "missing".into())
            .unwrap_err()
            .contains("not found"));
    }
    #[test]
    fn csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes() {
        assert_eq!(csv_escape("=SUM(1,2)"), "\"'=SUM(1,2)\"");
        assert_eq!(csv_escape("  @cmd"), "'  @cmd");
        assert_eq!(csv_escape("\nplain"), "\"'\nplain\"");
        assert_eq!(csv_escape("line\rbreak"), "\"line\rbreak\"");
        assert_eq!(csv_escape(" +SUM(1,2)"), "\"' +SUM(1,2)\"");
    }
    #[test]
    fn clean_opt_preserves_nonempty_value_and_collapses_whitespace_only() {
        assert_eq!(clean_opt(Some(" +SUM".into())), Some(" +SUM".to_string()));
        assert_eq!(
            clean_opt(Some("  done  ".into())),
            Some("  done  ".to_string())
        );
        assert_eq!(clean_opt(Some("   ".into())), None);
        assert_eq!(clean_opt(Some("".into())), None);
        assert_eq!(clean_opt(None), None);
    }

    // ----- TASK-024: the export command resolves deterministically (non-dialog logic) ---------
    // The GUI deadlock fix (save dialog off the main thread) is validated by manual macOS smoke; the
    // destination validation and write-failure paths the command relies on to *always resolve* are
    // unit-testable and asserted here.

    #[test]
    fn validate_csv_destination_requires_csv_file_and_rejects_directories() {
        // Wrong / missing extension → Err (surfaced to the user, never a hang).
        assert!(validate_csv_destination(Path::new("/tmp/report.txt"))
            .unwrap_err()
            .contains(".csv"));
        assert!(validate_csv_destination(Path::new("/tmp/report")).is_err());
        // A directory whose name ends in .csv is rejected as a directory, not treated as a file.
        let dir = tempfile::tempdir().unwrap();
        let csv_named_dir = dir.path().join("bundle.csv");
        std::fs::create_dir(&csv_named_dir).unwrap();
        assert!(validate_csv_destination(&csv_named_dir)
            .unwrap_err()
            .contains("directory"));
        // A plain .csv file path (extension ok, not a directory) is accepted.
        assert!(validate_csv_destination(&dir.path().join("out.csv")).is_ok());
    }

    #[test]
    fn export_csv_repo_surfaces_write_failure_as_error_not_a_hang() {
        let c = conn();
        let p = create_project_repo(
            &c,
            ProjectInput {
                name: "P".into(),
                notes: None,
            },
        )
        .unwrap();
        create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: p.id.clone(),
                date: "2026-03-01".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: None,
            },
        )
        .unwrap();
        // Destination under a directory that does not exist → fs::write fails; the error is returned
        // (Err), proving a write failure resolves deterministically rather than hanging or panicking.
        let bad = Path::new("/vire-nonexistent-dir-xyz/never/out.csv");
        assert!(!export_csv_repo(
            &c,
            "2026-03-01".into(),
            "2026-03-01".into(),
            Some(p.id),
            bad
        )
        .unwrap_err()
        .is_empty());
    }

    // ----- S-6: the manual import command is bounded ---------------------------------------

    #[test]
    fn run_bounded_times_out_promptly_with_a_secret_free_error() {
        let start = std::time::Instant::now();
        let err = run_bounded_result(Duration::from_millis(50), IMPORT_TIMEOUT_MSG, || {
            std::thread::sleep(Duration::from_millis(1500));
            Ok(())
        })
        .unwrap_err();
        // The command returned well within the ceiling rather than blocking on the 1.5s worker.
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "must not block past the ceiling"
        );
        assert!(!err.is_empty());
        for needle in ["sk-", "pk-", "password", "token", "Bearer", "Authorization"] {
            assert!(
                !err.contains(needle),
                "timeout error must be secret-free, found {needle}"
            );
        }
    }

    #[test]
    fn run_bounded_returns_the_works_result_within_the_ceiling() {
        assert!(run_bounded_result(Duration::from_secs(5), IMPORT_TIMEOUT_MSG, || Ok(())).is_ok());
        let err = run_bounded_result::<(), _>(Duration::from_secs(5), IMPORT_TIMEOUT_MSG, || {
            Err("import failed".to_string())
        })
        .unwrap_err();
        assert_eq!(
            err, "import failed",
            "a normal failure is surfaced verbatim, not masked as a timeout"
        );
    }

    // ----- TASK-026: the value-returning bounded wrapper backs test_langfuse_connection -------

    #[test]
    fn run_bounded_result_returns_a_value_within_the_ceiling() {
        let v = run_bounded_result(Duration::from_secs(5), "unused", || Ok(42u32)).unwrap();
        assert_eq!(v, 42);
    }

    #[test]
    fn run_bounded_result_times_out_with_the_supplied_secret_free_message() {
        let start = std::time::Instant::now();
        let err = run_bounded_result::<u32, _>(
            Duration::from_millis(50),
            TEST_CONNECTION_TIMEOUT_MSG,
            || {
                std::thread::sleep(Duration::from_millis(1500));
                Ok(7)
            },
        )
        .unwrap_err();
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "must not block past the ceiling"
        );
        assert_eq!(err, TEST_CONNECTION_TIMEOUT_MSG);
        for needle in ["sk-", "pk-", "Bearer", "Authorization", "password"] {
            assert!(!err.contains(needle), "timeout message must be secret-free");
        }
    }

    // ----- TASK-027 B: automatic import — serialization guard + disabled short-circuit ---------

    #[test]
    fn import_slot_serializes_concurrent_imports() {
        let lock = Mutex::new(());
        let first = try_acquire_import_slot(&lock).expect("the first import acquires the slot");
        assert!(
            try_acquire_import_slot(&lock).is_none(),
            "a second import is refused (serialized) while one is in progress"
        );
        drop(first);
        assert!(
            try_acquire_import_slot(&lock).is_some(),
            "the slot frees once the prior import finishes"
        );
    }

    #[test]
    fn auto_import_cycle_runs_nothing_when_disabled() {
        // A disabled integration must run no import and fire no probe (no network, no Keychain): the
        // cycle short-circuits before `run_blocking_import`, so nothing is recorded. Using a temp DB
        // keeps this deterministic and offline.
        let f = NamedTempFile::new().unwrap();
        {
            let c = Connection::open(f.path()).unwrap();
            init_db(&c).unwrap();
            c.execute(
                "INSERT INTO settings(key, value) VALUES ('langfuse_enabled', 'false')
                 ON CONFLICT(key) DO UPDATE SET value = 'false'",
                [],
            )
            .unwrap();
        }
        let lock = Mutex::new(());
        run_auto_import_cycle(f.path(), &lock);
        let c = Connection::open(f.path()).unwrap();
        let runs: i64 = c
            .query_row("SELECT COUNT(*) FROM langfuse_import_runs", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            runs, 0,
            "a disabled integration records no import run and probes nothing"
        );
    }

    #[test]
    fn auto_import_cycle_skips_when_an_import_is_already_in_progress() {
        // Hold the slot, then invoke the cycle: it must return immediately without opening the DB or
        // probing, because another import holds the slot (serialization, B3).
        let f = NamedTempFile::new().unwrap();
        {
            let c = Connection::open(f.path()).unwrap();
            init_db(&c).unwrap();
        }
        let lock = Mutex::new(());
        let _held =
            try_acquire_import_slot(&lock).expect("hold the slot to simulate an in-flight import");
        run_auto_import_cycle(f.path(), &lock); // must skip, not block on the held slot
        let c = Connection::open(f.path()).unwrap();
        let runs: i64 = c
            .query_row("SELECT COUNT(*) FROM langfuse_import_runs", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(
            runs, 0,
            "the cycle skips entirely while another import holds the slot"
        );
    }

    #[test]
    fn auto_import_interval_floors_and_defaults() {
        // A pure check of the interval resolver's bounds (the env override is read process-wide, so
        // this asserts only the default + floor constants the resolver applies).
        assert_eq!(AUTO_IMPORT_INTERVAL_SECS, 900);
        assert!(AUTO_IMPORT_MIN_INTERVAL_SECS <= AUTO_IMPORT_INTERVAL_SECS);
        assert!(auto_import_interval() >= Duration::from_secs(AUTO_IMPORT_MIN_INTERVAL_SECS));
    }

    // ----- TASK-032 B: accept / dismiss IPC + AI-origin entry + reporting separation -----------
    // The engine→table path is covered by `suggestions/tests.rs`; these exercise the Workstream-B
    // surface directly by seeding a `time_entry_suggestions` row, so they stay deterministic and
    // independent of the importer.

    /// Insert one pending suggestion row. `start_ts`/`end_ts` are local `'YYYY-MM-DD HH:MM:SS'` (or
    /// `None` for an untimed "needs manual time" block); `duration` mirrors what the engine would store.
    fn seed_suggestion(
        c: &Connection,
        id: &str,
        project_id: &str,
        date: &str,
        start_ts: Option<&str>,
        end_ts: Option<&str>,
        duration: Option<i64>,
    ) {
        c.execute(
            "INSERT INTO time_entry_suggestions
                (id,project_id,date,block_start_ts,block_end_ts,duration_minutes,trace_count,
                 session_count,total_tokens,cost_total,cost_currency,health,confidence,source,reason,
                 status,accepted_entry_id,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,2,1,100,1.5,'USD','healthy','high','langfuse:veronavi',
                     '2 Langfuse traces, 1 session in env `veronavi`','pending',NULL,?7,?7)",
            params![id, project_id, date, start_ts, end_ts, duration, now()],
        )
        .unwrap();
    }

    fn seeded_project(c: &Connection) -> String {
        create_project_repo(
            c,
            ProjectInput {
                name: "Vero".into(),
                notes: None,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn accept_creates_exactly_one_ai_entry_marks_accepted_and_is_decided_once() {
        let mut c = conn();
        let pid = seeded_project(&c);
        seed_suggestion(
            &c,
            "s1",
            &pid,
            "2026-04-01",
            Some("2026-04-01 09:00:00"),
            Some("2026-04-01 10:00:00"),
            Some(60),
        );

        let entry = accept_suggestion_repo(&mut c, "s1".into(), None).unwrap();
        assert_eq!(entry.duration_minutes, 60);
        assert_eq!(entry.start_time, "09:00");
        assert_eq!(entry.end_time, "10:00");
        assert_eq!(entry.origin, "ai_suggested");
        // The provenance note is recorded and secret-free.
        assert!(entry.note.as_deref().unwrap().contains("AI-suggested"));

        // Exactly one ai_suggested entry exists.
        let ai: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM time_entries WHERE origin='ai_suggested'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ai, 1);

        // The suggestion is marked accepted and linked to the entry.
        let (status, eid): (String, Option<String>) = c
            .query_row(
                "SELECT status, accepted_entry_id FROM time_entry_suggestions WHERE id='s1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "accepted");
        assert_eq!(eid.as_deref(), Some(entry.id.as_str()));

        // Re-deciding an accepted suggestion is rejected, and no second entry is written.
        assert!(accept_suggestion_repo(&mut c, "s1".into(), None).is_err());
        assert!(dismiss_suggestion_repo(&c, "s1".into()).is_err());
        let total: i64 = c
            .query_row("SELECT COUNT(*) FROM time_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 1);
    }

    #[test]
    fn accept_of_untimed_block_requires_edits_and_never_invents_a_duration() {
        let mut c = conn();
        let pid = seeded_project(&c);
        seed_suggestion(&c, "s2", &pid, "2026-04-02", None, None, None);

        // Without edits supplying a time, accept errors and writes nothing (absence ≠ zero).
        let err = accept_suggestion_repo(&mut c, "s2".into(), None).unwrap_err();
        assert!(err.contains("start and end time"));
        let written: i64 = c
            .query_row("SELECT COUNT(*) FROM time_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(written, 0);
        // The suggestion is still pending — a failed accept did not decide it.
        let status: String = c
            .query_row(
                "SELECT status FROM time_entry_suggestions WHERE id='s2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "pending");

        // Supplying start/end edits lets it be accepted with the user-provided span.
        let edits = suggestions::SuggestionEdit {
            start_time: Some("13:00".into()),
            end_time: Some("14:30".into()),
            ..Default::default()
        };
        let entry = accept_suggestion_repo(&mut c, "s2".into(), Some(edits)).unwrap();
        assert_eq!(entry.duration_minutes, 90);
        assert_eq!(entry.origin, "ai_suggested");
    }

    #[test]
    fn dismiss_writes_no_entry_is_idempotent_and_cannot_undo_an_accept() {
        let mut c = conn();
        let pid = seeded_project(&c);
        seed_suggestion(
            &c,
            "s3",
            &pid,
            "2026-04-03",
            Some("2026-04-03 09:00:00"),
            Some("2026-04-03 10:00:00"),
            Some(60),
        );

        dismiss_suggestion_repo(&c, "s3".into()).unwrap();
        let written: i64 = c
            .query_row("SELECT COUNT(*) FROM time_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(written, 0, "dismiss writes no time entry");
        let status: String = c
            .query_row(
                "SELECT status FROM time_entry_suggestions WHERE id='s3'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "dismissed");

        // Dismiss is idempotent; a dismissed suggestion can no longer be accepted.
        assert!(dismiss_suggestion_repo(&c, "s3".into()).is_ok());
        assert!(accept_suggestion_repo(&mut c, "s3".into(), None).is_err());

        // An accepted suggestion cannot be dismissed.
        seed_suggestion(
            &c,
            "s4",
            &pid,
            "2026-04-04",
            Some("2026-04-04 09:00:00"),
            Some("2026-04-04 10:00:00"),
            Some(60),
        );
        accept_suggestion_repo(&mut c, "s4".into(), None).unwrap();
        assert!(dismiss_suggestion_repo(&c, "s4".into())
            .unwrap_err()
            .contains("accepted"));
    }

    #[test]
    fn summary_and_csv_report_human_and_ai_time_separately() {
        let mut c = conn();
        let pid = seeded_project(&c);
        // 60 min of human time + 30 min of accepted AI time, same project and day.
        create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: pid.clone(),
                date: "2026-05-01".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: None,
            },
        )
        .unwrap();
        seed_suggestion(
            &c,
            "s5",
            &pid,
            "2026-05-01",
            Some("2026-05-01 11:00:00"),
            Some("2026-05-01 11:30:00"),
            Some(30),
        );
        accept_suggestion_repo(&mut c, "s5".into(), None).unwrap();

        // DEC-003: human total is unchanged in meaning; AI time is a distinct figure.
        let s = summary_repo(
            &c,
            "2026-05-01".into(),
            "2026-05-01".into(),
            Some(pid.clone()),
        )
        .unwrap();
        assert_eq!(s[0].duration_minutes, 60, "human minutes (manual only)");
        assert_eq!(s[0].ai_minutes, 30, "AI minutes reported separately");

        // CSV labels each row's origin so AI time is never folded into the human column.
        let f = NamedTempFile::new().unwrap();
        export_csv_repo(
            &c,
            "2026-05-01".into(),
            "2026-05-01".into(),
            Some(pid),
            f.path(),
        )
        .unwrap();
        let csv = std::fs::read_to_string(f.path()).unwrap();
        assert!(
            csv.lines().next().unwrap().contains("origin"),
            "header has origin"
        );
        assert!(csv.contains(",manual,"), "manual row labeled");
        assert!(csv.contains(",ai_suggested,"), "ai row labeled");
    }

    // ----- TASK-034 A: accept never stores a zero/negative span (DEC-034 / day-end DEC-035) ----------

    #[test]
    fn accept_of_same_minute_block_bumps_end_and_never_stores_zero() {
        let mut c = conn();
        let pid = seeded_project(&c);
        // Engine artifact: a block contained in one clock minute (second-precision bounds, duration 1).
        // Re-derived at HH:MM both ends read "09:00"; without the bump accept would reject start == end.
        seed_suggestion(
            &c,
            "z1",
            &pid,
            "2026-06-01",
            Some("2026-06-01 09:00:10"),
            Some("2026-06-01 09:00:50"),
            Some(1),
        );
        let entry = accept_suggestion_repo(&mut c, "z1".into(), None).unwrap();
        assert_eq!(entry.start_time, "09:00");
        assert_eq!(
            entry.end_time, "09:01",
            "end rounded up to start + duration"
        );
        assert!(
            entry.end_time > entry.start_time,
            "span is strictly positive"
        );
        assert!(entry.duration_minutes >= 1, "never zero");
        assert_eq!(entry.duration_minutes, 1);
        assert_eq!(entry.origin, "ai_suggested");
    }

    #[test]
    fn accept_of_same_minute_block_without_engine_duration_bumps_by_minimum_one() {
        let mut c = conn();
        let pid = seeded_project(&c);
        // Defensive: same-minute bounds but no engine duration → minimum 1 minute, still never zero.
        seed_suggestion(
            &c,
            "z2",
            &pid,
            "2026-06-02",
            Some("2026-06-02 14:30:05"),
            Some("2026-06-02 14:30:40"),
            None,
        );
        let entry = accept_suggestion_repo(&mut c, "z2".into(), None).unwrap();
        assert_eq!(entry.start_time, "14:30");
        assert_eq!(entry.end_time, "14:31");
        assert_eq!(entry.duration_minutes, 1, "minimum one minute");
    }

    #[test]
    fn accept_of_day_end_same_minute_block_anchors_on_end_at_2359() {
        // DEC-035: at the day's last minute a forward bump would cross midnight and (under the old
        // clamp) collapse back to `start == end`, hard-erroring. Anchor on the end instead: keep
        // `23:59` and derive the start backward, so the span stays positive and same-day (`23:58 → 23:59`).
        let mut c = conn();
        let pid = seeded_project(&c);
        seed_suggestion(
            &c,
            "z3",
            &pid,
            "2026-06-12",
            Some("2026-06-12 23:59:10"),
            Some("2026-06-12 23:59:50"),
            Some(1),
        );
        let entry = accept_suggestion_repo(&mut c, "z3".into(), None).unwrap();
        assert_eq!(
            entry.start_time, "23:58",
            "start derived backward from 23:59"
        );
        assert_eq!(
            entry.end_time, "23:59",
            "end anchored at the day's last minute"
        );
        assert!(
            entry.end_time > entry.start_time,
            "span is strictly positive"
        );
        assert_eq!(entry.duration_minutes, 1, "never zero, no midnight cross");
        assert_eq!(entry.origin, "ai_suggested");
    }

    #[test]
    fn manual_entry_with_equal_start_and_end_is_still_rejected() {
        // DEC-034 is scoped to the accept path only — the manual path keeps rejecting start == end.
        let c = conn();
        let pid = seeded_project(&c);
        assert!(create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: pid,
                date: "2026-06-03".into(),
                start_time: "09:00".into(),
                end_time: "09:00".into(),
                note: None,
            }
        )
        .is_err());
    }

    // ----- TASK-034 B: AI cost reaches the entry, Reports summary, and CSV (DEC-003 completion) -------

    #[test]
    fn accept_copies_suggestion_cost_provenance_onto_the_entry() {
        let mut c = conn();
        let pid = seeded_project(&c);
        // seed_suggestion seeds cost_total=1.5, cost_currency='USD'.
        seed_suggestion(
            &c,
            "k1",
            &pid,
            "2026-07-01",
            Some("2026-07-01 09:00:00"),
            Some("2026-07-01 10:00:00"),
            Some(60),
        );
        let entry = accept_suggestion_repo(&mut c, "k1".into(), None).unwrap();
        assert_eq!(entry.cost_total, Some(1.5), "cost copied verbatim");
        assert_eq!(entry.cost_currency.as_deref(), Some("USD"));
    }

    #[test]
    fn summary_and_csv_carry_ai_cost_separately_and_manual_cost_is_null() {
        let mut c = conn();
        let pid = seeded_project(&c);
        // Manual entry — no AI cost (absence ≠ zero → NULL, never 0).
        let m = create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: pid.clone(),
                date: "2026-08-01".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: None,
            },
        )
        .unwrap();
        assert!(m.cost_total.is_none(), "manual entry carries no AI cost");
        assert!(m.cost_currency.is_none());
        // Accepted AI suggestion on the same project/day — seed cost 1.5 USD.
        seed_suggestion(
            &c,
            "k2",
            &pid,
            "2026-08-01",
            Some("2026-08-01 11:00:00"),
            Some("2026-08-01 11:30:00"),
            Some(30),
        );
        accept_suggestion_repo(&mut c, "k2".into(), None).unwrap();

        let s = summary_repo(
            &c,
            "2026-08-01".into(),
            "2026-08-01".into(),
            Some(pid.clone()),
        )
        .unwrap();
        assert_eq!(
            s[0].duration_minutes, 60,
            "human minutes unchanged in meaning"
        );
        assert_eq!(s[0].ai_minutes, 30);
        assert_eq!(s[0].ai_cost_total, Some(1.5), "AI cost summed separately");
        assert_eq!(s[0].ai_cost_currency.as_deref(), Some("USD"));

        // A range with only the manual entry → AI cost stays NULL (absence ≠ zero).
        create_entry_repo(
            &c,
            TimeEntryInput {
                project_id: pid.clone(),
                date: "2026-08-02".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: None,
            },
        )
        .unwrap();
        let s2 = summary_repo(
            &c,
            "2026-08-02".into(),
            "2026-08-02".into(),
            Some(pid.clone()),
        )
        .unwrap();
        assert_eq!(s2[0].ai_minutes, 0);
        assert!(
            s2[0].ai_cost_total.is_none(),
            "no AI rows ⇒ NULL cost, never 0"
        );

        // CSV header gains the cost columns; the AI row carries its currency.
        let f = NamedTempFile::new().unwrap();
        export_csv_repo(
            &c,
            "2026-08-01".into(),
            "2026-08-01".into(),
            Some(pid),
            f.path(),
        )
        .unwrap();
        let csv = std::fs::read_to_string(f.path()).unwrap();
        let header = csv.lines().next().unwrap();
        assert!(header.contains("cost_total"), "CSV header has cost_total");
        assert!(
            header.contains("cost_currency"),
            "CSV header has cost_currency"
        );
        assert!(
            csv.contains(",1.5,USD,"),
            "AI row carries cost and currency"
        );
        assert!(
            csv.contains(",manual,,,"),
            "manual row's cost cells are empty (NULL ≠ 0)"
        );
    }
}
