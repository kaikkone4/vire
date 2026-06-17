//! Observer tests: strict-allowlist drop (adversarial), path safety, idempotent ingest, matcher
//! precedence, import-health gating, absence-≠-zero, and the no-content-leak IPC surface. No
//! network, no process scanning — fixtures are local `events.jsonl` files and a seeded importer DB.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use rusqlite::Connection;
use tempfile::tempdir;

use super::config::{RuntimeConfig, MAX_LOG_BYTES};
use super::ingest::ingest;
use super::model::{status as st, ReconciliationState};
use super::reconcile::reconcile_sessions;
use super::{observe_with_config, store};
use crate::langfuse::model::{AiEvidence, HealthState};
use crate::langfuse::store::{insert_import_run, ImportRunRecord};

// ----- helpers -----------------------------------------------------------------------------

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    crate::init_db(&c).unwrap();
    c
}

fn config_for(path: PathBuf) -> RuntimeConfig {
    RuntimeConfig {
        log_path: Some(path),
        env_map: HashMap::new(),
        allowed_environments: vec!["vire".into()],
        slop_secs: 300,
    }
}

/// Write a `events.jsonl` fixture into a fresh temp dir and return (config, keep-alive dir).
fn log_with(lines: &[&str]) -> (RuntimeConfig, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(lines.join("\n").as_bytes()).unwrap();
    (config_for(path), dir)
}

fn seed_evidence(
    c: &Connection,
    trace_id: &str,
    env: &str,
    session_id: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
) {
    let ev = AiEvidence {
        trace_id: trace_id.into(),
        environment: env.into(),
        session_id: session_id.map(Into::into),
        ai_start_ts: start.map(Into::into),
        ai_end_ts: end.map(Into::into),
        prompt_tokens: None,
        completion_tokens: None,
        total_tokens: None,
        cost_total: None,
        health: HealthState::Healthy,
    };
    crate::langfuse::store::upsert_ai_evidence(c, &ev, "run-seed").unwrap();
}

fn seed_import_run(c: &Connection, id: &str, env: &str, status: HealthState, from: &str, to: &str) {
    let run = ImportRunRecord {
        id: id.into(),
        environment: env.into(),
        window_from: Some(from.into()),
        window_to: Some(to.into()),
        cursor_ts: None,
        status,
        pages_walked: 0,
        traces_seen: 0,
        duplicates_suppressed: 0,
        warnings: vec![],
        started_at: "2026-06-05T00:00:00Z".into(),
        finished_at: "2026-06-05T23:59:59Z".into(),
    };
    insert_import_run(c, &run).unwrap();
}

/// Concatenate every stored column of every session row — the surface an adversarial test scans to
/// prove no prohibited value reached the store.
fn dump_sessions(c: &Connection) -> String {
    let mut stmt = c
        .prepare(
            "SELECT session_key||'|'||project_key||'|'||COALESCE(tool,'')||'|'||\
             COALESCE(session_id,'')||'|'||COALESCE(environment,'')||'|'||COALESCE(started_at,'')||'|'||\
             COALESCE(ended_at,'')||'|'||COALESCE(status,'')||'|'||reconciliation||'|'||\
             COALESCE(matched_trace_id,'')||'|'||observed_at FROM ai_runtime_sessions",
        )
        .unwrap();
    let rows: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    rows.join("\n")
}

fn session_count(c: &Connection) -> i64 {
    c.query_row("SELECT COUNT(*) FROM ai_runtime_sessions", [], |r| r.get(0))
        .unwrap()
}

fn started(run_id: &str, project: &str, session_id: &str, ts: &str, tool: &str) -> String {
    format!(
        r#"{{"event":"tool_started","run_id":"{run_id}","project":"{project}","session_id":"{session_id}","tool":"{tool}","ts":"{ts}"}}"#
    )
}

