//! Read-only Langfuse importer (TASK-019). Lives entirely in the Rust core: the webview CSP is
//! untouched and the renderer never reaches the network. Default source is local Docker
//! self-hosted Langfuse on loopback; Cloud is an explicit, non-default override (DEC-020).

pub mod api;
pub mod config;
pub mod discovery;
pub mod importer;
pub mod model;
pub mod store;

use std::path::Path;

use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use rusqlite::Connection;

use api::{LangfuseApi, ReqwestLangfuseApi};
use config::ImporterConfig;
use model::{ApiError, HealthState, ImportWindow};

pub use importer::ImportReport;

use crate::settings::secret_store::KeyringSecretStore;

/// How far back an import reaches (TASK-029 C / DEC-030). Resolved settings-first; the default is
/// [`ImportRange::Last30d`]. Maps to a UTC RFC3339 **range floor** via [`ImportRange::floor`]. This is
/// a non-secret value — it only ever names a fixed range keyword or a timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportRange {
    Last7d,
    Last30d,
    Last90d,
    All,
    /// A custom floor — a validated, normalized UTC RFC3339 timestamp.
    Since(String),
}

impl ImportRange {
    /// The default range when none is configured or a stored value is malformed.
    pub fn default_range() -> ImportRange {
        ImportRange::Last30d
    }

    /// The canonical stored/displayed string (`last_7d|last_30d|last_90d|all|since:<RFC3339>`).
    pub fn as_setting(&self) -> String {
        match self {
            ImportRange::Last7d => "last_7d".to_string(),
            ImportRange::Last30d => "last_30d".to_string(),
            ImportRange::Last90d => "last_90d".to_string(),
            ImportRange::All => "all".to_string(),
            ImportRange::Since(ts) => format!("since:{ts}"),
        }
    }

    /// Parse a stored/user value. A `since:` value is validated as RFC3339 and normalized to UTC. An
    /// unknown keyword or an unparseable `since:` timestamp is rejected with a **fixed, secret-free**
    /// error string that never echoes the input value, so the caller can fall back to the default
    /// with a safe note (SEC-011).
    pub fn parse(value: &str) -> Result<ImportRange, &'static str> {
        let trimmed = value.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "last_7d" => Ok(ImportRange::Last7d),
            "last_30d" => Ok(ImportRange::Last30d),
            "last_90d" => Ok(ImportRange::Last90d),
            "all" => Ok(ImportRange::All),
            lowered if lowered.starts_with("since:") => {
                // "since:" is ASCII (6 bytes); slice the original to preserve the timestamp's case.
                let raw = trimmed[6..].trim();
                match DateTime::parse_from_rfc3339(raw) {
                    Ok(dt) => Ok(ImportRange::Since(
                        dt.with_timezone(&Utc)
                            .to_rfc3339_opts(SecondsFormat::Secs, true),
                    )),
                    Err(_) => Err("import range 'since:' value is not a valid RFC3339 timestamp"),
                }
            }
            _ => Err(
                "import range value is not one of last_7d/last_30d/last_90d/all/since:<timestamp>",
            ),
        }
    }

    /// The UTC RFC3339 **range floor** — the earliest timestamp an import reaches — relative to `now`.
    pub fn floor(&self, now: DateTime<Utc>) -> String {
        let floor = match self {
            ImportRange::Last7d => now - ChronoDuration::days(7),
            ImportRange::Last30d => now - ChronoDuration::days(30),
            ImportRange::Last90d => now - ChronoDuration::days(90),
            ImportRange::All => DateTime::<Utc>::from_timestamp(0, 0).unwrap_or(now),
            ImportRange::Since(ts) => return ts.clone(),
        };
        floor.to_rfc3339_opts(SecondsFormat::Secs, true)
    }
}

/// The look-back window environment discovery scans: the **resolved import-range floor → now** — the
/// same span the import just walked (TASK-045 B). Discovery enumerates only environment *names*, so a
/// wide floor (up to `all`) carries no extra data risk: it stays read-only, inside the `/api/public/`
/// allowlist + loopback gate, and bounded by the discovery `MAX_PAGES` backstop. Following the import
/// range keeps `langfuse_discovered_environments` aligned with the history the user actually imports,
/// instead of the old fixed 7-day subset that silently hid older environments from the mapping picker.
fn discovery_window(range_floor: &str, now: &str) -> ImportWindow {
    ImportWindow {
        from: range_floor.to_string(),
        to: now.to_string(),
    }
}

/// Run one import against the real Langfuse REST API using its own SQLite connection.
///
/// Intended to be called from a dedicated OS thread (not a Tauri async context): the blocking
/// `reqwest` client must not run inside the Tauri runtime, and using a separate connection keeps
/// the network call off the UI's database lock. Returns a secret-free [`ImportReport`] on success,
/// or a secret-free error string on failure (including the in-band persist-failure surfacing).
pub fn run_blocking_import(db_path: &Path) -> Result<ImportReport, String> {
    run_blocking(db_path, ImportMode::Incremental)
}

