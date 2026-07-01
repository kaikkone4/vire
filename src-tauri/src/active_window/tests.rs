//! Tests for the active-window evidence store (TASK-046). Structural, adversarial, and lifecycle.
//! Mirrors the posture of `runtime_observer::tests` — no network, no process scanning.

use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};
use serde_json::json;

use super::config;
use super::model::{
    health_state, idle_state, source, title_state, CaptureHealthEvent, EvidenceBlock,
    RawObservation, RawObservationIn, TitleMode,
};
use super::settings_api::{self, CaptureSettingsInput};
use super::store;

// Serializes all tests that read or write process environment variables so that concurrent
// test threads cannot observe each other's env mutations (env is process-global state).
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, val: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, val);
        Self { key, prev }
    }
    fn remove(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    crate::init_db(&c).unwrap();
    c
}

/// Clear every capture-related env var (restored on drop) so a resolve/default test is a pure
/// function of the `settings` rows. Caller must also hold `ENV_LOCK`.
fn remove_capture_env() -> Vec<EnvGuard> {
    vec![
        EnvGuard::remove("VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED"),
        EnvGuard::remove("VIRE_ACTIVE_WINDOW_SAMPLE_SECONDS"),
        EnvGuard::remove("VIRE_ACTIVE_WINDOW_IDLE_CANDIDATE_SECONDS"),
        EnvGuard::remove("VIRE_ACTIVE_WINDOW_IDLE_AWAY_SECONDS"),
        EnvGuard::remove("VIRE_ACTIVE_WINDOW_RETENTION_DAYS"),
        EnvGuard::remove("VIRE_ACTIVE_WINDOW_TITLE_MODE"),
    ]
}

fn setting_value(c: &Connection, key: &str) -> Option<String> {
    c.query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .unwrap()
}

fn health_event(
    day: &str,
    start_ts: &str,
    end_ts: Option<&str>,
    state: &str,
    detail: Option<&str>,
) -> CaptureHealthEvent {
    CaptureHealthEvent {
        day: day.into(),
        start_ts: start_ts.into(),
        end_ts: end_ts.map(Into::into),
        state: state.into(),
        detail: detail.map(Into::into),
        source: source::NSWORKSPACE.into(),
    }
}

fn valid_input() -> CaptureSettingsInput {
    CaptureSettingsInput {
        capture_enabled: true,
        sample_seconds: 10,
        idle_candidate_seconds: 60,
        idle_away_seconds: 300,
        retention_days: 30,
    }
}

fn raw_obs(sample_ts: &str, day: &str, title: Option<&str>) -> RawObservation {
    let ts = title_state::CAPTURED;
    let no_ts = title_state::ABSENT_NO_WINDOW;
    RawObservation {
        sample_ts: sample_ts.into(),
        day: day.into(),
        app_name: Some("TestApp".into()),
        app_bundle_id: Some("com.example.test".into()),
        window_title: title.map(Into::into),
        title_state: if title.is_some() {
            ts.into()
        } else {
            no_ts.into()
        },
        idle_state: idle_state::ACTIVE.into(),
        source: source::NSWORKSPACE.into(),
        capture_health: None,
    }
}

fn evidence_block(id: &str, day: &str, title: Option<&str>) -> EvidenceBlock {
    let ts = title_state::CAPTURED;
    let no_ts = title_state::ABSENT_NO_WINDOW;
    EvidenceBlock {
        id: id.into(),
        day: day.into(),
        start_ts: format!("{day}T09:00:00Z"),
        end_ts: format!("{day}T09:30:00Z"),
        duration_seconds: 1800,
        app_name: Some("TestApp".into()),
        app_bundle_id: Some("com.example.test".into()),
        window_title: title.map(Into::into),
        title_state: if title.is_some() {
            ts.into()
        } else {
            no_ts.into()
        },
        idle_state: idle_state::ACTIVE.into(),
        source: source::NSWORKSPACE.into(),
        capture_health: None,
    }
}

// ----- migrate: idempotent + additive ----------------------------------------------------------

#[test]
fn migrate_is_idempotent_and_additive() {
    let c = Connection::open_in_memory().unwrap();
    store::migrate(&c).expect("first migrate");
    store::migrate(&c).expect("second migrate must be idempotent");
    for table in &[
        "active_window_raw_evidence",
        "active_window_evidence",
        "active_window_capture_health",
    ] {
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "table {table} must exist after migrate");
    }
}

// ----- no prohibited column (structural) -------------------------------------------------------

const PROHIBITED_COLS: &[&str] = &[
    "screenshot",
    "pixels",
    "keystroke",
    "key",
    "mouse",
    "url",
    "path",
    "command",
    "prompt",
    "response",
    "clipboard",
    "secret",
    "password",
    "token",
    "env",
];