fn finished(run_id: &str, ts: &str, exit: i64) -> String {
    format!(r#"{{"event":"tool_finished","run_id":"{run_id}","ts":"{ts}","exit_code":{exit}}}"#)
}

// ----- strict allowlist (adversarial) ------------------------------------------------------

#[test]
fn allowlist_drops_injected_prohibited_fields_and_never_persists_them() {
    // A hostile line stuffing prompts, command bodies, shell history, env dumps, secrets, and
    // prohibited metadata alongside the allowlisted coarse fields.
    let hostile = r#"{"event":"tool_started","run_id":"r1","project":"vire","session_id":"session-abc","tool":"claude","ts":"2026-06-05T10:00:00Z","prompt":"SECRET_PROMPT_payload","command_body":"rm -rf / SECRET_CMD","shell_history":"export K=SECRET","env":{"AWS_SECRET_ACCESS_KEY":"AKIASECRETKEY000"},"metadata":{"summary":"SECRET_SUMMARY","command_label":"SECRET_LABEL","git_branch":"SECRET_BRANCH","git_remote_hash":"SECRETHASH","cwd_basename":"SECRETCWD"},"api_key":"sk-ant-SECRETTOKEN"}"#;
    let finish = r#"{"event":"tool_finished","run_id":"r1","ts":"2026-06-05T10:05:00Z","exit_code":0,"response":"SECRET_RESPONSE_text"}"#;
    let (config, _dir) = log_with(&[hostile, finish]);
    let c = conn();

    observe_with_config(&c, &config).unwrap();

    let dump = dump_sessions(&c);
    // Allowlisted coarse fields survived.
    assert!(dump.contains("vire"), "safe project token kept");
    assert!(dump.contains("claude"), "coarse tool label kept");
    assert!(
        dump.contains("session-abc"),
        "opaque hashed session id kept"
    );
    assert!(
        dump.contains(st::SUCCESS),
        "coarse status derived from exit_code"
    );
    // No prohibited value reached the store, in ANY column.
    for needle in [
        "SECRET",
        "AKIA",
        "sk-ant",
        "rm -rf",
        "export K",
        "git_branch",
        "summary",
        "command_body",
        "AWS_SECRET",
        "cwd",
    ] {
        assert!(
            !dump.contains(needle),
            "prohibited value `{needle}` must never be persisted; dump=`{dump}`"
        );
    }
}

#[test]
fn malformed_lines_are_skipped_not_fatal() {
    let (config, _dir) = log_with(&[
        "{ this is not json",
        &started("r1", "vire", "session-x", "2026-06-05T10:00:00Z", "claude"),
        "garbage \x00 line",
        &finished("r1", "2026-06-05T10:01:00Z", 0),
    ]);
    let result = ingest(&config);
    assert!(result.log_present);
    assert_eq!(
        result.sessions.len(),
        1,
        "the one valid session is still paired"
    );
    assert_eq!(result.sessions[0].status, st::SUCCESS);
}

// ----- path safety -------------------------------------------------------------------------

#[test]
fn absent_log_is_no_evidence_not_zero() {
    let dir = tempdir().unwrap();
    let config = config_for(dir.path().join("does-not-exist.jsonl"));
    let result = ingest(&config);
    assert!(!result.log_present, "an absent log is no runtime evidence");
    assert!(result.sessions.is_empty());
}

#[test]
fn empty_log_is_no_evidence() {
    let (config, _dir) = log_with(&[]);
    let result = ingest(&config);
    assert!(!result.log_present, "an empty log is no runtime evidence");
}

#[test]
fn symlinked_log_is_refused() {
    let dir = tempdir().unwrap();
    let real = dir.path().join("real.jsonl");
    std::fs::write(
        &real,
        started("r1", "vire", "s", "2026-06-05T10:00:00Z", "claude"),
    )
    .unwrap();
    let link = dir.path().join("link.jsonl");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let config = config_for(link);
    let result = ingest(&config);
    assert!(!result.log_present, "a symlinked log path is refused");
    assert!(result.sessions.is_empty());
}

#[test]
fn oversize_log_is_refused() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("big.jsonl");
    let big = vec![b'x'; (MAX_LOG_BYTES + 1) as usize];
    std::fs::write(&path, big).unwrap();
    let config = config_for(path);
    let result = ingest(&config);
    assert!(
        !result.log_present,
        "a file over the byte cap is refused, never partially read"
    );
    assert!(result.sessions.is_empty());
}