/// Run one **backfill** against the real Langfuse REST API on this thread (TASK-029 C5). Identical
/// posture to [`run_blocking_import`] — settings-first config, off-UI thread, secret-free report,
/// in-band persist-failure surfacing — but re-scans the configured range floor → now in bounded,
/// atomically-committed, resumable chunks instead of a single incremental window.
pub fn run_blocking_backfill(db_path: &Path) -> Result<ImportReport, String> {
    run_blocking(db_path, ImportMode::Backfill)
}

/// Whether an import resumes from each environment's cursor (incremental) or re-scans the full
/// configured range in resumable chunks (backfill).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportMode {
    Incremental,
    Backfill,
}

fn run_blocking(db_path: &Path, mode: ImportMode) -> Result<ImportReport, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    store::migrate(&conn).map_err(|e| e.to_string())?;
    // Settings-first resolution (TASK-026): stored settings + Keychain credentials win; process env
    // is the marked dev fallback. The Keychain read is on this dedicated import thread, off the UI.
    // A genuine Keychain read failure propagates a coarse, secret-free error rather than silently
    // resolving to no/partial credentials.
    let config = crate::settings::resolve_config(&conn, &KeyringSecretStore::new())?;
    let api = ReqwestLangfuseApi::new(config.clone()).map_err(|e| e.message)?;

    // Resolve the configured import range (settings-first, default last_30d) into a UTC range floor
    // (TASK-029 C). A malformed stored value resolves to the default — the import never fails for it.
    let now_dt = Utc::now();
    let range = crate::settings::resolve_import_range(&conn);
    let range_floor = range.floor(now_dt);
    let now = now_dt.to_rfc3339_opts(SecondsFormat::Secs, true);

    let summaries = match mode {
        // Incremental: each environment resumes from its own persisted cursor (less the overlap),
        // floored at the configured range; a never-imported environment starts at the range floor.
        ImportMode::Incremental => {
            importer::run_import_with(&api, &conn, &config, &|_env, cursor| {
                importer::incremental_window(&range_floor, cursor, &now)
            })
        }
        // Backfill: re-scan the range floor → now in bounded, atomically-committed, resumable chunks.
        ImportMode::Backfill => importer::run_backfill(&api, &conn, &config, &range_floor, &now),
    };

    // TASK-027 C + TASK-045 B: discover the environments present in the source as part of the import
    // (a read-only, name-only scan over the same allowlist). Best-effort and secret-free — a discovery
    // failure must not fail an otherwise-successful import, and only environment names (not trace
    // content) are persisted. The look-back now follows the **resolved import-range floor** (the same
    // floor the import just used), not a fixed 7-day window, so an environment backfilled from older
    // history is enumerated and becomes mappable. Discovery stays bounded by its `MAX_PAGES` backstop,
    // so even an `all` floor cannot spin.
    discover_and_record(&api, &conn, &discovery_window(&range_floor, &now));
    // Build the secret-free diagnostics report from the counts the importer just computed, then
    // apply the TASK-021 in-band persist-failure check: a run that could not be persisted still
    // surfaces as `Err` (never a stale-healthy result), so the report is returned only for a run
    // that did persist.
    let report = ImportReport::from_summaries(&summaries);
    import_result(&summaries)?;
    Ok(report)
}

/// Run environment discovery and persist the distinct environment names additively (TASK-027 C3).
/// Best-effort: a transport/API failure or an individual persist failure is swallowed so it can
/// never fail the import that already ran. Only environment names + a `last_seen` timestamp are
/// written — no credential, trace content, or count (SEC-010).
fn discover_and_record(api: &dyn LangfuseApi, conn: &Connection, window: &ImportWindow) {
    if let Ok(environments) = discovery::discover_environments(api, window) {
        let seen_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        for environment in environments {
            let _ = store::upsert_discovered_environment(conn, &environment, &seen_at);
        }
    }
}

/// Collapse the per-environment summaries into the manual-import command's in-band result. A run that
/// could not be persisted carries the secret-free `PERSIST_FAILURE_MSG` sentinel in its warnings;
/// when present, surface it as an `Err` so the IPC returns a non-healthy result and never falls back
/// to a stale `healthy` snapshot (TASK-021 S-4). This is the fault-independent channel: when the
/// store is unwritable, no durable failure-marker can be recorded, so correctness must come from the
/// function return, not a database read. Keys on the exact sentinel — **not** `health == Unknown`,
/// which is also produced by a legitimately-persisted indeterminate classification.
fn import_result(summaries: &[importer::ImportSummary]) -> Result<(), String> {
    let persist_failed = summaries.iter().any(|s| {
        s.warnings
            .iter()
            .any(|w| w == importer::PERSIST_FAILURE_MSG)
    });
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