fn table_columns(c: &Connection, table: &str) -> Vec<String> {
    let mut stmt = c.prepare(&format!("PRAGMA table_info({table})")).unwrap();
    stmt.query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn no_prohibited_column_in_any_active_window_table() {
    let c = conn();
    for table in &[
        "active_window_raw_evidence",
        "active_window_evidence",
        "active_window_capture_health",
    ] {
        let cols = table_columns(&c, table);
        for col in &cols {
            let lc = col.to_ascii_lowercase();
            for banned in PROHIBITED_COLS {
                assert!(
                    !lc.contains(banned),
                    "table `{table}` must have no `{banned}`-bearing column; found `{col}`"
                );
            }
        }
    }
}

// ----- allowlist drop (adversarial) ------------------------------------------------------------

#[test]
fn raw_observation_in_drops_prohibited_keys_before_anything_is_persisted() {
    let hostile = json!({
        "sample_ts": "2026-06-21T10:00:00Z",
        "app_name": "SafeApp",
        "app_bundle_id": "com.safe.app",
        "idle_state": "active",
        "source": "nsworkspace",
        // Injected prohibited keys — all must be silently dropped by serde:
        "prompt": "SECRET_PROMPT_payload",
        "command": "rm -rf / SECRET_CMD",
        "url": "https://secret.example.com/token?key=value",
        "clipboard": "CLIPBOARD_SECRET_VALUE",
        "secret": "MY_API_KEY_sk-ant-SECRET",
        "password": "PROHIBITED_SYNTHETIC_PW",  // gitleaks:allow
        "token": "SYNTHETIC_TOKEN_FIXTURE",      // gitleaks:allow
        "env": {"AWS_SECRET_ACCESS_KEY": "AKIASECRETKEY000"},
        "keystroke": "ctrl+c",
        "screenshot": "base64encodedPNG",
        "path": "/home/user/.ssh/id_rsa",
        "mouse": "x=100,y=200",
        "response": "SECRET_RESPONSE_text",
    });
    let obs: RawObservationIn = serde_json::from_value(hostile).unwrap();
    // Allowlisted fields survive.
    assert_eq!(obs.app_name.as_deref(), Some("SafeApp"));
    assert_eq!(obs.idle_state.as_deref(), Some("active"));
    // Insert via the typed API.
    let c = conn();
    let raw = RawObservation {
        sample_ts: obs
            .sample_ts
            .unwrap_or_else(|| "2026-06-21T10:00:00Z".into()),
        day: "2026-06-21".into(),
        app_name: obs.app_name,
        app_bundle_id: obs.app_bundle_id,
        window_title: obs.window_title,
        title_state: obs
            .title_state
            .unwrap_or_else(|| title_state::ABSENT_NO_WINDOW.into()),
        idle_state: obs.idle_state.unwrap_or_else(|| idle_state::ACTIVE.into()),
        source: obs.source.unwrap_or_else(|| source::NSWORKSPACE.into()),
        capture_health: obs.capture_health,
    };
    store::insert_raw_observation(&c, &raw, TitleMode::Redacted, "2026-06-21T10:00:00Z").unwrap();
    let dump: String = c
        .query_row(
            "SELECT COALESCE(app_name,'')||'|'||COALESCE(app_bundle_id,'')||'|'||
                    COALESCE(window_title,'')||'|'||title_state||'|'||idle_state||'|'||source
               FROM active_window_raw_evidence",
            [],
            |r| r.get(0),
        )
        .unwrap();
    for prohibited in [
        "SECRET",
        "PROHIBITED",
        "rm -rf",
        "AKIASECRET",
        "SYNTHETIC_TOKEN",
        "CLIPBOARD",
        "ctrl+c",
        "base64",
        "id_rsa",
    ] {
        assert!(
            !dump.contains(prohibited),
            "prohibited value `{prohibited}` must not be persisted; dump=`{dump}`"
        );
    }
    // Allowlisted values made it through.
    assert!(dump.contains("SafeApp"), "safe app_name must be preserved");
}

// ----- title redaction -------------------------------------------------------------------------

#[test]
fn default_redacted_mode_stores_null_title_and_redacted_state() {
    let c = conn();
    let obs = raw_obs(
        "2026-06-21T10:00:00Z",
        "2026-06-21",
        Some("SYNTHETIC_TITLE_DO_NOT_STORE"),
    );
    store::insert_raw_observation(&c, &obs, TitleMode::Redacted, "2026-06-21T10:00:00Z").unwrap();
    let (title, ts): (Option<String>, String) = c
        .query_row(
            "SELECT window_title, title_state FROM active_window_raw_evidence",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        title.is_none(),
        "window_title must be NULL under redacted mode"
    );
    assert_eq!(ts, title_state::REDACTED, "title_state must be 'redacted'");
}

#[test]
fn stored_mode_persists_title_with_captured_state() {
    let c = conn();
    let obs = raw_obs(
        "2026-06-21T10:00:00Z",
        "2026-06-21",
        Some("My Document Title"),
    );
    store::insert_raw_observation(&c, &obs, TitleMode::Stored, "2026-06-21T10:00:00Z").unwrap();
    let (title, ts): (Option<String>, String) = c
        .query_row(
            "SELECT window_title, title_state FROM active_window_raw_evidence",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(title.as_deref(), Some("My Document Title"));
    assert_eq!(ts, title_state::CAPTURED);
}

#[test]
fn read_projection_omits_title_under_redacted_mode() {
    let c = conn();
    // Write with stored mode so the title is actually in the DB.
    let block = evidence_block("b1", "2026-06-21", Some("Secret Title"));
    store::upsert_evidence_block(&c, &block, TitleMode::Stored, "2026-06-21T09:30:00Z").unwrap();
    // Read back under redacted mode — title must be absent from the projection.
    let views =
        store::evidence_blocks_in_range(&c, "2026-06-21", "2026-06-21", TitleMode::Redacted)
            .unwrap();
    assert_eq!(views.len(), 1);
    assert!(
        views[0].window_title.is_none(),
        "read projection must not return raw title under redacted mode"
    );
}

#[test]
fn stored_mode_evidence_block_round_trips_title() {
    let c = conn();
    let block = evidence_block("b1", "2026-06-21", Some("Round Trip Title"));
    store::upsert_evidence_block(&c, &block, TitleMode::Stored, "2026-06-21T09:30:00Z").unwrap();
    let views =
        store::evidence_blocks_in_range(&c, "2026-06-21", "2026-06-21", TitleMode::Stored).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].window_title.as_deref(), Some("Round Trip Title"));
    assert_eq!(views[0].title_state, title_state::CAPTURED);
}

