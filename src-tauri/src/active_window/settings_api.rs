//! Validated read/write seam over the existing zero-permission capture config (TASK-056 A).
//!
//! This is the thin backend behind the Settings "Active-window capture" panel. It reads and writes
//! ONLY the five `settings` keys that [`CaptureConfig::from_settings`] already resolves each tick,
//! and projects the read-only status snapshot (TASK-056 B). No schema change, no new native/TCC
//! surface, no network. `title_mode` stays `redacted` and is never accepted as input (DEC-044) —
//! window titles require an Accessibility grant that this task does not take, so a "store titles"
//! toggle would be misleading. Writes take effect on the capture loop's next tick (no restart).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::config::CaptureConfig;
use super::model::CaptureStatusView;
use super::store;

// ----- validation bounds (design §1) -----------------------------------------------------------

pub const MIN_SAMPLE_SECONDS: u64 = 1;
pub const MAX_SAMPLE_SECONDS: u64 = 3600;
pub const MIN_IDLE_CANDIDATE_SECONDS: u64 = 1;
/// Upper cap shared by the idle-candidate and away thresholds (24h).
pub const MAX_IDLE_SECONDS: u64 = 86_400;
pub const MIN_RETENTION_DAYS: i64 = 1;
pub const MAX_RETENTION_DAYS: i64 = 3650;

/// `settings` table keys — MUST match [`CaptureConfig::from_settings`] exactly.
const KEY_ENABLED: &str = "active_window_capture_enabled";
const KEY_SAMPLE: &str = "active_window_sample_seconds";
const KEY_IDLE_CANDIDATE: &str = "active_window_idle_candidate_seconds";
const KEY_IDLE_AWAY: &str = "active_window_idle_away_seconds";
const KEY_RETENTION: &str = "active_window_retention_days";

/// Fixed informational marker surfaced to the UI: window titles are never captured through this
/// surface (DEC-044). Not user-editable; never written back to the store.
pub const TITLE_MODE_REDACTED: &str = "redacted";

// ----- DTOs ------------------------------------------------------------------------------------

/// Renderer-facing resolved view: the config the capture loop will act on, the platform gate, the
/// fixed `title_mode` marker, and the read-only status snapshot.
#[derive(Debug, Clone, Serialize)]
pub struct CaptureSettingsView {
    /// `cfg!(target_os = "macos")`; `false` ⇒ the UI disables the controls.
    pub platform_supported: bool,
    pub capture_enabled: bool,
    pub sample_seconds: u64,
    pub idle_candidate_seconds: u64,
    pub idle_away_seconds: u64,
    pub retention_days: i64,
    /// Always [`TITLE_MODE_REDACTED`] — informational, never accepted from input.
    pub title_mode: String,
    pub status: CaptureStatusView,
}

/// Non-title knobs written by the renderer. serde DROPS any non-allowlisted key (runtime_observer
/// discipline), so `title_mode` — or any prohibited field — can never be deserialized here.
#[derive(Debug, Clone, Deserialize)]
pub struct CaptureSettingsInput {
    pub capture_enabled: bool,
    pub sample_seconds: u64,
    pub idle_candidate_seconds: u64,
    pub idle_away_seconds: u64,
    pub retention_days: i64,
}

// ----- resolve (get) ---------------------------------------------------------------------------

/// Resolve the full settings view at wall-clock `now` (`YYYY-MM-DD HH:MM:SS`). Pure read — never
/// mutates. `now` is injected so the status day bounds are a pure function of the clock (testable).
pub fn resolve_view(conn: &Connection, now: &str) -> rusqlite::Result<CaptureSettingsView> {
    let cfg = CaptureConfig::from_settings(conn)?;
    let now_day = day_of(now);
    let from_day = retention_from_day(now, cfg.retention_days);
    let status = store::capture_status_snapshot(conn, &now_day, &from_day)?;
    Ok(CaptureSettingsView {
        platform_supported: cfg!(target_os = "macos"),
        capture_enabled: cfg.capture_enabled,
        sample_seconds: cfg.sample_seconds,
        idle_candidate_seconds: cfg.idle_candidate_seconds,
        idle_away_seconds: cfg.away_seconds,
        retention_days: cfg.retention_days,
        title_mode: TITLE_MODE_REDACTED.to_string(),
        status,
    })
}

