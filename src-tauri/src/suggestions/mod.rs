//! AI time-entry suggestion engine + persistence (TASK-032 Workstream A).
//!
//! The engine reads imported `langfuse_ai_evidence`, associates each row to a Vire project at **read
//! time** via the env→project map (DEC-001 — no evidence row is ever rewritten), buckets evidence by
//! `(project, local date)`, clusters timed rows into contiguous time blocks, and persists *pending*
//! suggestions to the additive `time_entry_suggestions` table. It is a pure function over SQLite —
//! no network, no Langfuse egress (DEC-001/017).
//!
//! Three guarantees are load-bearing here and negative-tested in `tests.rs`:
//! - **absence ≠ zero** (DEC-004): an unknown duration / token total / cost stays `NULL`, never `0`; a
//!   bucket with no usable timestamps yields a "needs manual time" suggestion, never a 0-minute block;
//!   evidence in an unmapped environment is reported separately, never dropped and never zeroed.
//! - **no auto-posting** (DEC-006): generating suggestions writes only `time_entry_suggestions`, never
//!   `time_entries`. The accept path (Workstream B) is the sole writer of a time entry.
//! - **no secrets** (SEC-012): every surfaced field is an environment name, project reference, local
//!   time, aggregate number, count, or health/confidence label — never a raw session id, payload,
//!   prompt/metadata, or credential.

// Workstream A delivers the engine + persistence; the public surface (`generate`, the IPC DTOs) is
// wired into Tauri commands and the renderer in Workstreams B/C. Until then these items are exercised
// only by this module's unit tests, so the not-yet-consumed `dead_code`/`unused_imports` warnings are
// expected — suppressed here rather than prematurely adding IPC, which is out of A-scope.
#![allow(dead_code, unused_imports)]

mod engine;
pub mod store;

#[cfg(test)]
mod tests;

pub use engine::{current, generate};

use serde::{Deserialize, Serialize};

/// One persisted, renderer-facing suggestion. Carries only secret-free fields (SEC-012): a project
/// reference, local times, aggregate numbers, counts, and health/confidence labels. `Option` fields
/// are `None` (rendered "—" / "needs manual time") for genuinely unknown values — never coerced to 0.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Suggestion {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub date: String,
    pub block_start_ts: Option<String>,
    pub block_end_ts: Option<String>,
    pub duration_minutes: Option<i64>,
    pub trace_count: i64,
    pub session_count: i64,
    pub total_tokens: Option<i64>,
    pub cost_total: Option<f64>,
    pub cost_currency: Option<String>,
    pub health: String,
    pub confidence: String,
    pub source: String,
    pub reason: String,
    pub status: String,
    pub accepted_entry_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Evidence in an environment that maps to no Vire project — surfaced so the UI (Workstream C) can
/// prompt the user to map it. Never folded into a block and never zeroed (absence ≠ zero).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UnmappedEnv {
    pub environment: String,
    pub trace_count: i64,
}

/// The engine's output: the fresh `pending` set plus the unmapped-environment summary.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SuggestionList {
    pub suggestions: Vec<Suggestion>,
    pub unmapped: Vec<UnmappedEnv>,
}

/// Optional edits supplied on accept (Workstream B). Each field overrides the corresponding suggestion
/// value before the time entry is committed. For an untimed ("needs manual time") suggestion the
/// `start_time`/`end_time` overrides are *required* — accept never invents a duration (absence ≠ zero).
/// Validated by the same rules as a manual entry (`parse_duration`, length caps); carries no secret.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SuggestionEdit {
    pub project_id: Option<String>,
    pub date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub note: Option<String>,
}