#[test]
fn absent_title_states_pass_through_gate_unchanged() {
    // The three absence states (no title available) must pass through unchanged with title=None.
    // title_state='empty' is excluded here: it requires Some("") not None — see matrix test.
    let c = conn();
    for (ts_val, label) in [
        (title_state::ABSENT_NO_PERMISSION, "absent_no_permission"),
        (title_state::ABSENT_NO_WINDOW, "absent_no_window"),
        (title_state::ABSENT_UNSUPPORTED, "absent_unsupported"),
    ] {
        let obs = RawObservation {
            sample_ts: format!("2026-06-21T10:00:{label}Z").replace('_', "0"),
            day: "2026-06-21".into(),
            app_name: Some("TestApp".into()),
            app_bundle_id: Some(format!("com.test.{label}")),
            window_title: None,
            title_state: ts_val.into(),
            idle_state: idle_state::ACTIVE.into(),
            source: source::NSWORKSPACE.into(),
            capture_health: None,
        };
        store::insert_raw_observation(&c, &obs, TitleMode::Redacted, "2026-06-21T10:00:00Z")
            .unwrap();
        let stored_state: String = c
            .query_row(
                "SELECT title_state FROM active_window_raw_evidence WHERE app_bundle_id=?1",
                [format!("com.test.{label}").as_str()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            stored_state, ts_val,
            "absence state `{ts_val}` must pass through gate unchanged"
        );
    }
}

// ----- title/state consistency (Fix §8.2) -----------------------------------------------------

#[test]
fn stored_mode_rejects_absence_state_with_present_title() {
    // Contradictory input (presence state + non-empty title) must be rejected, not silently fixed.
    // The §8.2 truth table is fail-closed: the store cannot safely infer an absence reason.
    let c = conn();
    let obs = RawObservation {
        sample_ts: "2026-06-21T10:00:00Z".into(),
        day: "2026-06-21".into(),
        app_name: Some("TestApp".into()),
        app_bundle_id: Some("com.example.consistency".into()),
        window_title: Some("Actual Title".into()),
        title_state: title_state::ABSENT_NO_WINDOW.into(), // contradictory — must be rejected
        idle_state: idle_state::ACTIVE.into(),
        source: source::NSWORKSPACE.into(),
        capture_health: None,
    };
    let result = store::insert_raw_observation(&c, &obs, TitleMode::Stored, "2026-06-21T10:00:00Z");
    assert!(
        result.is_err(),
        "absence_state + non-empty title must be rejected (capture-side invariant violated)"
    );
}

// ----- title gate matrix: §8.2 truth table (both modes, both write paths) ---------------------

#[test]
fn title_gate_matrix_no_title_with_captured_state_rejected() {
    // captured + None (Absent) → reject in both modes for both paths.
    let c = conn();
    for mode in [TitleMode::Stored, TitleMode::Redacted] {
        let mut obs = raw_obs("2026-06-21T10:00:00Z", "2026-06-21", None);
        obs.title_state = title_state::CAPTURED.into(); // absent title, but captured state
        let result = store::insert_raw_observation(&c, &obs, mode, "2026-06-21T10:00:00Z");
        assert!(
            result.is_err(),
            "raw: captured+absent must be rejected in {mode:?} mode"
        );

        let mut block = evidence_block("m-no-title-captured", "2026-06-21", None);
        block.title_state = title_state::CAPTURED.into();
        let result = store::upsert_evidence_block(&c, &block, mode, "2026-06-21T09:30:00Z");
        assert!(
            result.is_err(),
            "evidence: captured+absent must be rejected in {mode:?} mode"
        );
    }
}

#[test]
fn title_gate_matrix_empty_string_title_with_empty_state() {
    // empty state + Some("") → (NULL, "empty") in both modes, both paths.
    // Verifies the invariant: window_title IS NULL ⟺ title_state ≠ 'captured'.
    let c = conn();

    // Raw — Stored mode
    store::insert_raw_observation(
        &c,
        &RawObservation {
            sample_ts: "2026-06-21T10:00:01Z".into(),
            day: "2026-06-21".into(),
            app_name: Some("TestApp".into()),
            app_bundle_id: Some("com.test.empty.stored".into()),
            window_title: Some("".into()), // Some("") → Empty class
            title_state: title_state::EMPTY.into(),
            idle_state: idle_state::ACTIVE.into(),
            source: source::NSWORKSPACE.into(),
            capture_health: None,
        },
        TitleMode::Stored,
        "2026-06-21T10:00:01Z",
    )
    .expect("raw stored: empty+empty must succeed");
    let (t, ts): (Option<String>, String) = c
        .query_row(
            "SELECT window_title, title_state FROM active_window_raw_evidence
              WHERE app_bundle_id='com.test.empty.stored'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        t.is_none(),
        "raw stored: empty state must store NULL title (invariant)"
    );
    assert_eq!(ts, title_state::EMPTY);

    // Raw — Redacted mode
    store::insert_raw_observation(
        &c,
        &RawObservation {
            sample_ts: "2026-06-21T10:00:02Z".into(),
            day: "2026-06-21".into(),
            app_name: Some("TestApp".into()),
            app_bundle_id: Some("com.test.empty.redacted".into()),
            window_title: Some("".into()),
            title_state: title_state::EMPTY.into(),
            idle_state: idle_state::ACTIVE.into(),
            source: source::NSWORKSPACE.into(),
            capture_health: None,
        },
        TitleMode::Redacted,
        "2026-06-21T10:00:02Z",
    )
    .expect("raw redacted: empty+empty must succeed");
    let (t, ts): (Option<String>, String) = c
        .query_row(
            "SELECT window_title, title_state FROM active_window_raw_evidence
              WHERE app_bundle_id='com.test.empty.redacted'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        t.is_none(),
        "raw redacted: empty state must store NULL title"
    );
    assert_eq!(ts, title_state::EMPTY);

    // Evidence block — Stored mode
    let mut block_stored = evidence_block("m-empty-stored", "2026-06-21", None);
    block_stored.window_title = Some("".into());
    block_stored.title_state = title_state::EMPTY.into();
    block_stored.app_bundle_id = Some("com.test.empty.block.stored".into());
    store::upsert_evidence_block(&c, &block_stored, TitleMode::Stored, "2026-06-21T09:30:00Z")
        .expect("evidence stored: empty+empty must succeed");
    let (t, ts): (Option<String>, String) = c
        .query_row(
            "SELECT window_title, title_state FROM active_window_evidence WHERE id='m-empty-stored'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        t.is_none(),
        "evidence stored: empty state must store NULL title (invariant)"
    );
    assert_eq!(ts, title_state::EMPTY);

    // Evidence block — Redacted mode
    let mut block_redacted = evidence_block("m-empty-redacted", "2026-06-21", None);
    block_redacted.window_title = Some("".into());
    block_redacted.title_state = title_state::EMPTY.into();
    block_redacted.app_bundle_id = Some("com.test.empty.block.redacted".into());
    store::upsert_evidence_block(
        &c,
        &block_redacted,
        TitleMode::Redacted,
        "2026-06-21T09:30:00Z",
    )
    .expect("evidence redacted: empty+empty must succeed");
    let (t, ts): (Option<String>, String) = c
        .query_row(
            "SELECT window_title, title_state FROM active_window_evidence
              WHERE id='m-empty-redacted'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        t.is_none(),
        "evidence redacted: empty state must store NULL title"
    );
    assert_eq!(ts, title_state::EMPTY);
}

#[test]
fn title_gate_matrix_title_with_absence_state_rejected() {
    // Absence states + non-empty title (Present) → reject in both modes, both paths.
    let c = conn();
    for absence_state in [
        title_state::ABSENT_NO_PERMISSION,
        title_state::ABSENT_NO_WINDOW,
        title_state::ABSENT_UNSUPPORTED,
    ] {
        for mode in [TitleMode::Stored, TitleMode::Redacted] {
            let obs = RawObservation {
                sample_ts: "2026-06-21T10:00:00Z".into(),
                day: "2026-06-21".into(),
                app_name: None,
                app_bundle_id: None,
                window_title: Some("Contradictory Title".into()),
                title_state: absence_state.into(),
                idle_state: idle_state::ACTIVE.into(),
                source: source::NSWORKSPACE.into(),
                capture_health: None,
            };
            let result = store::insert_raw_observation(&c, &obs, mode, "2026-06-21T10:00:00Z");
            assert!(
                result.is_err(),
                "raw: {absence_state}+present must be rejected in {mode:?} mode"
            );

            let mut block = evidence_block("m-abs-title", "2026-06-21", None);
            block.window_title = Some("Contradictory Title".into());
            block.title_state = absence_state.into();
            let result = store::upsert_evidence_block(&c, &block, mode, "2026-06-21T09:30:00Z");
            assert!(
                result.is_err(),
                "evidence: {absence_state}+present must be rejected in {mode:?} mode"
            );
        }
    }
}

// ----- controlled vocabulary enforcement (Fix 3) -----------------------------------------------

#[test]
fn insert_raw_observation_rejects_invalid_title_state() {
    let c = conn();
    let obs = RawObservation {
        sample_ts: "2026-06-21T10:00:00Z".into(),
        day: "2026-06-21".into(),
        app_name: None,
        app_bundle_id: None,
        window_title: None,
        title_state: "invalid_state".into(),
        idle_state: idle_state::ACTIVE.into(),
        source: source::NSWORKSPACE.into(),
        capture_health: None,
    };
    let result =
        store::insert_raw_observation(&c, &obs, TitleMode::Redacted, "2026-06-21T10:00:00Z");
    assert!(
        result.is_err(),
        "invalid title_state must be rejected at the write boundary"
    );
}

#[test]
fn insert_raw_observation_rejects_invalid_idle_state() {
    let c = conn();
    let obs = RawObservation {
        sample_ts: "2026-06-21T10:00:00Z".into(),
        day: "2026-06-21".into(),
        app_name: None,
        app_bundle_id: None,
        window_title: None,
        title_state: title_state::ABSENT_NO_WINDOW.into(),
        idle_state: "unknown_idle".into(),
        source: source::NSWORKSPACE.into(),
        capture_health: None,
    };
    let result =
        store::insert_raw_observation(&c, &obs, TitleMode::Redacted, "2026-06-21T10:00:00Z");
    assert!(
        result.is_err(),
        "invalid idle_state must be rejected at the write boundary"
    );
}

#[test]
fn upsert_evidence_block_rejects_invalid_source() {
    let c = conn();
    let mut block = evidence_block("b1", "2026-06-21", None);
    block.source = "invalid_source".into();
    let result =
        store::upsert_evidence_block(&c, &block, TitleMode::Redacted, "2026-06-21T09:30:00Z");
    assert!(
        result.is_err(),
        "invalid source must be rejected at the write boundary"
    );
}

#[test]
fn record_capture_health_rejects_invalid_state() {
    let c = conn();
    let ev = CaptureHealthEvent {
        day: "2026-06-21".into(),
        start_ts: "2026-06-21T10:00:00Z".into(),
        end_ts: None,
        state: "not_a_real_state".into(),
        detail: None,
        source: source::NSWORKSPACE.into(),
    };
    let result = store::record_capture_health(&c, &ev, "2026-06-21T10:00:00Z");
    assert!(
        result.is_err(),
        "invalid health state must be rejected at the write boundary"
    );
}

#[test]
fn record_capture_health_rejects_oversized_detail() {
    let c = conn();
    let oversized_detail = "x".repeat(store::MAX_DETAIL_BYTES + 1);
    let ev = CaptureHealthEvent {
        day: "2026-06-21".into(),
        start_ts: "2026-06-21T10:00:00Z".into(),
        end_ts: None,
        state: health_state::SAMPLING_GAP.into(),
        detail: Some(oversized_detail),
        source: source::NSWORKSPACE.into(),
    };
    let result = store::record_capture_health(&c, &ev, "2026-06-21T10:00:00Z");
    assert!(
        result.is_err(),
        "detail exceeding MAX_DETAIL_BYTES must be rejected"
    );
}

// ----- capture_health vocab enforcement on raw/evidence write paths (Fix §8.1) ----------------

#[test]
fn insert_raw_observation_rejects_invalid_capture_health() {
    let c = conn();
    let mut obs = raw_obs("2026-06-21T10:00:00Z", "2026-06-21", None);
    obs.capture_health = Some("bad_health_state".into());
    let result =
        store::insert_raw_observation(&c, &obs, TitleMode::Redacted, "2026-06-21T10:00:00Z");
    assert!(
        result.is_err(),
        "invalid capture_health must be rejected at the raw write boundary"
    );
}

#[test]
fn insert_raw_observation_accepts_none_capture_health() {
    let c = conn();
    let mut obs = raw_obs("2026-06-21T10:00:00Z", "2026-06-21", None);
    obs.capture_health = None;
    store::insert_raw_observation(&c, &obs, TitleMode::Redacted, "2026-06-21T10:00:00Z")
        .expect("None capture_health must be accepted (nullable per schema)");
}

#[test]
fn upsert_evidence_block_rejects_invalid_capture_health() {
    let c = conn();
    let mut block = evidence_block("b-health-invalid", "2026-06-21", None);
    block.capture_health = Some("bad_health_state".into());
    let result =
        store::upsert_evidence_block(&c, &block, TitleMode::Redacted, "2026-06-21T09:30:00Z");
    assert!(
        result.is_err(),
        "invalid capture_health must be rejected at the evidence write boundary"
    );
}

#[test]
fn upsert_evidence_block_accepts_none_capture_health() {
    let c = conn();
    let mut block = evidence_block("b-health-none", "2026-06-21", None);
    block.capture_health = None;
    store::upsert_evidence_block(&c, &block, TitleMode::Redacted, "2026-06-21T09:30:00Z")
        .expect("None capture_health must be accepted (nullable per schema)");
}

// ----- capture health first-class --------------------------------------------------------------

#[test]
fn capture_health_stored_as_first_class_rows_not_dropped() {
    let c = conn();
    let ev = CaptureHealthEvent {
        day: "2026-06-21".into(),
        start_ts: "2026-06-21T10:00:00Z".into(),
        end_ts: None,
        state: health_state::AX_PERMISSION_DENIED.into(),
        detail: Some("AXErrorAPIDisabled".into()),
        source: source::ACCESSIBILITY.into(),
    };
    store::record_capture_health(&c, &ev, "2026-06-21T10:00:00Z").unwrap();
    let events = store::capture_health_in_range(&c, "2026-06-21", "2026-06-21").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].state, health_state::AX_PERMISSION_DENIED);
    assert_eq!(events[0].detail.as_deref(), Some("AXErrorAPIDisabled"));
}