// ----- validate + persist (set) ----------------------------------------------------------------

/// Validate to safe bounds, reject (never silently clamp) on failure, else upsert exactly the five
/// capture `settings` keys in one transaction and re-resolve the fresh view. `title_mode` is never
/// accepted or written. Validation runs BEFORE the transaction opens, so a rejected write touches
/// no row and the previously-saved config stays in effect.
pub fn apply(
    conn: &Connection,
    input: &CaptureSettingsInput,
    now: &str,
) -> Result<CaptureSettingsView, String> {
    validate(input)?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let enabled = if input.capture_enabled {
        "true"
    } else {
        "false"
    };
    upsert(&tx, KEY_ENABLED, enabled).map_err(|e| e.to_string())?;
    upsert(&tx, KEY_SAMPLE, &input.sample_seconds.to_string()).map_err(|e| e.to_string())?;
    upsert(
        &tx,
        KEY_IDLE_CANDIDATE,
        &input.idle_candidate_seconds.to_string(),
    )
    .map_err(|e| e.to_string())?;
    upsert(&tx, KEY_IDLE_AWAY, &input.idle_away_seconds.to_string()).map_err(|e| e.to_string())?;
    upsert(&tx, KEY_RETENTION, &input.retention_days.to_string()).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    resolve_view(conn, now).map_err(|e| e.to_string())
}

/// Enforce the safe-bounds contract (design §1). Returns a clear, user-facing error string.
pub fn validate(input: &CaptureSettingsInput) -> Result<(), String> {
    if !(MIN_SAMPLE_SECONDS..=MAX_SAMPLE_SECONDS).contains(&input.sample_seconds) {
        return Err(format!(
            "Sample interval must be between {MIN_SAMPLE_SECONDS} and {MAX_SAMPLE_SECONDS} seconds."
        ));
    }
    if !(MIN_IDLE_CANDIDATE_SECONDS..=MAX_IDLE_SECONDS).contains(&input.idle_candidate_seconds) {
        return Err(format!(
            "Idle-candidate threshold must be between {MIN_IDLE_CANDIDATE_SECONDS} and {MAX_IDLE_SECONDS} seconds."
        ));
    }
    if input.idle_away_seconds <= input.idle_candidate_seconds {
        return Err(
            "Away threshold must be greater than the idle-candidate threshold.".to_string(),
        );
    }
    if input.idle_away_seconds > MAX_IDLE_SECONDS {
        return Err(format!(
            "Away threshold must be at most {MAX_IDLE_SECONDS} seconds."
        ));
    }
    if !(MIN_RETENTION_DAYS..=MAX_RETENTION_DAYS).contains(&input.retention_days) {
        return Err(format!(
            "Retention must be between {MIN_RETENTION_DAYS} and {MAX_RETENTION_DAYS} days."
        ));
    }
    Ok(())
}

fn upsert(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

// ----- day-bound helpers -----------------------------------------------------------------------

/// First 10 chars (`YYYY-MM-DD`) of a `YYYY-MM-DD HH:MM:SS` timestamp — matches `capture::day_of`.
fn day_of(ts: &str) -> String {
    ts.get(..10).unwrap_or(ts).to_string()
}

/// The inclusive lower day bound of the retention window: `today - retention_days`. Coarse
/// day-level bound for the status count (the precise prune boundary lives in `store::prune_expired`).
fn retention_from_day(now: &str, retention_days: i64) -> String {
    let today = day_of(now);
    match chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d") {
        Ok(d) => (d - chrono::Duration::days(retention_days.max(0)))
            .format("%Y-%m-%d")
            .to_string(),
        Err(_) => today,
    }
}
