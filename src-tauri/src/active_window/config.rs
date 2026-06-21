//! Active-window store configuration. Retention window and title mode, with explicit
//! precedence: stored settings table > env vars > compile-time defaults.

use rusqlite::Connection;

use super::model::TitleMode;

pub const DEFAULT_RETENTION_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct ActiveWindowConfig {
    pub retention_days: i64,
    pub title_mode: TitleMode,
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