#[test]
fn capture_health_vocabulary_round_trips() {
    let c = conn();
    let states = [
        health_state::AX_PERMISSION_DENIED,
        health_state::SCREEN_RECORDING_ABSENT,
        health_state::NO_FOCUSED_WINDOW,
        health_state::WINDOW_UNAVAILABLE,
        health_state::TITLE_EMPTY,
        health_state::SAMPLING_GAP,
        health_state::SYSTEM_SLEEP,
        health_state::SYSTEM_WAKE,
        health_state::SCREEN_LOCKED,
        health_state::HELPER_CRASH,
        health_state::HELPER_RESTART,
        health_state::NO_GUI_SESSION,
    ];
    for state in states {
        store::record_capture_health(
            &c,
            &CaptureHealthEvent {
                day: "2026-06-21".into(),
                start_ts: format!("2026-06-21T10:00:00Z-{state}"),
                end_ts: None,
                state: state.into(),
                detail: None,
                source: source::NSWORKSPACE.into(),
            },
            "2026-06-21T10:00:00Z",
        )
        .unwrap();
    }
    let events = store::capture_health_in_range(&c, "2026-06-21", "2026-06-21").unwrap();
    assert_eq!(
        events.len(),
        states.len(),
        "all health states must be stored as first-class rows"
    );
}

