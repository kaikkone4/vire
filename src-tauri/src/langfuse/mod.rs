//! Read-only Langfuse importer (TASK-019). Lives entirely in the Rust core: the webview CSP is
//! untouched and the renderer never reaches the network. Default source is local Docker
//! self-hosted Langfuse on loopback; Cloud is an explicit, non-default override (DEC-020).

pub mod api;
pub mod config;
pub mod importer;
pub mod model;
pub mod store;

use std::path::Path;

use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::Connection;

use api::ReqwestLangfuseApi;
use config::ImporterConfig;
use model::ImportWindow;
use store::SourceHealthSnapshot;

/// Default look-back window for a manual import (last 7 days, UTC, RFC3339).
pub fn recent_window(days: i64) -> ImportWindow {
    let to = Utc::now();
    let from = to - ChronoDuration::days(days.max(1));
    ImportWindow {
        from: from.to_rfc3339_opts(SecondsFormat::Secs, true),
        to: to.to_rfc3339_opts(SecondsFormat::Secs, true),
    }
}

/// Run one import against the real Langfuse REST API using its own SQLite connection.
///
/// Intended to be called from a dedicated OS thread (not a Tauri async context): the blocking
/// `reqwest` client must not run inside the Tauri runtime, and using a separate connection keeps
/// the network call off the UI's database lock. Returns a secret-free error string on failure.
pub fn run_blocking_import(db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    store::migrate(&conn).map_err(|e| e.to_string())?;
    let config = ImporterConfig::from_env();
    let api = ReqwestLangfuseApi::new(config.clone()).map_err(|e| e.message)?;
    let window = recent_window(7);
    importer::run_import(&api, &conn, &config, &window);
    Ok(())
}

/// Read-only health snapshot from persisted state plus the public (non-secret) config.
pub fn health_snapshot(conn: &Connection) -> Result<SourceHealthSnapshot, String> {
    let config = ImporterConfig::public_from_env();
    store::source_health_snapshot(conn, &config)
}

#[cfg(test)]
mod tests;
