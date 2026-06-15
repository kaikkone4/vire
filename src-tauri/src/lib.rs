mod langfuse;
mod runtime_observer;

use chrono::{Local, NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}, sync::{mpsc, Mutex, MutexGuard}, time::Duration};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use langfuse::store::SourceHealthSnapshot;
use runtime_observer::model::RuntimeReconciliationSnapshot;

struct AppState { db: Mutex<Connection>, db_path: PathBuf }

type CmdResult<T> = Result<T, String>;

fn db_conn<'a>(state: &'a State<'a, AppState>) -> CmdResult<MutexGuard<'a, Connection>> {
    state.db.lock().map_err(|_| "Database lock is unavailable after an internal error; please restart Vire".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project { pub id: String, pub name: String, pub notes: Option<String>, pub archived: bool, pub created_at: String, pub updated_at: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry { pub id: String, pub project_id: String, pub project_name: String, pub date: String, pub start_time: String, pub end_time: String, pub duration_minutes: i64, pub note: Option<String>, pub created_at: String, pub updated_at: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInput { pub name: String, pub notes: Option<String> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntryInput { pub project_id: String, pub date: String, pub start_time: String, pub end_time: String, pub note: Option<String> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRow { pub project_id: String, pub project_name: String, pub duration_minutes: i64 }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatus { pub mode: String, pub message: String }

const MAX_PROJECT_NAME_LEN: usize = 120;
const MAX_PROJECT_NOTES_LEN: usize = 5000;
const MAX_ENTRY_NOTE_LEN: usize = 2000;

fn now() -> String { Local::now().naive_local().format("%Y-%m-%d %H:%M:%S").to_string() }

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
    langfuse::store::migrate(conn)?;
    runtime_observer::store::migrate(conn)
}

fn clean_opt(s: Option<String>) -> Option<String> { s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) }) }
fn validate_len(value: &str, max: usize, label: &str) -> Result<(), String> { if value.chars().count() > max { Err(format!("{label} must be {max} characters or fewer")) } else { Ok(()) } }
fn validate_project_input(input: ProjectInput) -> Result<(String, Option<String>), String> { let name = input.name.trim().to_string(); if name.is_empty() { return Err("Project name cannot be empty".into()); } validate_len(&name, MAX_PROJECT_NAME_LEN, "Project name")?; let notes = clean_opt(input.notes); if let Some(n)=&notes { validate_len(n, MAX_PROJECT_NOTES_LEN, "Project notes")?; } Ok((name, notes)) }
fn validate_entry_note(note: Option<String>) -> Result<Option<String>, String> { let note = clean_opt(note); if let Some(n)=&note { validate_len(n, MAX_ENTRY_NOTE_LEN, "Entry note")?; } Ok(note) }
fn parse_duration(date: &str, start: &str, end: &str) -> Result<i64, String> {
    let s = NaiveDateTime::parse_from_str(&format!("{} {}", date, start), "%Y-%m-%d %H:%M").map_err(|_| "Start date/time must be valid".to_string())?;
    let e = NaiveDateTime::parse_from_str(&format!("{} {}", date, end), "%Y-%m-%d %H:%M").map_err(|_| "End date/time must be valid".to_string())?;
    let mins = (e - s).num_minutes();
    if mins <= 0 { return Err("End time must be after start time".into()); }
    Ok(mins)
}
fn parse_date(date: &str) -> Result<NaiveDate, String> { NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| "Date must use YYYY-MM-DD".into()) }
fn validate_date(date: &str) -> Result<(), String> { parse_date(date).map(|_|()) }
fn validate_date_range(start: &str, end: &str) -> Result<(), String> { let s=parse_date(start)?; let e=parse_date(end)?; if s > e { Err("Start date must be on or before end date".into()) } else { Ok(()) } }
fn project_exists_active(conn: &Connection, id: &str) -> Result<bool, String> {
    conn.query_row("SELECT archived FROM projects WHERE id=?1", params![id], |r| r.get::<_, i64>(0)).optional().map_err(|e| e.to_string()).map(|v| v == Some(0))
}
fn existing_entry_project(conn: &Connection, id: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT project_id FROM time_entries WHERE id=?1", params![id], |r| r.get(0)).optional().map_err(|e| e.to_string())
}

pub fn create_project_repo(conn: &Connection, input: ProjectInput) -> Result<Project, String> {
    let (name, notes) = validate_project_input(input)?;
    let id = Uuid::new_v4().to_string(); let ts = now();
    conn.execute("INSERT INTO projects(id,name,notes,archived,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![id, name, notes, ts]).map_err(|e| e.to_string())?;
    get_project(conn, &id)
}
fn get_project(conn: &Connection, id: &str) -> Result<Project, String> {
    conn.query_row("SELECT id,name,notes,archived,created_at,updated_at FROM projects WHERE id=?1", params![id], |r| Ok(Project { id:r.get(0)?, name:r.get(1)?, notes:r.get(2)?, archived:r.get::<_,i64>(3)? != 0, created_at:r.get(4)?, updated_at:r.get(5)? })).map_err(|e| e.to_string())
}
pub fn update_project_repo(conn: &Connection, id: String, input: ProjectInput) -> Result<Project, String> {
    let (name, notes) = validate_project_input(input)?;
    conn.execute("UPDATE projects SET name=?1, notes=?2, updated_at=?3 WHERE id=?4", params![name, notes, now(), id]).map_err(|e| e.to_string())?;
    get_project(conn, &id)
}
pub fn archive_project_repo(conn: &Connection, id: String) -> Result<Project, String> {
    conn.execute("UPDATE projects SET archived=1, updated_at=?1 WHERE id=?2", params![now(), id]).map_err(|e| e.to_string())?; get_project(conn, &id)
}
pub fn list_projects_repo(conn: &Connection, include_archived: bool) -> Result<Vec<Project>, String> {
    let sql = if include_archived { "SELECT id,name,notes,archived,created_at,updated_at FROM projects ORDER BY archived,name" } else { "SELECT id,name,notes,archived,created_at,updated_at FROM projects WHERE archived=0 ORDER BY name" };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(Project { id:r.get(0)?, name:r.get(1)?, notes:r.get(2)?, archived:r.get::<_,i64>(3)? != 0, created_at:r.get(4)?, updated_at:r.get(5)? }))
        .map_err(|e| e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e| e.to_string());
    rows
}

pub fn create_entry_repo(conn: &Connection, input: TimeEntryInput) -> Result<TimeEntry, String> {
    validate_date(&input.date)?; let duration = parse_duration(&input.date, &input.start_time, &input.end_time)?;
    if !project_exists_active(conn, &input.project_id)? { return Err("New time entries must reference an active project".into()); }
    let id = Uuid::new_v4().to_string(); let ts = now(); let note = validate_entry_note(input.note)?;
    conn.execute("INSERT INTO time_entries(id,project_id,date,start_time,end_time,duration_minutes,note,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)", params![id,input.project_id,input.date,input.start_time,input.end_time,duration,note,ts]).map_err(|e| e.to_string())?;
    get_entry(conn, &id)
}
fn get_entry(conn: &Connection, id: &str) -> Result<TimeEntry, String> {
    conn.query_row("SELECT e.id,e.project_id,p.name,e.date,e.start_time,e.end_time,e.duration_minutes,e.note,e.created_at,e.updated_at FROM time_entries e JOIN projects p ON p.id=e.project_id WHERE e.id=?1", params![id], row_to_entry).map_err(|e| e.to_string())
}
fn row_to_entry(r: &rusqlite::Row) -> rusqlite::Result<TimeEntry> { Ok(TimeEntry { id:r.get(0)?, project_id:r.get(1)?, project_name:r.get(2)?, date:r.get(3)?, start_time:r.get(4)?, end_time:r.get(5)?, duration_minutes:r.get(6)?, note:r.get(7)?, created_at:r.get(8)?, updated_at:r.get(9)? }) }
pub fn update_entry_repo(conn: &Connection, id: String, input: TimeEntryInput) -> Result<TimeEntry, String> {
    validate_date(&input.date)?; let duration = parse_duration(&input.date, &input.start_time, &input.end_time)?;
    let existing_project = existing_entry_project(conn, &id)?.ok_or_else(|| "Time entry not found".to_string())?;
    if input.project_id != existing_project && !project_exists_active(conn, &input.project_id)? { return Err("Changing a time entry to an archived or missing project is not allowed".into()); }
    let note = validate_entry_note(input.note)?;
    conn.execute("UPDATE time_entries SET project_id=?1,date=?2,start_time=?3,end_time=?4,duration_minutes=?5,note=?6,updated_at=?7 WHERE id=?8", params![input.project_id,input.date,input.start_time,input.end_time,duration,note,now(),id]).map_err(|e| e.to_string())?; get_entry(conn, &id)
}
pub fn delete_entry_repo(conn: &Connection, id: String) -> Result<(), String> { let affected = conn.execute("DELETE FROM time_entries WHERE id=?1", params![id]).map_err(|e| e.to_string())?; if affected == 0 { return Err("Time entry not found".into()); } Ok(()) }
pub fn list_entries_repo(conn: &Connection, start: String, end: String, project_id: Option<String>) -> Result<Vec<TimeEntry>, String> {
    validate_date_range(&start, &end)?;
    let mut sql = "SELECT e.id,e.project_id,p.name,e.date,e.start_time,e.end_time,e.duration_minutes,e.note,e.created_at,e.updated_at FROM time_entries e JOIN projects p ON p.id=e.project_id WHERE e.date>=?1 AND e.date<=?2".to_string();
    if project_id.is_some() { sql.push_str(" AND e.project_id=?3"); } sql.push_str(" ORDER BY e.date DESC,e.start_time DESC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Result<Vec<_>, _> = if let Some(pid)=project_id { stmt.query_map(params![start,end,pid], row_to_entry).map_err(|e| e.to_string())?.collect() } else { stmt.query_map(params![start,end], row_to_entry).map_err(|e| e.to_string())?.collect() };
    rows.map_err(|e| e.to_string())
}
pub fn summary_repo(conn: &Connection, start: String, end: String, project_id: Option<String>) -> Result<Vec<SummaryRow>, String> {
    validate_date_range(&start, &end)?;
    let mut sql = "SELECT p.id,p.name,COALESCE(SUM(e.duration_minutes),0) FROM time_entries e JOIN projects p ON p.id=e.project_id WHERE e.date>=?1 AND e.date<=?2".to_string();
    if project_id.is_some() { sql.push_str(" AND e.project_id=?3"); } sql.push_str(" GROUP BY p.id,p.name ORDER BY p.name");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mapper = |r: &rusqlite::Row| Ok(SummaryRow { project_id:r.get(0)?, project_name:r.get(1)?, duration_minutes:r.get(2)? });
    let rows: Result<Vec<_>, _> = if let Some(pid)=project_id { stmt.query_map(params![start,end,pid], mapper).map_err(|e| e.to_string())?.collect() } else { stmt.query_map(params![start,end], mapper).map_err(|e| e.to_string())?.collect() };
    rows.map_err(|e| e.to_string())
}
fn csv_formula_neutralized(v: &str) -> String {
    let trimmed = v.trim_start_matches(|c: char| c.is_whitespace());
    // Spreadsheet apps can interpret cells as formulas after leading whitespace/control characters.
    if trimmed.starts_with(['=', '+', '-', '@']) || matches!(v.chars().next(), Some('\t' | '\r' | '\n')) {
        format!("'{v}")
    } else {
        v.to_string()
    }
}
fn csv_escape(v: &str) -> String { let safe = csv_formula_neutralized(v); if safe.contains([',','\n','\r','"']) { format!("\"{}\"", safe.replace('"', "\"\"")) } else { safe } }
pub fn export_csv_repo(conn: &Connection, start: String, end: String, project_id: Option<String>, path: &Path) -> Result<usize, String> {
    let entries = list_entries_repo(conn, start, end, project_id)?; let mut out = String::from("date,project,start_time,end_time,duration_minutes,note,total_duration_hours\n");
    for e in &entries { out.push_str(&format!("{},{},{},{},{},{},{:.2}\n", e.date, csv_escape(&e.project_name), e.start_time, e.end_time, e.duration_minutes, csv_escape(e.note.as_deref().unwrap_or("")), e.duration_minutes as f64 / 60.0)); }
    fs::write(path, out).map_err(|e| e.to_string())?; Ok(entries.len())
}

#[tauri::command] fn get_capture_status() -> CaptureStatus { CaptureStatus { mode: "Manual Mode / Capture deferred".into(), message: "Automatic activity capture is not active in v0.1. Vire stores only projects and manual time entries locally.".into() } }
#[tauri::command] fn list_projects(state: State<AppState>, include_archived: bool) -> CmdResult<Vec<Project>> { let db = db_conn(&state)?; list_projects_repo(&db, include_archived) }
#[tauri::command] fn create_project(state: State<AppState>, input: ProjectInput) -> CmdResult<Project> { let db = db_conn(&state)?; create_project_repo(&db, input) }
#[tauri::command] fn update_project(state: State<AppState>, id: String, input: ProjectInput) -> CmdResult<Project> { let db = db_conn(&state)?; update_project_repo(&db, id, input) }
#[tauri::command] fn archive_project(state: State<AppState>, id: String) -> CmdResult<Project> { let db = db_conn(&state)?; archive_project_repo(&db, id) }
#[tauri::command] fn list_time_entries(state: State<AppState>, start_date: String, end_date: String, project_id: Option<String>) -> CmdResult<Vec<TimeEntry>> { let db = db_conn(&state)?; list_entries_repo(&db, start_date, end_date, project_id) }
#[tauri::command] fn create_time_entry(state: State<AppState>, input: TimeEntryInput) -> CmdResult<TimeEntry> { let db = db_conn(&state)?; create_entry_repo(&db, input) }
#[tauri::command] fn update_time_entry(state: State<AppState>, id: String, input: TimeEntryInput) -> CmdResult<TimeEntry> { let db = db_conn(&state)?; update_entry_repo(&db, id, input) }
#[tauri::command] fn delete_time_entry(state: State<AppState>, id: String) -> CmdResult<()> { let db = db_conn(&state)?; delete_entry_repo(&db, id) }
#[tauri::command] fn get_summary(state: State<AppState>, start_date: String, end_date: String, project_id: Option<String>) -> CmdResult<Vec<SummaryRow>> { let db = db_conn(&state)?; summary_repo(&db, start_date, end_date, project_id) }
#[tauri::command] fn export_report_csv(app: tauri::AppHandle, state: State<AppState>, start_date: String, end_date: String, project_id: Option<String>) -> CmdResult<Option<usize>> {
    validate_date_range(&start_date, &end_date)?;
    let destination = app.dialog().file().add_filter("CSV", &["csv"]).set_file_name("vire-report.csv").blocking_save_file();
    let Some(destination) = destination else { return Ok(None); };
    let path = destination.into_path().map_err(|_| "CSV export destination must be a local file path".to_string())?;
    validate_csv_destination(&path)?;
    let db = db_conn(&state)?;
    export_csv_repo(&db, start_date, end_date, project_id, &path).map(Some)
}

#[tauri::command]
fn get_langfuse_source_health(state: State<AppState>) -> CmdResult<SourceHealthSnapshot> {
    let db = db_conn(&state)?;
    langfuse::health_snapshot(&db)
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

/// Ceiling for a manual import, comfortably above the reqwest 15s request / 5s connect ceilings so a
/// normal slow import is not cut off, but an indefinite hang (dependency deadlock, lock contention)
/// cannot block the UI forever (S-6).
const IMPORT_TIMEOUT_SECS: u64 = 30;
const IMPORT_TIMEOUT_MSG: &str =
    "Langfuse import did not complete within the time limit — AI usage and cost are unknown, not zero";

/// Run blocking `work` on a dedicated OS thread (off the Tauri runtime) and wait at most `timeout`.
/// Returns the work's result, or a secret-free timeout error if it does not finish in time. An
/// orphaned worker that finishes late just fails to send (the receiver is gone) and is bounded by
/// the reqwest ceilings; it persists atomically (S-3) if it completes. No new health state is added.
fn run_bounded<F>(timeout: Duration, work: F) -> CmdResult<()>
where
    F: FnOnce() -> CmdResult<()> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => Err(IMPORT_TIMEOUT_MSG.to_string()),
    }
}