// ----- upsert idempotency ----------------------------------------------------------------------

#[test]
fn upsert_evidence_block_is_idempotent() {
    let c = conn();
    let block = evidence_block("b1", "2026-06-21", None);
    store::upsert_evidence_block(&c, &block, TitleMode::Redacted, "2026-06-21T09:30:00Z").unwrap();
    store::upsert_evidence_block(&c, &block, TitleMode::Redacted, "2026-06-21T09:30:01Z").unwrap();
    let count: i64 = c
        .query_row("SELECT COUNT(*) FROM active_window_evidence", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        count, 1,
        "re-upserting the same block must not create duplicate rows"
    );
}

#[test]
fn upsert_evidence_block_is_idempotent_when_bundle_id_is_none() {
    // Regression: SQLite treats NULL as distinct in UNIQUE constraints; repeated upserts of a
    // no-bundle block must not create duplicate rows.
    let c = conn();
    let mut block = evidence_block("no-bundle", "2026-06-21", None);
    block.app_bundle_id = None;
    store::upsert_evidence_block(&c, &block, TitleMode::Redacted, "2026-06-21T09:30:00Z").unwrap();
    store::upsert_evidence_block(&c, &block, TitleMode::Redacted, "2026-06-21T09:30:01Z").unwrap();
    let count: i64 = c
        .query_row("SELECT COUNT(*) FROM active_window_evidence", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        count, 1,
        "re-upserting a no-bundle block must not create duplicate rows"
    );
}

#[test]
fn evidence_block_bundle_id_none_reads_back_as_none() {
    // The empty-string sentinel stored for null bundle IDs must be transparent at the API level.
    let c = conn();
    let mut block = evidence_block("no-bundle", "2026-06-21", None);
    block.app_bundle_id = None;
    store::upsert_evidence_block(&c, &block, TitleMode::Redacted, "2026-06-21T09:30:00Z").unwrap();
    let views =
        store::evidence_blocks_in_range(&c, "2026-06-21", "2026-06-21", TitleMode::Redacted)
            .unwrap();
    assert_eq!(views.len(), 1);
    assert!(
        views[0].app_bundle_id.is_none(),
        "app_bundle_id=None must round-trip through the sentinel as None, not empty string"
    );
}

// ----- retention safety (C6) -------------------------------------------------------------------

#[test]
fn prune_removes_only_expired_active_window_rows_and_leaves_time_entries_intact() {
    let c = conn();
    // Seed a time_entries sentinel row (C6 proof: retention must never touch approved human time).
    let project = crate::create_project_repo(
        &c,
        crate::ProjectInput {
            name: "Sentinel Project".into(),
            notes: None,
        },
    )
    .unwrap();
    crate::create_entry_repo(
        &c,
        crate::TimeEntryInput {
            project_id: project.id,
            date: "2026-01-01".into(),
            start_time: "09:00".into(),
            end_time: "10:00".into(),
            note: None,
        },
    )
    .unwrap();

    // Expired raw row (older than 30 days from 2026-06-21 → before 2026-05-22).
    store::insert_raw_observation(
        &c,
        &raw_obs("2026-05-01T10:00:00Z", "2026-05-01", None),
        TitleMode::Redacted,
        "2026-05-01T10:00:00Z",
    )
    .unwrap();
    // In-window raw row (within 30 days).
    store::insert_raw_observation(
        &c,
        &raw_obs("2026-06-20T10:00:00Z", "2026-06-20", None),
        TitleMode::Redacted,
        "2026-06-20T10:00:00Z",
    )
    .unwrap();
    // Expired evidence block.
    store::upsert_evidence_block(
        &c,
        &evidence_block("old-block", "2026-05-01", None),
        TitleMode::Redacted,
        "2026-05-01T10:00:00Z",
    )
    .unwrap();
    // Expired health row.
    store::record_capture_health(
        &c,
        &CaptureHealthEvent {
            day: "2026-05-01".into(),
            start_ts: "2026-05-01T10:00:00Z".into(),
            end_ts: None,
            state: health_state::SAMPLING_GAP.into(),
            detail: None,
            source: source::NSWORKSPACE.into(),
        },
        "2026-05-01T10:00:00Z",
    )
    .unwrap();

    // Prune with now=2026-06-21, retention=30 days → cutoff=2026-05-22T00:00:00.
    let stats = store::prune_expired(&c, "2026-06-21", 30).unwrap();
    assert_eq!(
        stats.raw_evidence_deleted, 1,
        "expired raw row must be deleted"
    );
    assert_eq!(
        stats.evidence_deleted, 1,
        "expired evidence block must be deleted"
    );
    assert_eq!(
        stats.capture_health_deleted, 1,
        "expired health row must be deleted"
    );

    // In-window raw row must survive.
    let raw_count: i64 = c
        .query_row("SELECT COUNT(*) FROM active_window_raw_evidence", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(raw_count, 1, "in-window raw row must survive prune");

    // time_entries sentinel must be completely untouched.
    let te_count: i64 = c
        .query_row("SELECT COUNT(*) FROM time_entries", [], |r| r.get(0))
        .unwrap();
    assert_eq!(te_count, 1, "time_entries must be unaffected by prune (C6)");
}

// ----- prune exact timestamp cutoff (Fix 4) ---------------------------------------------------

#[test]
fn prune_uses_exact_per_table_timestamp_not_calendar_day() {
    // Row whose sample_ts is before the cutoff time on the cutoff day must be deleted.
    // Under the old day-based prune it would have been kept (day == cutoff_day is not < cutoff_day).
    let c = conn();
    let cutoff_day = "2026-05-22";
    let before_cutoff = "2026-05-22T09:00:00Z"; // same cutoff day, before cutoff time
    let after_cutoff = "2026-05-22T12:00:00Z"; // same cutoff day, after cutoff time (now=noon)
    let now_ts = "2026-06-21T12:00:00Z"; // retention=30 → cutoff = 2026-05-22 12:00:00

    // Raw: sample_ts before cutoff → must be deleted.
    store::insert_raw_observation(
        &c,
        &RawObservation {
            sample_ts: before_cutoff.into(),
            day: cutoff_day.into(),
            app_name: None,
            app_bundle_id: Some("com.before".into()),
            window_title: None,
            title_state: title_state::ABSENT_NO_WINDOW.into(),
            idle_state: idle_state::ACTIVE.into(),
            source: source::NSWORKSPACE.into(),
            capture_health: None,
        },
        TitleMode::Redacted,
        before_cutoff,
    )
    .unwrap();
    // Raw: sample_ts after cutoff → must survive.
    store::insert_raw_observation(
        &c,
        &RawObservation {
            sample_ts: after_cutoff.into(),
            day: cutoff_day.into(),
            app_name: None,
            app_bundle_id: Some("com.after".into()),
            window_title: None,
            title_state: title_state::ABSENT_NO_WINDOW.into(),
            idle_state: idle_state::ACTIVE.into(),
            source: source::NSWORKSPACE.into(),
            capture_health: None,
        },
        TitleMode::Redacted,
        after_cutoff,
    )
    .unwrap();

    let stats = store::prune_expired(&c, now_ts, 30).unwrap();
    assert_eq!(
        stats.raw_evidence_deleted, 1,
        "row with sample_ts before exact cutoff must be deleted even when on the cutoff day"
    );
    let remaining: i64 = c
        .query_row("SELECT COUNT(*) FROM active_window_raw_evidence", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(remaining, 1, "row with sample_ts after cutoff must survive");
}

#[test]
fn prune_keeps_row_at_exact_cutoff_deletes_one_second_before() {
    // Boundary: row exactly at the cutoff timestamp (not strictly less-than) must survive.
    let c = conn();
    let now_ts = "2026-06-21T00:00:00Z";
    let cutoff = "2026-05-22T00:00:00Z"; // datetime(now, "-30 days")

    // Row at exact cutoff → must survive (cutoff < cutoff is false).
    store::insert_raw_observation(
        &c,
        &RawObservation {
            sample_ts: cutoff.into(),
            day: "2026-05-22".into(),
            app_name: None,
            app_bundle_id: Some("com.at-cutoff".into()),
            window_title: None,
            title_state: title_state::ABSENT_NO_WINDOW.into(),
            idle_state: idle_state::ACTIVE.into(),
            source: source::NSWORKSPACE.into(),
            capture_health: None,
        },
        TitleMode::Redacted,
        cutoff,
    )
    .unwrap();
    // Row one second before cutoff → must be deleted.
    store::insert_raw_observation(
        &c,
        &RawObservation {
            sample_ts: "2026-05-21T23:59:59Z".into(),
            day: "2026-05-21".into(),
            app_name: None,
            app_bundle_id: Some("com.just-before".into()),
            window_title: None,
            title_state: title_state::ABSENT_NO_WINDOW.into(),
            idle_state: idle_state::ACTIVE.into(),
            source: source::NSWORKSPACE.into(),
            capture_health: None,
        },
        TitleMode::Redacted,
        "2026-05-21T23:59:59Z",
    )
    .unwrap();

    let stats = store::prune_expired(&c, now_ts, 30).unwrap();
    assert_eq!(
        stats.raw_evidence_deleted, 1,
        "row one second before cutoff must be deleted"
    );
    let row_at_cutoff: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM active_window_raw_evidence WHERE app_bundle_id='com.at-cutoff'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        row_at_cutoff, 1,
        "row exactly at the cutoff timestamp must survive"
    );
}

// ----- config: precedence (Fix 2) --------------------------------------------------------------

#[test]
fn config_default_when_no_settings_no_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _r1 = EnvGuard::remove("VIRE_ACTIVE_WINDOW_RETENTION_DAYS");
    let _r2 = EnvGuard::remove("VIRE_ACTIVE_WINDOW_TITLE_MODE");
    let c = conn();
    let cfg = config::ActiveWindowConfig::from_settings(&c).unwrap();
    assert_eq!(
        cfg.retention_days,
        config::DEFAULT_RETENTION_DAYS,
        "retention must be the default when no settings row and no env var"
    );
    assert_eq!(
        cfg.title_mode,
        TitleMode::Redacted,
        "title_mode must default to Redacted"
    );
}

