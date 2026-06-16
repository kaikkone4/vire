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

use api::{LangfuseApi, ReqwestLangfuseApi};
use config::ImporterConfig;
use model::{ApiError, HealthState, ImportWindow};

pub use importer::ImportReport;

use crate::settings::secret_store::KeyringSecretStore;

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
/// the network call off the UI's database lock. Returns a secret-free [`ImportReport`] on success,
/// or a secret-free error string on failure (including the in-band persist-failure surfacing).
pub fn run_blocking_import(db_path: &Path) -> Result<ImportReport, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    store::migrate(&conn).map_err(|e| e.to_string())?;
    // Settings-first resolution (TASK-026): stored settings + Keychain credentials win; process env
    // is the marked dev fallback. The Keychain read is on this dedicated import thread, off the UI.
    // A genuine Keychain read failure propagates a coarse, secret-free error rather than silently
    // resolving to no/partial credentials.
    let config = crate::settings::resolve_config(&conn, &KeyringSecretStore::new())?;
    let api = ReqwestLangfuseApi::new(config.clone()).map_err(|e| e.message)?;
    let window = recent_window(7);
    let summaries = importer::run_import(&api, &conn, &config, &window);
    // Build the secret-free diagnostics report from the counts the importer just computed, then
    // apply the TASK-021 in-band persist-failure check: a run that could not be persisted still
    // surfaces as `Err` (never a stale-healthy result), so the report is returned only for a run
    // that did persist.
    let report = ImportReport::from_summaries(&summaries);
    import_result(&summaries)?;
    Ok(report)
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

/// Coarse, secret-free verdict for the in-app **Test connection** action (TASK-026). Reports only
/// whether the configured endpoint is reachable and the credentials authenticate — never a secret,
/// raw response body, or stack-internal detail.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TestConnectionResult {
    pub ok: bool,
    pub verdict: String,
    pub message: String,
}

impl TestConnectionResult {
    fn reachable() -> Self {
        TestConnectionResult {
            ok: true,
            verdict: "reachable".into(),
            message: "Langfuse is reachable and the credentials authenticated.".into(),
        }
    }

    fn invalid_config(message: impl Into<String>) -> Self {
        TestConnectionResult {
            ok: false,
            verdict: "invalid_config".into(),
            message: message.into(),
        }
    }

    /// The integration is disabled: report an explicit, secret-free `disabled` verdict. The caller
    /// reaches this **without** resolving credentials or opening a socket, so no Keychain read or
    /// network probe occurs while disabled. Mirrors the disabled health snapshot's posture — a
    /// disabled integration is an explicit state, never "reachable" or zero.
    pub fn disabled() -> Self {
        TestConnectionResult {
            ok: false,
            verdict: "disabled".into(),
            message: "Langfuse integration is disabled — enable it to test the connection.".into(),
        }
    }

    /// Map a probe failure to a coarse verdict. The source `ApiError::message` is secret-free by
    /// construction (`langfuse/api.rs`), but we emit our own stable strings here regardless.
    fn from_api_error(error: &ApiError) -> Self {
        let (verdict, message) = match error.health() {
            HealthState::Unavailable => (
                "unavailable",
                "Could not reach Langfuse — the local stack appears to be down.",
            ),
            HealthState::AuthOrNetworkError => (
                "auth_or_network_error",
                "Langfuse rejected the credentials or could not be reached (auth or network).",
            ),
            _ => (
                "unknown",
                "Langfuse returned an unexpected response — connection state is unknown.",
            ),
        };
        TestConnectionResult {
            ok: false,
            verdict: verdict.into(),
            message: message.into(),
        }
    }
}

/// Run a single read-only availability probe against the resolved config and return a coarse
/// verdict. A non-loopback `local` target (SEC-002) is refused before any network call. Intended to
/// be called inside the bounded `run_bounded` pattern so a hung probe cannot freeze the UI.
pub fn test_connection(config: ImporterConfig) -> TestConnectionResult {
    let api = match ReqwestLangfuseApi::new(config) {
        Ok(api) => api,
        Err(error) => return TestConnectionResult::invalid_config(error.message),
    };
    match api.probe() {
        Ok(()) => TestConnectionResult::reachable(),
        Err(error) => TestConnectionResult::from_api_error(&error),
    }
}

#[cfg(test)]
mod tests;