// ----- session pairing ---------------------------------------------------------------------

#[test]
fn pairs_start_and_finish_into_one_session() {
    let (config, _dir) = log_with(&[
        &started("r1", "vire", "session-1", "2026-06-05T10:00:00Z", "claude"),
        &finished("r1", "2026-06-05T10:30:00Z", 0),
    ]);
    let s = &ingest(&config).sessions[0];
    assert_eq!(s.session_key, "r1");
    assert_eq!(s.project_key, "vire");
    assert_eq!(s.tool.as_deref(), Some("claude"));
    assert_eq!(s.started_at.as_deref(), Some("2026-06-05T10:00:00Z"));
    assert_eq!(s.ended_at.as_deref(), Some("2026-06-05T10:30:00Z"));
    assert_eq!(s.status, st::SUCCESS);
}

#[test]
fn nonzero_exit_pairs_to_failed_and_orphan_to_open() {
    let (config, _dir) = log_with(&[
        &started("r1", "vire", "s1", "2026-06-05T10:00:00Z", "claude"),
        &finished("r1", "2026-06-05T10:01:00Z", 1),
        &started("r2", "vire", "s2", "2026-06-05T11:00:00Z", "pi"),
        r#"{"event":"tool_orphaned","run_id":"r2","ts":"2026-06-05T12:00:00Z"}"#,
    ]);
    let sessions = ingest(&config).sessions;
    let r1 = sessions.iter().find(|s| s.session_key == "r1").unwrap();
    let r2 = sessions.iter().find(|s| s.session_key == "r2").unwrap();
    assert_eq!(r1.status, st::FAILED, "non-zero exit ⇒ failed");
    assert_eq!(r2.status, st::OPEN, "orphaned ⇒ open");
}

#[test]
fn idempotent_reingest_does_not_duplicate_rows() {
    let (config, _dir) = log_with(&[
        &started("r1", "vire", "session-1", "2026-06-05T10:00:00Z", "claude"),
        &finished("r1", "2026-06-05T10:30:00Z", 0),
    ]);
    let c = conn();
    observe_with_config(&c, &config).unwrap();
    observe_with_config(&c, &config).unwrap();
    assert_eq!(
        session_count(&c),
        1,
        "re-ingesting the same log re-resolves in place, no dup rows"
    );
}

// ----- matcher precedence ------------------------------------------------------------------

#[test]
fn matches_by_session_id_exact_even_cross_environment() {
    let c = conn();
    // Evidence carries the session id but sits in a different environment.
    seed_evidence(
        &c,
        "T1",
        "other",
        Some("session-zzz"),
        Some("2026-06-05T10:00:00Z"),
        Some("2026-06-05T10:30:00Z"),
    );
    let (config, _dir) = log_with(&[
        &started(
            "r1",
            "vire",
            "session-zzz",
            "2026-06-05T10:00:00Z",
            "claude",
        ),
        &finished("r1", "2026-06-05T10:30:00Z", 0),
    ]);
    observe_with_config(&c, &config).unwrap();
    let dump = dump_sessions(&c);
    assert!(
        dump.contains("matched"),
        "session_id match resolves to matched"
    );
    assert!(dump.contains("T1"), "the matched trace id is recorded");
}

