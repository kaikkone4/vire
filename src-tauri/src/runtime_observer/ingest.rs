//! Read-only ingest of the coarse session log. Path safety (reject symlinks, cap bytes, tolerate
//! absence), strict-allowlist parsing (the `RawEvent` type *is* the allowlist), and lifecycle
//! pairing into coarse sessions. No process scanning, no command-lines, no network.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::config::{RuntimeConfig, MAX_LOG_BYTES};
use super::model::{status, RawEvent, RuntimeSession};

/// Outcome of an ingest pass: the coarse sessions, plus whether a *usable* runtime log existed.
/// `log_present == false` means no runtime evidence — never a conclusion of zero AI usage/cost.
#[derive(Debug, Clone, Default)]
pub struct IngestResult {
    pub sessions: Vec<RuntimeSession>,
    pub log_present: bool,
}

/// Read and pair sessions from the configured log. A missing/empty/symlinked/oversize file yields
/// `log_present = false` and no sessions — refused safely, never crashing, never partial.
pub fn ingest(config: &RuntimeConfig) -> IngestResult {
    let Some(path) = &config.log_path else {
        return IngestResult::default();
    };
    let Some(contents) = read_safely(path) else {
        return IngestResult::default();
    };
    if contents.is_empty() {
        // Present but empty ⇒ no runtime evidence (spec: "absent or empty").
        return IngestResult::default();
    }
    IngestResult {
        sessions: pair_sessions(&contents),
        log_present: true,
    }
}

/// Read the file only when it is a regular file (not a symlink) within the byte cap. Any failure of
/// those guards returns `None` (no usable evidence) instead of crashing or reading partial/unsafe
/// data. Symlink rejection prevents a planted link from redirecting the read at an arbitrary file.
fn read_safely(path: &Path) -> Option<String> {
    let meta = fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() {
        return None;
    }
    if !meta.is_file() {
        return None;
    }
    if meta.len() > MAX_LOG_BYTES {
        return None;
    }
    // Lossy decode: a non-UTF-8 byte cannot become a typed allowlist value; it degrades to a
    // replacement char in a field we never persist, and malformed lines are skipped below.
    fs::read(path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// A session under construction. Fields are only ever overwritten with a *present* value, so a
/// later event missing `project_key`/`tool` never erases what an earlier event established.
#[derive(Default)]
struct Builder {
    project_key: Option<String>,
    tool: Option<String>,
    session_id: Option<String>,
    started_at: Option<String>,
    ended_at: Option<String>,
    status: Option<String>,
}

/// Lifecycle pairing mirrors pi-observe's own model. `tool_started`/`manual_active` open a session;
/// `tool_finished`/`manual_inactive` close it with a coarse status; `tool_orphaned` closes it as
/// `open` (started, never cleanly finished). Unknown event types (e.g. `idle_*`) are ignored.
/// Malformed JSON lines are skipped, never fatal.
fn pair_sessions(contents: &str) -> Vec<RuntimeSession> {
    // Preserve first-seen order for stable, deterministic output.
    let mut order: Vec<String> = Vec::new();
    let mut builders: HashMap<String, Builder> = HashMap::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(ev) = serde_json::from_str::<RawEvent>(line) else {
            continue; // malformed line — skip, never fatal
        };
        let Some(key) = ev.session_key() else {
            continue; // no run_id/session_id — cannot be keyed
        };
        let event = ev.event.as_deref().unwrap_or("").trim().to_string();
        if !is_known_event(&event) {
            continue;
        }

        if !builders.contains_key(&key) {
            order.push(key.clone());
        }
        let b = builders.entry(key).or_default();

        // Allowlisted coarse fields only — set when present, never clear what was set.
        if let Some(p) = ev.project() {
            b.project_key = Some(p);
        }
        if let Some(t) = nonempty(&ev.tool) {
            b.tool = Some(t);
        }
        if let Some(s) = nonempty(&ev.session_id) {
            b.session_id = Some(s);
        }

        match event.as_str() {
            "tool_started" | "manual_active" => {
                if let Some(ts) = nonempty(&ev.ts) {
                    b.started_at = Some(ts);
                }
                b.status.get_or_insert_with(|| status::OPEN.to_string());
            }
            "tool_finished" | "manual_inactive" => {
                if let Some(ts) = nonempty(&ev.ts) {
                    b.ended_at = Some(ts);
                }
                b.status = Some(coarse_status(&ev));
            }
            "tool_orphaned" => {
                if let Some(ts) = nonempty(&ev.ts) {
                    b.ended_at = Some(ts);
                }
                // Orphaned = started but never cleanly closed: coarse `open`, not success/failure.
                b.status = Some(status::OPEN.to_string());
            }
            _ => {}
        }
    }

    order
        .into_iter()
        .filter_map(|key| builders.remove(&key).map(|b| finish(key, b)))
        .collect()
}

fn finish(session_key: String, b: Builder) -> RuntimeSession {
    RuntimeSession {
        session_key,
        project_key: b.project_key.unwrap_or_default(),
        tool: b.tool,
        session_id: b.session_id,
        started_at: b.started_at,
        ended_at: b.ended_at,
        status: b.status.unwrap_or_else(|| status::OPEN.to_string()),
    }
}

fn is_known_event(event: &str) -> bool {
    matches!(
        event,
        "tool_started" | "manual_active" | "tool_finished" | "manual_inactive" | "tool_orphaned"
    )
}

/// Map an emitter status / exit code to the coarse four-value label. Exit codes are read only to
/// derive success/failure and are never persisted as numbers.
fn coarse_status(ev: &RawEvent) -> String {
    if let Some(s) = nonempty(&ev.status) {
        let s = s.to_ascii_lowercase();
        if s.contains("success")
            || s.contains("ok")
            || s.contains("complete")
            || s.contains("finish")
        {
            return status::SUCCESS.to_string();
        }
        if s.contains("fail") || s.contains("error") {
            return status::FAILED.to_string();
        }
        if s.contains("cancel") {
            return status::CANCELED.to_string();
        }
    }
    match ev.exit_code {
        Some(0) => status::SUCCESS.to_string(),
        Some(_) => status::FAILED.to_string(),
        None => status::OPEN.to_string(),
    }
}

fn nonempty(v: &Option<String>) -> Option<String> {
    v.as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