#[test]
fn config_stored_retention_overrides_default() {
    let c = conn();
    c.execute(
        "INSERT OR REPLACE INTO settings(key, value) VALUES ('active_window_retention_days', '90')",
        [],
    )
    .unwrap();
    let cfg = config::ActiveWindowConfig::from_settings(&c).unwrap();
    assert_eq!(
        cfg.retention_days, 90,
        "stored retention_days must take precedence"
    );
}

#[test]
fn config_stored_title_mode_stored_overrides_default() {
    let c = conn();
    c.execute(
        "INSERT OR REPLACE INTO settings(key, value) VALUES ('active_window_title_mode', 'stored')",
        [],
    )
    .unwrap();
    let cfg = config::ActiveWindowConfig::from_settings(&c).unwrap();
    assert_eq!(
        cfg.title_mode,
        TitleMode::Stored,
        "stored title_mode must take precedence"
    );
}

#[test]
fn config_invalid_stored_retention_falls_back_to_default() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _r1 = EnvGuard::remove("VIRE_ACTIVE_WINDOW_RETENTION_DAYS");
    let c = conn();
    c.execute(
        "INSERT OR REPLACE INTO settings(key, value) VALUES ('active_window_retention_days', 'not_a_number')",
        [],
    )
    .unwrap();
    let cfg = config::ActiveWindowConfig::from_settings(&c).unwrap();
    assert_eq!(
        cfg.retention_days,
        config::DEFAULT_RETENTION_DAYS,
        "non-numeric stored retention must fall back to default (env cleared)"
    );
}

// ----- config env-only and DB-over-env precedence tests (Fix §8.3) ----------------------------
//
// All tests acquire ENV_LOCK and use EnvGuard for save/restore so CI ambient env cannot flip
// results and concurrent test threads cannot observe each other's env mutations.

#[test]
fn config_env_only_retention_wins_over_default() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _r1 = EnvGuard::set("VIRE_ACTIVE_WINDOW_RETENTION_DAYS", "60");
    let _r2 = EnvGuard::remove("VIRE_ACTIVE_WINDOW_TITLE_MODE");
    let c = conn(); // no stored setting
    let cfg = config::ActiveWindowConfig::from_settings(&c).unwrap();
    assert_eq!(
        cfg.retention_days, 60,
        "env-only retention must win over the compile-time default"
    );
}

#[test]
fn config_env_only_title_mode_wins_over_default() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _r1 = EnvGuard::remove("VIRE_ACTIVE_WINDOW_RETENTION_DAYS");
    let _r2 = EnvGuard::set("VIRE_ACTIVE_WINDOW_TITLE_MODE", "stored");
    let c = conn(); // no stored setting
    let cfg = config::ActiveWindowConfig::from_settings(&c).unwrap();
    assert_eq!(
        cfg.title_mode,
        TitleMode::Stored,
        "env-only title_mode must win over the compile-time default"
    );
}

#[test]
fn config_stored_retention_overrides_conflicting_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _r1 = EnvGuard::set("VIRE_ACTIVE_WINDOW_RETENTION_DAYS", "60");
    let _r2 = EnvGuard::remove("VIRE_ACTIVE_WINDOW_TITLE_MODE");
    let c = conn();
    c.execute(
        "INSERT OR REPLACE INTO settings(key, value) VALUES ('active_window_retention_days', '90')",
        [],
    )
    .unwrap();
    let cfg = config::ActiveWindowConfig::from_settings(&c).unwrap();
    assert_eq!(
        cfg.retention_days, 90,
        "stored DB retention must take precedence over a conflicting env var (DB > env > default)"
    );
}

