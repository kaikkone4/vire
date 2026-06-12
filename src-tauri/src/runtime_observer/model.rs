//! Domain types for the AI Runtime Observer (TASK-022). Coarse session boundaries, the
//! reconciliation-state taxonomy TASK-022 *owns*, and the strict ingest allowlist DTO.
//!
//! Privacy invariant: `RawEvent` declares **only** the allowlisted coarse fields. serde silently
//! drops every other key on deserialization, so a prompt / command body / shell-history line /
//! env dump / secret present in a hostile log line can never reach a typed value, the store, or a
//! log. The allowlist is the type, not a runtime filter that could be bypassed.

use serde::{Deserialize, Serialize};

/// Coarse terminal status of an observed runtime session. No exit codes, durations, or command
/// bodies — only this four-value label is persisted.
pub mod status {
    pub const SUCCESS: &str = "success";
    pub const FAILED: &str = "failed";
    pub const CANCELED: &str = "canceled";
    /// Still open at end of log, or orphaned (started, never cleanly closed).
    pub const OPEN: &str = "open";
}

/// The TASK-022-owned reconciliation state. Trace-side health states
/// (`stale`/`wrong_env`/`delayed`/…) are *referenced* from the importer, never re-derived here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationState {
    /// Session aligns with ≥1 imported trace (session_id or env+time overlap).
    Matched,
    /// Session observed AND its window+env import was `healthy`, but no matching trace — the
    /// DEC-017/FR-012 gap signal.
    ObservedNoTrace,
    /// Session cannot be evaluated against any environment (no project→environment mapping).
    UnmatchedRuntime,
    /// Import covering the window+env was down/uncertain, or no runtime log — absence ≠ zero.
    ReconciliationUnknown,
    /// An imported trace with no runtime session (counted per evidence row, never stored as a
    /// session row).
    UnmatchedTrace,
}

impl ReconciliationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReconciliationState::Matched => "matched",
            ReconciliationState::ObservedNoTrace => "observed_no_trace",
            ReconciliationState::UnmatchedRuntime => "unmatched_runtime",
            ReconciliationState::ReconciliationUnknown => "reconciliation_unknown",
            ReconciliationState::UnmatchedTrace => "unmatched_trace",
        }
    }
}

/// One line of the coarse session log, parsed through the strict allowlist. Every field is
/// `Option` and non-allowlisted keys are dropped by serde. No `metadata`, `summary`,
/// `command_label`, `git_*`, `cwd_*`, prompt, response, or secret field is declared — so none can
/// be deserialized.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawEvent {
    /// Lifecycle event type (`tool_started`/`tool_finished`/`manual_active`/…).
    #[serde(default)]
    pub event: Option<String>,
    /// Safe, already-tokenized project label. Either `project_key` or `project` may carry it.
    #[serde(default)]
    pub project_key: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    /// Coarse tool label only (e.g. "claude", "pi").
    #[serde(default)]
    pub tool: Option<String>,
    /// Stable run id; primary dedup key.
    #[serde(default)]
    pub run_id: Option<String>,
    /// Hashed `session-<hash>` id (opaque, not content).
    #[serde(default)]
    pub session_id: Option<String>,
    /// ISO-8601 timestamp of this event.
    #[serde(default)]
    pub ts: Option<String>,
    /// Coarse status label if the emitter supplies one.
    #[serde(default)]
    pub status: Option<String>,
    /// Process exit code → folded to a coarse status; never persisted as a number.
    #[serde(default)]
    pub exit_code: Option<i64>,
    /// Allowlisted coarse flags, parsed for fidelity but intentionally NOT persisted (the store
    /// carries no duration/cost ledger — DEC-003/DEC-017).
    #[allow(dead_code)]
    #[serde(default)]
    pub billable: Option<bool>,
    #[allow(dead_code)]
    #[serde(default)]
    pub duration_ms: Option<i64>,
}

impl RawEvent {
    /// The safe project token, preferring the explicit `project_key`.
    pub fn project(&self) -> Option<String> {
        self.project_key
            .clone()
            .or_else(|| self.project.clone())
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
    }

    /// The dedup unit: the stable `run_id`, else the hashed `session_id`. `None` when a line
    /// carries neither (it cannot be keyed and is skipped).
    pub fn session_key(&self) -> Option<String> {
        nonempty(&self.run_id).or_else(|| nonempty(&self.session_id))
    }
}

fn nonempty(v: &Option<String>) -> Option<String> {
    v.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// A coarse runtime session after lifecycle pairing — boundaries only, no content.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeSession {
    pub session_key: String,
    pub project_key: String,
    pub tool: Option<String>,
    pub session_id: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub status: String,
}

/// A reconciled session ready to persist: the coarse session plus the resolved environment, the
/// TASK-022 reconciliation state, and the matched trace id (if any).
#[derive(Debug, Clone, PartialEq)]
pub struct ReconciledSession {
    pub session: RuntimeSession,
    pub environment: Option<String>,
    pub reconciliation: ReconciliationState,
    pub matched_trace_id: Option<String>,
}

/// A read-only projection of `langfuse_ai_evidence` used for matching — the normalized row, never
/// the prompt-bearing raw payload. No token/cost field is read.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceRef {
    pub trace_id: String,
    pub environment: String,
    pub session_id: Option<String>,
    pub ai_start_ts: Option<String>,
    pub ai_end_ts: Option<String>,
}

/// Coarse reconciliation snapshot returned by the read-only IPC. Counts and posture only — no
/// secrets, session content, command text, or raw log line.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeReconciliationSnapshot {
    /// Whether a usable (present, safe, non-empty) runtime session log was found.
    pub runtime_log_present: bool,
    /// Source posture — always local; the observer makes no network call.
    pub source: String,
    pub observed_runs: u32,
    pub matched: u32,
    pub observed_no_trace: u32,
    pub unmatched_runtime: u32,
    pub reconciliation_unknown: u32,
    pub unmatched_trace: u32,
    pub message: String,
}
