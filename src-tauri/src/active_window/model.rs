//! Domain types for the Active-Window Evidence Store (TASK-046).
//!
//! Privacy invariant: `RawObservationIn` declares ONLY the allowlisted coarse fields. serde
//! silently drops every other key, so a prohibited value (prompt, command, URL, secret, etc.)
//! can never reach a typed value, the store, or a log. The allowlist is the type, not a runtime
//! filter that could be bypassed.

use serde::{Deserialize, Serialize};

/// Redaction policy for `window_title`. Default is `Redacted`: no raw title is ever written to
/// disk. Only an explicit `Stored` opt-in allows a title to be persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleMode {
    /// Default: raw title is discarded at the write boundary; `title_state` records `'redacted'`.
    Redacted,
    /// Explicit opt-in: raw title is persisted with `title_state = 'captured'`.
    Stored,
}

/// Controlled vocabulary for the `title_state` column.
pub mod title_state {
    /// AX title was observed and persisted — only under `title_mode='stored'`.
    pub const CAPTURED: &str = "captured";
    /// A title was observed but deliberately not stored — the default.
    pub const REDACTED: &str = "redacted";
    /// Accessibility permission not granted.
    pub const ABSENT_NO_PERMISSION: &str = "absent_no_permission";
    /// No focused window at sample time.
    pub const ABSENT_NO_WINDOW: &str = "absent_no_window";
    /// App not AX-scriptable or attribute unsupported.
    pub const ABSENT_UNSUPPORTED: &str = "absent_unsupported";
    /// Title attribute present but empty string.
    pub const EMPTY: &str = "empty";
}

/// Controlled vocabulary for the `idle_state` column.
pub mod idle_state {
    pub const ACTIVE: &str = "active";
    pub const IDLE_CANDIDATE: &str = "idle_candidate";
    pub const AWAY: &str = "away";
}

/// Controlled vocabulary for the `source` column.
pub mod source {
    pub const NSWORKSPACE: &str = "nsworkspace";
    pub const ACCESSIBILITY: &str = "accessibility";
    /// Quartz: off the default path (C2); vocabulary admits it but capture won't emit it by default.
    pub const QUARTZ: &str = "quartz";
}

/// Controlled vocabulary for `active_window_capture_health.state` (TASK-002 §4).
pub mod health_state {
    pub const AX_PERMISSION_DENIED: &str = "ax_permission_denied";
    pub const SCREEN_RECORDING_ABSENT: &str = "screen_recording_absent";
    pub const NO_FOCUSED_WINDOW: &str = "no_focused_window";
    pub const WINDOW_UNAVAILABLE: &str = "window_unavailable";
    pub const TITLE_EMPTY: &str = "title_empty";
    pub const SAMPLING_GAP: &str = "sampling_gap";
    pub const SYSTEM_SLEEP: &str = "system_sleep";
    pub const SYSTEM_WAKE: &str = "system_wake";
    pub const SCREEN_LOCKED: &str = "screen_locked";
    pub const HELPER_CRASH: &str = "helper_crash";
    pub const HELPER_RESTART: &str = "helper_restart";
    pub const NO_GUI_SESSION: &str = "no_gui_session";
}

/// Strict ingest DTO for the future capture IPC payload. serde DROPS every non-allowlisted key,
/// exactly like `runtime_observer::model::RawEvent`. No prohibited field is declared so none can
/// be deserialized into a typed value or reach the store.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawObservationIn {
    #[serde(default)]
    pub sample_ts: Option<String>,
    #[serde(default)]
    pub app_name: Option<String>,
    #[serde(default)]
    pub app_bundle_id: Option<String>,
    /// Populated only under `title_mode='stored'`; write API nulls it under `'redacted'`.
    #[serde(default)]
    pub window_title: Option<String>,
    #[serde(default)]
    pub title_state: Option<String>,
    #[serde(default)]
    pub idle_state: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub capture_health: Option<String>,
}

/// Typed, allowlist-only raw observation ready to persist.
#[derive(Debug, Clone)]
pub struct RawObservation {
    pub sample_ts: String,
    pub day: String,
    pub app_name: Option<String>,
    pub app_bundle_id: Option<String>,
    /// Pre-gate: caller supplies the observed title; write API applies redaction.
    pub window_title: Option<String>,
    pub title_state: String,
    pub idle_state: String,
    pub source: String,
    pub capture_health: Option<String>,
}

/// Coalesced contiguous run of one app/window/state — the reviewable unit.
#[derive(Debug, Clone)]
pub struct EvidenceBlock {
    pub id: String,
    pub day: String,
    pub start_ts: String,
    pub end_ts: String,
    pub duration_seconds: i64,
    pub app_name: Option<String>,
    pub app_bundle_id: Option<String>,
    pub window_title: Option<String>,
    pub title_state: String,
    pub idle_state: String,
    pub source: String,
    pub capture_health: Option<String>,
}

/// A degraded capture state row. `detail` holds only a bounded coarse reason code — never a
/// title, path, command, or secret.
#[derive(Debug, Clone)]
pub struct CaptureHealthEvent {
    pub day: String,
    pub start_ts: String,
    pub end_ts: Option<String>,
    pub state: String,
    /// Bounded coarse reason code only (e.g. an AXError code). Never a title or secret.
    pub detail: Option<String>,
    pub source: String,
}

/// Read projection for normalized evidence blocks. Under default `Redacted` mode,
/// `window_title` is always `None` — the caller never receives a raw title.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceBlockView {
    pub id: String,
    pub day: String,
    pub start_ts: String,
    pub end_ts: String,
    pub duration_seconds: i64,
    pub app_name: Option<String>,
    pub app_bundle_id: Option<String>,
    /// `None` under `Redacted` mode; populated only under `Stored`.
    pub window_title: Option<String>,
    pub title_state: String,
    pub idle_state: String,
    pub source: String,
    pub capture_health: Option<String>,
    pub review_state: String,
}

/// Per-table deleted-row counts returned by `prune_expired`.
#[derive(Debug, Default)]
pub struct PruneStats {
    pub raw_evidence_deleted: usize,
    pub evidence_deleted: usize,
    pub capture_health_deleted: usize,
}

/// A coarse capture-health state for the status readout (TASK-056 B). Carries only the controlled
/// `state` code, its start/since timestamp, and the already-bounded coarse `detail` reason code —
/// never a title, path, URL, command, or secret (structurally absent upstream).
#[derive(Debug, Clone, Serialize)]
pub struct HealthMarker {
    pub state: String,
    pub since_or_start_ts: String,
    pub detail: Option<String>,
}

/// Read-only capture status/health projection over the three existing `active_window_*` tables
/// (TASK-056 B). All fields are aggregates or bounded coarse state/detail codes from allowlisted
/// columns; no schema change and no write path produce it, and it can carry no prohibited value.
#[derive(Debug, Clone, Serialize)]
pub struct CaptureStatusView {
    /// `MAX(sample_ts)` over raw evidence; `None` when nothing has been captured yet.
    pub last_sample_ts: Option<String>,
    /// Raw observations recorded for the current local day.
    pub samples_today: i64,
    /// Normalized evidence blocks whose `day` is within the retention window.
    pub evidence_blocks_retained: i64,
    /// Currently-open (`end_ts IS NULL`) degraded-capture markers — ongoing gaps.
    pub open_health: Vec<HealthMarker>,
    /// Most-recent capture-health markers (open or closed), newest first.
    pub recent_health: Vec<HealthMarker>,
}