#[test]
fn config_stored_title_mode_overrides_conflicting_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _r1 = EnvGuard::remove("VIRE_ACTIVE_WINDOW_RETENTION_DAYS");
    let _r2 = EnvGuard::set("VIRE_ACTIVE_WINDOW_TITLE_MODE", "stored");
    let c = conn();
    c.execute(
        "INSERT OR REPLACE INTO settings(key, value) VALUES ('active_window_title_mode', 'redacted')",
        [],
    )
    .unwrap();
    let cfg = config::ActiveWindowConfig::from_settings(&c).unwrap();
    assert_eq!(
        cfg.title_mode,
        TitleMode::Redacted,
        "stored DB title_mode must take precedence over a conflicting env var"
    );
}

// ----- no raw title in logs (structural) -------------------------------------------------------

#[test]
fn write_path_does_not_persist_title_under_redacted_mode() {
    // Structural proof: the write API discards the title before any SQL bind under redacted mode.
    // If the title reaches the store, the NULL assertion below will fail — the same invariant
    // that prevents it from reaching a log line.
    let c = conn();
    let title = "SYNTHETIC_TITLE_THAT_MUST_NOT_APPEAR_ANYWHERE";
    let obs = raw_obs("2026-06-21T10:00:00Z", "2026-06-21", Some(title));
    store::insert_raw_observation(&c, &obs, TitleMode::Redacted, "2026-06-21T10:00:00Z").unwrap();
    let stored: Option<String> = c
        .query_row(
            "SELECT window_title FROM active_window_raw_evidence",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        stored.is_none(),
        "title must not reach the store under redacted mode; also never reaches a log"
    );
}

// ----- TASK-056 B: capture_status_snapshot (read-only status/health projection) ----------------

#[test]
fn status_snapshot_empty_db_is_zero_counts_and_none() {
    let c = conn();
    let snap = store::capture_status_snapshot(&c, "2026-06-21", "2026-05-22").unwrap();
    assert!(snap.last_sample_ts.is_none(), "empty DB has no last sample");
    assert_eq!(snap.samples_today, 0);
    assert_eq!(snap.evidence_blocks_retained, 0);
    assert!(snap.open_health.is_empty());
    assert!(snap.recent_health.is_empty());
}

#[test]
fn status_snapshot_counts_samples_today_and_reports_max_sample_ts() {
    let c = conn();
    for ts in ["2026-06-21 09:00:00", "2026-06-21 09:05:00"] {
        store::insert_raw_observation(
            &c,
            &raw_obs(ts, "2026-06-21", None),
            TitleMode::Redacted,
            ts,
        )
        .unwrap();
    }
    let earlier = "2026-06-20 09:00:00";
    store::insert_raw_observation(
        &c,
        &raw_obs(earlier, "2026-06-20", None),
        TitleMode::Redacted,
        earlier,
    )
    .unwrap();

    let snap = store::capture_status_snapshot(&c, "2026-06-21", "2026-05-22").unwrap();
    assert_eq!(snap.samples_today, 2, "only today's raw rows count");
    assert_eq!(
        snap.last_sample_ts.as_deref(),
        Some("2026-06-21 09:05:00"),
        "last_sample_ts is MAX(sample_ts) across all days"
    );
}

#[test]
fn status_snapshot_counts_only_evidence_within_retention_window() {
    let c = conn();
    store::upsert_evidence_block(
        &c,
        &evidence_block("in-1", "2026-06-21", None),
        TitleMode::Redacted,
        "2026-06-21 09:30:00",
    )
    .unwrap();
    store::upsert_evidence_block(
        &c,
        &evidence_block("in-2", "2026-05-22", None),
        TitleMode::Redacted,
        "2026-05-22 09:30:00",
    )
    .unwrap();
    store::upsert_evidence_block(
        &c,
        &evidence_block("out-1", "2026-04-01", None),
        TitleMode::Redacted,
        "2026-04-01 09:30:00",
    )
    .unwrap();

    let snap = store::capture_status_snapshot(&c, "2026-06-21", "2026-05-22").unwrap();
    assert_eq!(
        snap.evidence_blocks_retained, 2,
        "only blocks with day >= retention_from_day count (boundary day is inclusive)"
    );
}

#[test]
fn status_snapshot_separates_open_and_recent_health() {
    let c = conn();
    store::record_capture_health(
        &c,
        &health_event(
            "2026-06-21",
            "2026-06-21 14:02:00",
            None,
            health_state::NO_GUI_SESSION,
            Some("code=1"),
        ),
        "2026-06-21 14:02:00",
    )
    .unwrap();
    store::record_capture_health(
        &c,
        &health_event(
            "2026-06-21",
            "2026-06-21 10:00:00",
            Some("2026-06-21 10:01:00"),
            health_state::SAMPLING_GAP,
            None,
        ),
        "2026-06-21 10:00:00",
    )
    .unwrap();

    let snap = store::capture_status_snapshot(&c, "2026-06-21", "2026-05-22").unwrap();
    assert_eq!(
        snap.open_health.len(),
        1,
        "only end_ts IS NULL rows are open"
    );
    assert_eq!(snap.open_health[0].state, health_state::NO_GUI_SESSION);
    assert_eq!(snap.open_health[0].since_or_start_ts, "2026-06-21 14:02:00");
    assert_eq!(snap.open_health[0].detail.as_deref(), Some("code=1"));
    assert_eq!(
        snap.recent_health.len(),
        2,
        "recent_health includes both open and closed markers"
    );
    assert_eq!(
        snap.recent_health[0].state,
        health_state::NO_GUI_SESSION,
        "recent_health is newest-first"
    );
}

#[test]
fn status_snapshot_recent_health_is_bounded_and_newest_first() {
    let c = conn();
    for i in 0..(store::RECENT_HEALTH_LIMIT + 5) {
        let ts = format!("2026-06-21 10:{i:02}:00");
        store::record_capture_health(
            &c,
            &health_event(
                "2026-06-21",
                &ts,
                Some(&ts),
                health_state::SAMPLING_GAP,
                None,
            ),
            &ts,
        )
        .unwrap();
    }
    let snap = store::capture_status_snapshot(&c, "2026-06-21", "2026-05-22").unwrap();
    assert_eq!(
        snap.recent_health.len() as i64,
        store::RECENT_HEALTH_LIMIT,
        "recent_health is capped at RECENT_HEALTH_LIMIT"
    );
    assert!(
        snap.recent_health[0].since_or_start_ts > snap.recent_health[1].since_or_start_ts,
        "recent_health is ordered newest-first"
    );
}

// ----- TASK-056 A: settings_api (validated read/write seam) ------------------------------------