#[test]
fn matches_by_env_and_time_overlap_when_no_session_id() {
    let c = conn();
    seed_evidence(
        &c,
        "T2",
        "vire",
        None,
        Some("2026-06-05T10:05:00Z"),
        Some("2026-06-05T10:20:00Z"),
    );
    // Session has no session_id; project maps to env `vire`; windows overlap.
    let line_start = r#"{"event":"tool_started","run_id":"r9","project":"vire","tool":"claude","ts":"2026-06-05T10:00:00Z"}"#;
    let line_finish = finished("r9", "2026-06-05T10:30:00Z", 0);
    let (config, _dir) = log_with(&[line_start, &line_finish]);
    let evidence = store::evidence_refs(&c).unwrap();
    let runs = store::import_runs(&c).unwrap();
    let sessions = ingest(&config).sessions;
    let reconciled = reconcile_sessions(&sessions, &evidence, &runs, &config);
    assert_eq!(reconciled[0].reconciliation, ReconciliationState::Matched);
    assert_eq!(reconciled[0].matched_trace_id.as_deref(), Some("T2"));
}

#[test]
fn session_id_takes_precedence_over_time_overlap() {
    let c = conn();
    // Trace A overlaps in time; trace B shares the session id. session_id must win.
    seed_evidence(
        &c,
        "A",
        "vire",
        None,
        Some("2026-06-05T10:00:00Z"),
        Some("2026-06-05T10:30:00Z"),
    );
    seed_evidence(
        &c,
        "B",
        "vire",
        Some("session-key9"),
        Some("2026-06-01T00:00:00Z"),
        Some("2026-06-01T00:10:00Z"),
    );
    let (config, _dir) = log_with(&[
        &started(
            "r1",
            "vire",
            "session-key9",
            "2026-06-05T10:00:00Z",
            "claude",
        ),
        &finished("r1", "2026-06-05T10:30:00Z", 0),
    ]);
    let evidence = store::evidence_refs(&c).unwrap();
    let runs = store::import_runs(&c).unwrap();
    let sessions = ingest(&config).sessions;
    let reconciled = reconcile_sessions(&sessions, &evidence, &runs, &config);
    assert_eq!(
        reconciled[0].matched_trace_id.as_deref(),
        Some("B"),
        "session_id match precedes time"
    );
}

#[test]
fn unmappable_project_is_unmatched_runtime() {
    let c = conn();
    let (config, _dir) = log_with(&[
        // project `mystery` is not an allowed environment and has no session_id match.
        r#"{"event":"tool_started","run_id":"r1","project":"mystery","tool":"claude","ts":"2026-06-05T10:00:00Z"}"#,
        &finished("r1", "2026-06-05T10:05:00Z", 0),
    ]);
    let snap = observe_with_config(&c, &config).unwrap();
    assert_eq!(
        snap.unmatched_runtime, 1,
        "no env mapping ⇒ unmatched_runtime"
    );
    assert_eq!(snap.observed_no_trace, 0);
}

// ----- import-health gating (load-bearing) -------------------------------------------------

#[test]
fn observed_no_trace_only_under_a_healthy_import() {
    let c = conn();
    seed_import_run(
        &c,
        "ir-healthy",
        "vire",
        HealthState::Healthy,
        "2026-06-01T00:00:00Z",
        "2026-06-08T00:00:00Z",
    );
    let (config, _dir) = log_with(&[
        &started(
            "r1",
            "vire",
            "session-nogap",
            "2026-06-05T10:00:00Z",
            "claude",
        ),
        &finished("r1", "2026-06-05T10:30:00Z", 0),
    ]);
    let snap = observe_with_config(&c, &config).unwrap();
    assert_eq!(
        snap.observed_no_trace, 1,
        "healthy import + no trace ⇒ real gap"
    );
    assert_eq!(snap.reconciliation_unknown, 0);
}