#[tauri::command]
fn import_langfuse_now(state: State<AppState>) -> CmdResult<SourceHealthSnapshot> {
    // Run the blocking REST import on a dedicated OS thread (off the Tauri runtime and off the UI's
    // database lock); it uses its own SQLite connection. Bound the wait so a hung import returns a
    // secret-free error instead of blocking the UI. Then read the resulting snapshot.
    let db_path = state.db_path.clone();
    run_bounded(Duration::from_secs(IMPORT_TIMEOUT_SECS), move || {
        langfuse::run_blocking_import(&db_path)
    })?;
    let db = db_conn(&state)?;
    langfuse::health_snapshot(&db)
}

fn validate_csv_destination(path: &Path) -> Result<(), String> {
    if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("csv")) != Some(true) { return Err("CSV export destination must use a .csv extension".into()); }
    if path.is_dir() { return Err("CSV export destination must be a file, not a directory".into()); }
    Ok(())
}
fn db_path(app: &tauri::App) -> Result<PathBuf, std::io::Error> { let dir = app.path().app_data_dir().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Could not resolve Vire app data directory: {e}")))?; fs::create_dir_all(&dir)?; Ok(dir.join("vire.sqlite")) }
pub fn run() {
    tauri::Builder::default().plugin(tauri_plugin_dialog::init()).setup(|app| { let path = db_path(app)?; let conn = Connection::open(&path)?; init_db(&conn)?; app.manage(AppState { db: Mutex::new(conn), db_path: path }); Ok(()) })
        .invoke_handler(tauri::generate_handler![get_capture_status,list_projects,create_project,update_project,archive_project,list_time_entries,create_time_entry,update_time_entry,delete_time_entry,get_summary,export_report_csv,get_langfuse_source_health,import_langfuse_now,get_runtime_reconciliation])
        .run(tauri::generate_context!()).expect("error while running Vire");
}

#[cfg(test)]
mod tests {
    use super::*; use tempfile::NamedTempFile;
    fn conn() -> Connection { let c = Connection::open_in_memory().unwrap(); init_db(&c).unwrap(); c }
    #[test] fn project_crud_archive_and_active_filter() { let c=conn(); let p=create_project_repo(&c, ProjectInput{name:" Alpha ".into(), notes:Some("n".into())}).unwrap(); let p=update_project_repo(&c,p.id.clone(),ProjectInput{name:"Beta".into(),notes:None}).unwrap(); assert_eq!(p.name,"Beta"); archive_project_repo(&c,p.id).unwrap(); assert_eq!(list_projects_repo(&c,false).unwrap().len(),0); assert_eq!(list_projects_repo(&c,true).unwrap().len(),1); }
    #[test] fn manual_entry_crud_delete_and_validation() { let c=conn(); let p=create_project_repo(&c,ProjectInput{name:"P".into(),notes:None}).unwrap(); assert!(create_entry_repo(&c,TimeEntryInput{project_id:p.id.clone(),date:"2026-01-01".into(),start_time:"10:00".into(),end_time:"09:00".into(),note:None}).is_err()); let e=create_entry_repo(&c,TimeEntryInput{project_id:p.id.clone(),date:"2026-01-01".into(),start_time:"09:00".into(),end_time:"10:00".into(),note:None}).unwrap(); let e=update_entry_repo(&c,e.id.clone(),TimeEntryInput{project_id:p.id,date:"2026-01-01".into(),start_time:"09:00".into(),end_time:"10:30".into(),note:Some("done".into())}).unwrap(); assert_eq!(e.duration_minutes,90); delete_entry_repo(&c,e.id).unwrap(); assert!(list_entries_repo(&c,"2026-01-01".into(),"2026-01-01".into(),None).unwrap().is_empty()); }
    #[test] fn summaries_and_csv_filtering_escape() { let c=conn(); let p1=create_project_repo(&c,ProjectInput{name:"A, Inc".into(),notes:None}).unwrap(); let p2=create_project_repo(&c,ProjectInput{name:"B".into(),notes:None}).unwrap(); create_entry_repo(&c,TimeEntryInput{project_id:p1.id.clone(),date:"2026-02-01".into(),start_time:"09:00".into(),end_time:"10:00".into(),note:Some("said \"hi\"".into())}).unwrap(); create_entry_repo(&c,TimeEntryInput{project_id:p2.id,date:"2026-02-01".into(),start_time:"10:00".into(),end_time:"12:00".into(),note:None}).unwrap(); let s=summary_repo(&c,"2026-02-01".into(),"2026-02-01".into(),Some(p1.id.clone())).unwrap(); assert_eq!(s[0].duration_minutes,60); let f=NamedTempFile::new().unwrap(); let n=export_csv_repo(&c,"2026-02-01".into(),"2026-02-01".into(),Some(p1.id),f.path()).unwrap(); let csv=std::fs::read_to_string(f.path()).unwrap(); assert_eq!(n,1); assert!(csv.contains("\"A, Inc\"")); assert!(csv.contains("\"said \"\"hi\"\"\"")); }
    #[test] fn persistence_across_reopen() { let f=NamedTempFile::new().unwrap(); { let c=Connection::open(f.path()).unwrap(); init_db(&c).unwrap(); create_project_repo(&c,ProjectInput{name:"Persist".into(),notes:None}).unwrap(); } let c=Connection::open(f.path()).unwrap(); init_db(&c).unwrap(); assert_eq!(list_projects_repo(&c,false).unwrap()[0].name,"Persist"); }
    #[test] fn rejects_overlong_text_fields() { let c=conn(); assert!(create_project_repo(&c,ProjectInput{name:"x".repeat(MAX_PROJECT_NAME_LEN+1),notes:None}).unwrap_err().contains("Project name")); assert!(create_project_repo(&c,ProjectInput{name:"P".into(),notes:Some("x".repeat(MAX_PROJECT_NOTES_LEN+1))}).unwrap_err().contains("Project notes")); let p=create_project_repo(&c,ProjectInput{name:"P".into(),notes:None}).unwrap(); assert!(create_entry_repo(&c,TimeEntryInput{project_id:p.id,date:"2026-01-01".into(),start_time:"09:00".into(),end_time:"10:00".into(),note:Some("x".repeat(MAX_ENTRY_NOTE_LEN+1))}).unwrap_err().contains("Entry note")); }
    #[test] fn new_entries_reject_archived_projects_but_existing_can_keep_same_project() { let c=conn(); let p=create_project_repo(&c,ProjectInput{name:"Archived".into(),notes:None}).unwrap(); let e=create_entry_repo(&c,TimeEntryInput{project_id:p.id.clone(),date:"2026-01-01".into(),start_time:"09:00".into(),end_time:"10:00".into(),note:None}).unwrap(); archive_project_repo(&c,p.id.clone()).unwrap(); assert!(create_entry_repo(&c,TimeEntryInput{project_id:p.id.clone(),date:"2026-01-02".into(),start_time:"09:00".into(),end_time:"10:00".into(),note:None}).is_err()); assert!(update_entry_repo(&c,e.id,TimeEntryInput{project_id:p.id,date:"2026-01-01".into(),start_time:"09:00".into(),end_time:"10:30".into(),note:None}).is_ok()); }
    #[test] fn delete_missing_entry_returns_error() { let c=conn(); assert!(delete_entry_repo(&c,"missing".into()).unwrap_err().contains("not found")); }
    #[test] fn csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes() { assert_eq!(csv_escape("=SUM(1,2)"), "\"'=SUM(1,2)\""); assert_eq!(csv_escape("  @cmd"), "'  @cmd"); assert_eq!(csv_escape("\nplain"), "\"'\nplain\""); assert_eq!(csv_escape("line\rbreak"), "\"line\rbreak\""); assert_eq!(csv_escape(" +SUM(1,2)"), "\"' +SUM(1,2)\""); }
    #[test] fn clean_opt_preserves_nonempty_value_and_collapses_whitespace_only() { assert_eq!(clean_opt(Some(" +SUM".into())), Some(" +SUM".to_string())); assert_eq!(clean_opt(Some("  done  ".into())), Some("  done  ".to_string())); assert_eq!(clean_opt(Some("   ".into())), None); assert_eq!(clean_opt(Some("".into())), None); assert_eq!(clean_opt(None), None); }

    // ----- S-6: the manual import command is bounded ---------------------------------------

    #[test]
    fn run_bounded_times_out_promptly_with_a_secret_free_error() {
        let start = std::time::Instant::now();
        let err = run_bounded(Duration::from_millis(50), || {
            std::thread::sleep(Duration::from_millis(1500));
            Ok(())
        })
        .unwrap_err();
        // The command returned well within the ceiling rather than blocking on the 1.5s worker.
        assert!(start.elapsed() < Duration::from_secs(1), "must not block past the ceiling");
        assert!(!err.is_empty());
        for needle in ["sk-", "pk-", "password", "token", "Bearer", "Authorization"] {
            assert!(!err.contains(needle), "timeout error must be secret-free, found {needle}");
        }
    }

    #[test]
    fn run_bounded_returns_the_works_result_within_the_ceiling() {
        assert!(run_bounded(Duration::from_secs(5), || Ok(())).is_ok());
        let err = run_bounded(Duration::from_secs(5), || Err("import failed".to_string())).unwrap_err();
        assert_eq!(err, "import failed", "a normal failure is surfaced verbatim, not masked as a timeout");
    }
}