#[test]
fn settings_api_rejects_zero_sample_seconds() {
    let c = conn();
    let mut input = valid_input();
    input.sample_seconds = 0;
    let err = settings_api::apply(&c, &input, "2026-06-21 09:00:00").unwrap_err();
    assert!(
        err.to_lowercase().contains("sample interval"),
        "clear error mentioning the field: {err}"
    );
    assert!(
        setting_value(&c, "active_window_sample_seconds").is_none(),
        "a rejected write persists no row"
    );
}

#[test]
fn settings_api_rejects_away_not_greater_than_idle_candidate() {
    let c = conn();
    let mut input = valid_input();
    input.idle_candidate_seconds = 300;
    input.idle_away_seconds = 300; // not strictly greater — the ordering invariant is violated
    let err = settings_api::apply(&c, &input, "2026-06-21 09:00:00").unwrap_err();
    assert!(
        err.to_lowercase().contains("away"),
        "clear ordering error: {err}"
    );
    assert!(setting_value(&c, "active_window_idle_away_seconds").is_none());
}

#[test]
fn settings_api_rejects_zero_retention() {
    let c = conn();
    let mut input = valid_input();
    input.retention_days = 0;
    let err = settings_api::apply(&c, &input, "2026-06-21 09:00:00").unwrap_err();
    assert!(
        err.to_lowercase().contains("retention"),
        "clear error: {err}"
    );
    assert!(setting_value(&c, "active_window_retention_days").is_none());
}

#[test]
fn settings_api_rejects_out_of_range_values() {
    let c = conn();
    for mutate in [
        |i: &mut CaptureSettingsInput| i.sample_seconds = 3601,
        |i: &mut CaptureSettingsInput| i.idle_candidate_seconds = 86_401,
        |i: &mut CaptureSettingsInput| i.idle_away_seconds = 86_401,
        |i: &mut CaptureSettingsInput| i.retention_days = 3651,
    ] {
        let mut input = valid_input();
        mutate(&mut input);
        assert!(
            settings_api::apply(&c, &input, "2026-06-21 09:00:00").is_err(),
            "out-of-range value must be rejected"
        );
    }
    // Nothing persisted by any rejected write.
    assert!(setting_value(&c, "active_window_sample_seconds").is_none());
}

#[test]
fn settings_api_rejected_write_leaves_prior_config_unchanged() {
    let c = conn();
    settings_api::apply(&c, &valid_input(), "2026-06-21 09:00:00").unwrap();
    let before = setting_value(&c, "active_window_sample_seconds");
    let mut bad = valid_input();
    bad.sample_seconds = 0;
    assert!(settings_api::apply(&c, &bad, "2026-06-21 09:00:00").is_err());
    assert_eq!(
        setting_value(&c, "active_window_sample_seconds"),
        before,
        "a rejected write changes no already-persisted row"
    );
}

#[test]
fn settings_api_valid_set_writes_five_keys_and_resolves() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _env = remove_capture_env();
    let c = conn();
    let input = CaptureSettingsInput {
        capture_enabled: true,
        sample_seconds: 12,
        idle_candidate_seconds: 45,
        idle_away_seconds: 240,
        retention_days: 14,
    };
    let view = settings_api::apply(&c, &input, "2026-06-21 09:00:00").unwrap();

    assert_eq!(
        setting_value(&c, "active_window_capture_enabled").as_deref(),
        Some("true")
    );
    assert_eq!(
        setting_value(&c, "active_window_sample_seconds").as_deref(),
        Some("12")
    );
    assert_eq!(
        setting_value(&c, "active_window_idle_candidate_seconds").as_deref(),
        Some("45")
    );
    assert_eq!(
        setting_value(&c, "active_window_idle_away_seconds").as_deref(),
        Some("240")
    );
    assert_eq!(
        setting_value(&c, "active_window_retention_days").as_deref(),
        Some("14")
    );

    assert!(view.capture_enabled);
    assert_eq!(view.sample_seconds, 12);
    assert_eq!(view.idle_candidate_seconds, 45);
    assert_eq!(view.idle_away_seconds, 240);
    assert_eq!(view.retention_days, 14);

    // The capture loop's own resolver sees exactly the same values.
    let cfg = config::CaptureConfig::from_settings(&c).unwrap();
    assert!(cfg.capture_enabled);
    assert_eq!(cfg.sample_seconds, 12);
    assert_eq!(cfg.idle_candidate_seconds, 45);
    assert_eq!(cfg.away_seconds, 240);
    assert_eq!(cfg.retention_days, 14);

    // get() re-resolves identically.
    let got = settings_api::resolve_view(&c, "2026-06-21 09:00:00").unwrap();
    assert!(got.capture_enabled);
    assert_eq!(got.sample_seconds, 12);
}

#[test]
fn settings_api_enable_then_disable_round_trips() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _env = remove_capture_env();
    let c = conn();
    let mut input = valid_input();
    input.capture_enabled = true;
    let v1 = settings_api::apply(&c, &input, "2026-06-21 09:00:00").unwrap();
    assert!(v1.capture_enabled);
    input.capture_enabled = false;
    let v2 = settings_api::apply(&c, &input, "2026-06-21 09:00:00").unwrap();
    assert!(!v2.capture_enabled);
    assert_eq!(
        setting_value(&c, "active_window_capture_enabled").as_deref(),
        Some("false")
    );
}

#[test]
fn settings_api_set_never_writes_title_mode() {
    let c = conn();
    let view = settings_api::apply(&c, &valid_input(), "2026-06-21 09:00:00").unwrap();
    assert!(
        setting_value(&c, "active_window_title_mode").is_none(),
        "set must never write title_mode (DEC-044)"
    );
    assert_eq!(
        view.title_mode, "redacted",
        "the view marker is always the fixed redacted value"
    );
}

#[test]
fn settings_api_default_view_is_capture_off_and_redacted() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _env = remove_capture_env();
    let c = conn();
    let view = settings_api::resolve_view(&c, "2026-06-21 09:00:00").unwrap();
    assert!(!view.capture_enabled, "capture defaults OFF");
    assert_eq!(view.title_mode, "redacted");
    assert_eq!(view.sample_seconds, config::DEFAULT_SAMPLE_SECONDS);
    assert_eq!(view.retention_days, config::DEFAULT_RETENTION_DAYS);
    assert_eq!(view.platform_supported, cfg!(target_os = "macos"));
}

#[test]
fn settings_api_view_embeds_status_snapshot() {
    let _guard = ENV_LOCK.lock().unwrap();
    let _env = remove_capture_env();
    let c = conn();
    store::insert_raw_observation(
        &c,
        &raw_obs("2026-06-21 09:00:00", "2026-06-21", None),
        TitleMode::Redacted,
        "2026-06-21 09:00:00",
    )
    .unwrap();
    let view = settings_api::resolve_view(&c, "2026-06-21 09:00:00").unwrap();
    assert_eq!(view.status.samples_today, 1);
    assert_eq!(
        view.status.last_sample_ts.as_deref(),
        Some("2026-06-21 09:00:00")
    );
}