#[test]
fn down_or_uncertain_import_is_reconciliation_unknown_never_a_gap() {
    for down in [
        HealthState::Unavailable,
        HealthState::Unknown,
        HealthState::AuthOrNetworkError,
    ] {
        let c = conn();
        seed_import_run(
            &c,
            "ir",
            "vire",
            down,
            "2026-06-01T00:00:00Z",
            "2026-06-08T00:00:00Z",
        );
        let (config, _dir) = log_with(&[
            &started("r1", "vire", "session-x", "2026-06-05T10:00:00Z", "claude"),
            &finished("r1", "2026-06-05T10:30:00Z", 0),
        ]);
        let snap = observe_with_config(&c, &config).unwrap();
        assert_eq!(snap.reconciliation_unknown, 1, "{down:?} ⇒ unknown");
        assert_eq!(
            snap.observed_no_trace, 0,
            "{down:?} must never assert a gap"
        );
    }
}

#[test]
fn non_healthy_import_does_not_license_a_gap() {
    // stale / missing are not clean positives; a missing trace under them stays unknown.
    for state in [
        HealthState::Stale,
        HealthState::Missing,
        HealthState::Duplicate,
    ] {
        let c = conn();
        seed_import_run(
            &c,
            "ir",
            "vire",
            state,
            "2026-06-01T00:00:00Z",
            "2026-06-08T00:00:00Z",
        );
        let (config, _dir) = log_with(&[
            &started("r1", "vire", "session-x", "2026-06-05T10:00:00Z", "claude"),
            &finished("r1", "2026-06-05T10:30:00Z", 0),
        ]);
        let snap = observe_with_config(&c, &config).unwrap();
        assert_eq!(
            snap.observed_no_trace, 0,
            "{state:?} is not a clean positive ⇒ no gap asserted"
        );
        assert_eq!(snap.reconciliation_unknown, 1);
    }
}

#[test]
fn no_import_run_for_env_is_unknown_not_a_gap() {
    let c = conn();
    // No import run seeded at all.
    let (config, _dir) = log_with(&[
        &started("r1", "vire", "session-x", "2026-06-05T10:00:00Z", "claude"),
        &finished("r1", "2026-06-05T10:30:00Z", 0),
    ]);
    let snap = observe_with_config(&c, &config).unwrap();
    assert_eq!(
        snap.observed_no_trace, 0,
        "no covering import ⇒ cannot assert a gap"
    );
    assert_eq!(snap.reconciliation_unknown, 1);
}

// ----- unmatched_trace + absence ≠ zero ----------------------------------------------------

#[test]
fn trace_with_no_runtime_session_is_unmatched_trace() {
    let c = conn();
    seed_evidence(
        &c,
        "MATCHED",
        "vire",
        Some("session-1"),
        Some("2026-06-05T10:00:00Z"),
        Some("2026-06-05T10:30:00Z"),
    );
    seed_evidence(
        &c,
        "ORPHANTRACE",
        "vire",
        Some("session-none"),
        Some("2026-06-05T11:00:00Z"),
        Some("2026-06-05T11:30:00Z"),
    );
    let (config, _dir) = log_with(&[
        &started("r1", "vire", "session-1", "2026-06-05T10:00:00Z", "claude"),
        &finished("r1", "2026-06-05T10:30:00Z", 0),
    ]);
    let snap = observe_with_config(&c, &config).unwrap();
    assert_eq!(snap.matched, 1);
    assert_eq!(
        snap.unmatched_trace, 1,
        "the trace with no session is surfaced, not read as zero"
    );
}

#[test]
fn absent_log_with_evidence_yields_unknown_message_never_zero() {
    let c = conn();
    seed_evidence(
        &c,
        "T",
        "vire",
        Some("session-1"),
        Some("2026-06-05T10:00:00Z"),
        Some("2026-06-05T10:30:00Z"),
    );
    let dir = tempdir().unwrap();
    let config = config_for(dir.path().join("missing.jsonl"));
    let snap = observe_with_config(&c, &config).unwrap();
    assert!(!snap.runtime_log_present);
    assert_eq!(snap.observed_runs, 0);
    assert_eq!(
        snap.unmatched_trace, 1,
        "an unmatched trace is surfaced, never zeroed"
    );
    assert!(snap.message.contains("unknown"));
    assert!(snap.message.to_lowercase().contains("never zero"));
}

