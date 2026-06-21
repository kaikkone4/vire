//! Active-window store configuration. Retention window and title mode, readable from env vars
//! (power-use / tests) with the `settings` table as the user-facing override path (future privacy UI).

use super::model::TitleMode;

pub const DEFAULT_RETENTION_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct ActiveWindowConfig {
    pub retention_days: i64,
    pub title_mode: TitleMode,
}

impl ActiveWindowConfig {
    /// Read from env with safe defaults.
    /// - `VIRE_ACTIVE_WINDOW_RETENTION_DAYS` — default 30; also `settings.active_window_retention_days`.
    /// - `VIRE_ACTIVE_WINDOW_TITLE_MODE` — default `redacted`; also `settings.active_window_title_mode`.
    pub fn from_env() -> Self {
        let retention_days = env_nonempty("VIRE_ACTIVE_WINDOW_RETENTION_DAYS")
            .and_then(|v| v.parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(DEFAULT_RETENTION_DAYS);
        let title_mode = env_nonempty("VIRE_ACTIVE_WINDOW_TITLE_MODE")
            .map(|v| parse_title_mode(&v))
            .unwrap_or(TitleMode::Redacted);
        ActiveWindowConfig { retention_days, title_mode }
    }
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
