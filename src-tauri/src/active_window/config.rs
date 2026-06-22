//! Active-window store configuration. Retention window and title mode, with explicit
//! precedence: stored settings table > env vars > compile-time defaults.

use rusqlite::Connection;

use super::model::TitleMode;

pub const DEFAULT_RETENTION_DAYS: i64 = 30;

/// Capture-loop defaults (TASK-048). Tick cadence and idle thresholds; all overridable with the
/// same `settings` table > env > default precedence as `ActiveWindowConfig`.
pub const DEFAULT_SAMPLE_SECONDS: u64 = 5;
pub const DEFAULT_IDLE_CANDIDATE_SECONDS: u64 = 60;
pub const DEFAULT_AWAY_SECONDS: u64 = 300;

#[derive(Debug, Clone)]
pub struct ActiveWindowConfig {
    pub retention_days: i64,
    pub title_mode: TitleMode,
}

/// Resolved configuration for the zero-permission capture loop (TASK-048). Bundles the
/// capture-only knobs (enable switch, tick cadence, idle thresholds) with the storage-side
/// `retention_days` / `title_mode` already owned by [`ActiveWindowConfig`]. Default capture state
/// is **OFF**: nothing samples until `active_window_capture_enabled` (or the env override) is set.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub capture_enabled: bool,
    pub sample_seconds: u64,
    pub idle_candidate_seconds: u64,
    pub away_seconds: u64,
    pub retention_days: i64,
    pub title_mode: TitleMode,
}

impl CaptureConfig {
    /// Resolve with `settings` table > env > compile-time default precedence (mirrors
    /// [`ActiveWindowConfig::from_settings`]).
    ///
    /// Settings keys (env fallbacks in parentheses):
    /// - `active_window_capture_enabled` (`VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED`) — default **false**
    /// - `active_window_sample_seconds` (`VIRE_ACTIVE_WINDOW_SAMPLE_SECONDS`) — default 5, must be > 0
    /// - `active_window_idle_candidate_seconds` (`VIRE_ACTIVE_WINDOW_IDLE_CANDIDATE_SECONDS`) — default 60
    /// - `active_window_idle_away_seconds` (`VIRE_ACTIVE_WINDOW_IDLE_AWAY_SECONDS`) — default 300
    ///
    /// `retention_days` and `title_mode` are delegated to [`ActiveWindowConfig::from_settings`].
    pub fn from_settings(conn: &Connection) -> rusqlite::Result<Self> {
        let base = ActiveWindowConfig::from_settings(conn)?;

        let capture_enabled = db_setting_str(conn, "active_window_capture_enabled")?
            .map(|v| parse_bool(&v))
            .or_else(|| env_nonempty("VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED").map(|v| parse_bool(&v)))
            .unwrap_or(false);

        let sample_seconds = positive_u64_setting(
            conn,
            "active_window_sample_seconds",
            "VIRE_ACTIVE_WINDOW_SAMPLE_SECONDS",
            DEFAULT_SAMPLE_SECONDS,
        )?;
        let idle_candidate_seconds = positive_u64_setting(
            conn,
            "active_window_idle_candidate_seconds",
            "VIRE_ACTIVE_WINDOW_IDLE_CANDIDATE_SECONDS",
            DEFAULT_IDLE_CANDIDATE_SECONDS,
        )?;
        let away_seconds = positive_u64_setting(
            conn,
            "active_window_idle_away_seconds",
            "VIRE_ACTIVE_WINDOW_IDLE_AWAY_SECONDS",
            DEFAULT_AWAY_SECONDS,
        )?;

        Ok(CaptureConfig {
            capture_enabled,
            sample_seconds,
            idle_candidate_seconds,
            away_seconds,
            retention_days: base.retention_days,
            title_mode: base.title_mode,
        })
    }
}

impl ActiveWindowConfig {
    /// Resolve configuration with explicit precedence:
    /// `settings` table > env vars > compile-time defaults.
    ///
    /// Keys read from the `settings` table:
    /// - `active_window_retention_days` — positive integer, default 30
    /// - `active_window_title_mode` — `"stored"` or `"redacted"`, default `"redacted"`
    ///
    /// Env-var fallbacks (power-use / tests):
    /// - `VIRE_ACTIVE_WINDOW_RETENTION_DAYS`
    /// - `VIRE_ACTIVE_WINDOW_TITLE_MODE`
    pub fn from_settings(conn: &Connection) -> rusqlite::Result<Self> {
        let retention_days = db_setting_str(conn, "active_window_retention_days")?
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .or_else(env_retention)
            .unwrap_or(DEFAULT_RETENTION_DAYS);

        let title_mode = db_setting_str(conn, "active_window_title_mode")?
            .map(|v| parse_title_mode(&v))
            .or_else(env_title_mode)
            .unwrap_or(TitleMode::Redacted);

        Ok(ActiveWindowConfig {
            retention_days,
            title_mode,
        })
    }
}

fn db_setting_str(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    match conn.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
        r.get::<_, String>(0)
    }) {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

fn env_retention() -> Option<i64> {
    env_nonempty("VIRE_ACTIVE_WINDOW_RETENTION_DAYS")
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v > 0)
}

fn env_title_mode() -> Option<TitleMode> {
    env_nonempty("VIRE_ACTIVE_WINDOW_TITLE_MODE").map(|v| parse_title_mode(&v))
}

fn parse_title_mode(s: &str) -> TitleMode {
    if s.trim().eq_ignore_ascii_case("stored") {
        TitleMode::Stored
    } else {
        TitleMode::Redacted
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Resolve a strictly-positive `u64` knob with `settings` > env > default precedence. A stored or
/// env value that is non-numeric or `<= 0` is ignored (falls through), so a bad override can never
/// disable sampling by yielding a zero/negative cadence.
fn positive_u64_setting(
    conn: &Connection,
    db_key: &str,
    env_key: &str,
    default: u64,
) -> rusqlite::Result<u64> {
    Ok(db_setting_str(conn, db_key)?
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .or_else(|| {
            env_nonempty(env_key)
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|v| *v > 0)
        })
        .unwrap_or(default))
}

/// Truthy-only boolean parse for the capture enable switch. Only an explicit affirmative
/// (`1`/`true`/`yes`/`on`, case-insensitive) enables capture; everything else keeps it OFF.
fn parse_bool(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}