// ----- structural: no cost/time authority --------------------------------------------------

#[test]
fn runtime_sessions_table_has_no_cost_token_or_duration_column() {
    let c = conn();
    let mut stmt = c.prepare("PRAGMA table_info(ai_runtime_sessions)").unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for col in &cols {
        let lc = col.to_ascii_lowercase();
        for banned in [
            "cost", "token", "duration", "price", "prompt", "command", "secret",
        ] {
            assert!(
                !lc.contains(banned),
                "ai_runtime_sessions must carry no `{banned}`-bearing column (found `{col}`)"
            );
        }
    }
}

// ----- IPC surface: counts only, no content ------------------------------------------------

#[test]
fn snapshot_serializes_to_counts_only_with_no_secret_or_content() {
    let hostile = r#"{"event":"tool_started","run_id":"r1","project":"vire","session_id":"session-abc","tool":"claude","ts":"2026-06-05T10:00:00Z","prompt":"SECRET_PROMPT","command_body":"SECRET_CMD rm -rf /","api_key":"sk-ant-SECRETTOKEN"}"#;
    let (config, _dir) = log_with(&[hostile, &finished("r1", "2026-06-05T10:05:00Z", 0)]);
    let c = conn();
    let snap = observe_with_config(&c, &config).unwrap();
    let json = serde_json::to_string(&snap).unwrap();
    for needle in ["SECRET", "sk-ant", "rm -rf", "command_body", "prompt"] {
        assert!(
            !json.contains(needle),
            "IPC surface must carry no content; found `{needle}` in {json}"
        );
    }
    // It does carry coarse counts + posture.
    assert!(json.contains("observed_runs"));
    assert!(json.contains("reconciliation_unknown"));
    assert!(json.contains("\"source\":\"local\""));
}

// ----- environment resolution unit ---------------------------------------------------------

#[test]
fn environment_resolution_map_then_allowed_then_none() {
    let mut env_map = HashMap::new();
    env_map.insert("alpha".to_string(), "vire".to_string());
    let config = RuntimeConfig {
        log_path: None,
        env_map,
        allowed_environments: vec!["vire".into(), "staging".into()],
        slop_secs: 300,
    };
    assert_eq!(
        config.resolve_environment("alpha").as_deref(),
        Some("vire"),
        "explicit map wins"
    );
    assert_eq!(
        config.resolve_environment("staging").as_deref(),
        Some("staging"),
        "token that is an allowed env maps to itself"
    );
    assert_eq!(
        config.resolve_environment("mystery"),
        None,
        "unmapped ⇒ None ⇒ unmatched_runtime"
    );
    assert_eq!(
        config.resolve_environment("  "),
        None,
        "blank token never resolves"
    );
}

#[test]
fn reconciliation_is_persisted_and_reread() {
    let c = conn();
    seed_evidence(
        &c,
        "T1",
        "vire",
        Some("session-1"),
        Some("2026-06-05T10:00:00Z"),
        Some("2026-06-05T10:30:00Z"),
    );
    let (config, _dir) = log_with(&[
        &started("r1", "vire", "session-1", "2026-06-05T10:00:00Z", "claude"),
        &finished("r1", "2026-06-05T10:30:00Z", 0),
    ]);
    observe_with_config(&c, &config).unwrap();
    let (recon, trace): (String, Option<String>) = c
        .query_row(
            "SELECT reconciliation, matched_trace_id FROM ai_runtime_sessions WHERE session_key='r1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(recon, "matched");
    assert_eq!(trace.as_deref(), Some("T1"));
}
