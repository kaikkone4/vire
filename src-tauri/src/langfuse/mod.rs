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
    let summaries = importer::run_import(&api, &conn, &config, &window);
    import_result(&summaries)
}

/// Collapse the per-environment summaries into the manual-import command's in-band result. A run that
/// could not be persisted carries the secret-free `PERSIST_FAILURE_MSG` sentinel in its warnings;
/// when present, surface it as an `Err` so the IPC returns a non-healthy result and never falls back
/// to a stale `healthy` snapshot (TASK-021 S-4). This is the fault-independent channel: when the
/// store is unwritable, no durable failure-marker can be recorded, so correctness must come from the
/// function return, not a database read. Keys on the exact sentinel — **not** `health == Unknown`,
/// which is also produced by a legitimately-persisted indeterminate classification.
fn import_result(summaries: &[importer::ImportSummary]) -> Result<(), String> {
    let persist_failed = summaries
        .iter()
        .any(|s| s.warnings.iter().any(|w| w == importer::PERSIST_FAILURE_MSG));
    if persist_failed {
        return Err(importer::PERSIST_FAILURE_MSG.to_string());
    }
    Ok(())
}

/// Read-only health snapshot from persisted state plus the public (non-secret) config.
pub fn health_snapshot(conn: &Connection) -> Result<SourceHealthSnapshot, String> {
    let config = ImporterConfig::public_from_env();
    store::source_health_snapshot(conn, &config)
}

#[cfg(test)]
mod tests;
