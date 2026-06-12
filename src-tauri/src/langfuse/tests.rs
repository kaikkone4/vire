//! Importer unit tests against an in-memory mock `LangfuseApi` — no network, no credentials,
//! no container (the feeder probe's `--mock` style). Covers pagination/dedup, all ten health
//! transitions, the absence-≠-zero invariant, credential redaction, the loopback/Cloud URL
//! allowlist, observation-sourced usage/cost, and read-only (no raw-activity-egress) behavior.

use std::cell::RefCell;
use std::collections::HashMap;

use rusqlite::Connection;
use serde_json::{json, Value};

use super::api::LangfuseApi;
use super::config::{ApiPath, Credentials, ImporterConfig, Secret, Source, DEFAULT_BASE_URL};
use super::importer::run_import;
use super::model::{
    ApiError, ApiErrorKind, HealthState, ImportWindow, Observation, TracePage,
};
use super::store;

// ----- mock --------------------------------------------------------------------------------

#[derive(Default)]
struct MockApi {
    probe: Option<ApiError>,
    pages: HashMap<String, Vec<Vec<Value>>>,
    observations: HashMap<String, Vec<Observation>>,
    traces_err: HashMap<String, ApiError>,
    calls: RefCell<Vec<String>>,
}

impl MockApi {
    fn with_pages(env: &str, pages: Vec<Vec<Value>>) -> Self {
        let mut m = MockApi::default();
        m.pages.insert(env.to_string(), pages);
        m
    }
}

impl LangfuseApi for MockApi {
    fn probe(&self) -> Result<(), ApiError> {
        self.calls.borrow_mut().push("probe".into());
        match &self.probe {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    fn get_traces(
        &self,
        environment: &str,
        _from: &str,
        _to: &str,
        page: u32,
        limit: u32,
    ) -> Result<TracePage, ApiError> {
        self.calls
            .borrow_mut()
            .push(format!("get_traces:{environment}:{page}"));
        if let Some(e) = self.traces_err.get(environment) {
            return Err(e.clone());
        }
        let pages = self.pages.get(environment).cloned().unwrap_or_default();
        let total_pages = pages.len() as u32;
        let data = pages.get((page.saturating_sub(1)) as usize).cloned().unwrap_or_default();
        Ok(TracePage {
            data,
            meta: super::model::PageMeta {
                page,
                limit,
                total_items: 0,
                total_pages,
            },
        })
    }

    fn get_observations(&self, trace_id: &str) -> Result<Vec<Observation>, ApiError> {
        self.calls
            .borrow_mut()
            .push(format!("get_observations:{trace_id}"));
        Ok(self.observations.get(trace_id).cloned().unwrap_or_default())
    }
}

// ----- helpers -----------------------------------------------------------------------------

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    store::migrate(&c).unwrap();
    c
}

fn local_vire() -> ImporterConfig {
    ImporterConfig::new(DEFAULT_BASE_URL, Source::Local, vec!["vire".into()], None)
}

fn window() -> ImportWindow {
    ImportWindow {
        from: "2026-06-01T00:00:00Z".into(),
        to: "2026-06-10T00:00:00Z".into(),
    }
}

/// A trace with an embedded generation observation carrying usage + cost.
fn trace_with_generation(id: &str, env: &str, ts: &str, cost: f64, total_tokens: i64) -> Value {
    json!({
        "id": id,
        "environment": env,
        "timestamp": ts,
        "name": "claude-code",
        "sessionId": null,
        "totalCost": 999.0,
        "metadata": {},
        "observations": [{
            "type": "GENERATION",
            "model": "claude",
            "startTime": ts,
            "endTime": ts,
            "promptTokens": total_tokens / 2,
            "completionTokens": total_tokens / 2,
            "totalTokens": total_tokens,
            "usage": {"input": total_tokens / 2, "output": total_tokens / 2, "total": total_tokens, "unit": "TOKENS"},
            "calculatedTotalCost": cost
        }]
    })
}

/// A time-only trace (pi-observe style): no generation observation, so no AI usage/cost.
fn trace_time_only(id: &str, env: &str, ts: &str) -> Value {
    json!({
        "id": id,
        "environment": env,
        "timestamp": ts,
        "name": "",
        "sessionId": null,
        "metadata": {"project_key": "vire"},
        "observations": []
    })
}

fn evidence_cost(c: &Connection, env: &str, trace_id: &str) -> Option<f64> {
    c.query_row(
        "SELECT cost_total FROM langfuse_ai_evidence WHERE environment=?1 AND trace_id=?2",
        rusqlite::params![env, trace_id],
        |r| r.get::<_, Option<f64>>(0),
    )
    .unwrap()
}

// ----- pagination + dedup ------------------------------------------------------------------

#[test]
fn paginates_to_completion_and_dedups_cross_page() {
    let c = conn();
    // page1: [A, B], page2: [B (dup), C]
    let api = MockApi::with_pages(
        "vire",
        vec![
            vec![
                trace_with_generation("A", "vire", "2026-06-02T00:00:00Z", 1.0, 10),
                trace_with_generation("B", "vire", "2026-06-03T00:00:00Z", 2.0, 20),
            ],
            vec![
                trace_with_generation("B", "vire", "2026-06-03T00:00:00Z", 2.0, 20),
                trace_with_generation("C", "vire", "2026-06-04T00:00:00Z", 3.0, 30),
            ],
        ],
    );
    let summaries = run_import(&api, &c, &local_vire(), &window());
    let vire = summaries.iter().find(|s| s.environment == "vire").unwrap();
    assert_eq!(vire.pages, 2);
    assert_eq!(vire.unique, 3, "A,B,C counted once");
    assert_eq!(vire.duplicates, 1, "the cross-page B is suppressed");
    assert_eq!(vire.health, HealthState::Healthy);
    assert_eq!(vire.cursor_ts.as_deref(), Some("2026-06-04T00:00:00Z"));
}

// ----- the ten health states ---------------------------------------------------------------

#[test]
fn healthy_when_generations_have_usage_and_cost() {
    let c = conn();
    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation("A", "vire", "2026-06-05T00:00:00Z", 1.5, 12)]],
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    assert_eq!(s[0].health, HealthState::Healthy);
    assert_eq!(evidence_cost(&c, "vire", "A"), Some(1.5));
}

#[test]
fn missing_when_environment_is_empty_with_no_prior_cursor() {
    let c = conn();
    let api = MockApi::default(); // vire returns nothing
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(vire.health, HealthState::Missing);
    assert_eq!(vire.cursor_ts, None);
}

#[test]
fn stale_when_only_an_old_cursor_remains() {
    let c = conn();
    // First import sets an old cursor (well before window.to - 24h).
    let api1 = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation("A", "vire", "2026-06-01T00:00:00Z", 1.0, 4)]],
    );
    run_import(&api1, &c, &local_vire(), &window());
    // Second import finds nothing new; the carried cursor is now stale vs window.to.
    let api2 = MockApi::default();
    let s = run_import(&api2, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(vire.health, HealthState::Stale);
    assert_eq!(vire.cursor_ts.as_deref(), Some("2026-06-01T00:00:00Z"));
}

#[test]
fn wrong_env_when_traffic_lands_in_default() {
    let c = conn();
    let mut api = MockApi::default();
    api.pages.insert(
        "default".into(),
        vec![vec![trace_with_generation("D", "default", "2026-06-05T00:00:00Z", 2.0, 8)]],
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    let wrong = s.iter().find(|x| x.environment == "default").unwrap();
    assert_eq!(wrong.health, HealthState::WrongEnv);
    // Surfaced for review with wrong_env health, not folded into the vire total.
    let health: String = c
        .query_row(
            "SELECT health FROM langfuse_ai_evidence WHERE trace_id='D'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(health, "wrong_env");
}

#[test]
fn delayed_when_a_trace_predates_the_checkpoint() {
    let c = conn();
    let api1 = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation("A", "vire", "2026-06-06T00:00:00Z", 1.0, 4)]],
    );
    run_import(&api1, &c, &local_vire(), &window());
    // A new, older trace arrives after the checkpoint was set at 06-06.
    let api2 = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation("Z", "vire", "2026-06-02T00:00:00Z", 1.0, 4)]],
    );
    let s = run_import(&api2, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(vire.health, HealthState::Delayed);
    // The checkpoint did not regress to the older trace.
    assert_eq!(vire.cursor_ts.as_deref(), Some("2026-06-06T00:00:00Z"));
}

#[test]
fn duplicate_when_a_reimport_suppresses_everything() {
    let c = conn();
    let pages = vec![vec![trace_with_generation("A", "vire", "2026-06-05T00:00:00Z", 1.0, 4)]];
    let api = MockApi::with_pages("vire", pages.clone());
    run_import(&api, &c, &local_vire(), &window()); // first: healthy
    let api2 = MockApi::with_pages("vire", pages);
    let s = run_import(&api2, &c, &local_vire(), &window()); // second: all suppressed
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(vire.health, HealthState::Duplicate);
    assert_eq!(vire.unique, 0);
    assert_eq!(vire.duplicates, 1);
}

#[test]
fn schema_changed_when_generation_lacks_usage_and_cost() {
    let c = conn();
    let trace = json!({
        "id": "A", "environment": "vire", "timestamp": "2026-06-05T00:00:00Z",
        "metadata": {},
        "observations": [{"type": "GENERATION", "model": null}]
    });
    let api = MockApi::with_pages("vire", vec![vec![trace]]);
    let s = run_import(&api, &c, &local_vire(), &window());
    assert_eq!(s[0].health, HealthState::SchemaChanged);
    // The unmapped/empty generation does NOT produce a zero cost.
    assert_eq!(evidence_cost(&c, "vire", "A"), None);
}

#[test]
fn schema_changed_when_a_trace_is_unparseable() {
    let c = conn();
    let trace = json!({ "environment": "vire" }); // no id → fails to parse
    let api = MockApi::with_pages("vire", vec![vec![trace]]);
    let s = run_import(&api, &c, &local_vire(), &window());
    assert_eq!(s[0].health, HealthState::SchemaChanged);
}

#[test]
fn auth_or_network_error_on_probe_auth_failure() {
    let c = conn();
    let api = MockApi {
        probe: Some(ApiError::new(ApiErrorKind::Auth, "auth error")),
        ..Default::default()
    };
    let s = run_import(&api, &c, &local_vire(), &window());
    assert_eq!(s[0].health, HealthState::AuthOrNetworkError);
    assert!(s[0].evidence.is_empty(), "no evidence rows when auth fails");
}

#[test]
fn auth_or_network_error_on_rate_limit_during_listing() {
    let c = conn();
    let mut api = MockApi::default();
    api.traces_err
        .insert("vire".into(), ApiError::new(ApiErrorKind::RateLimited, "429"));
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(vire.health, HealthState::AuthOrNetworkError);
}

#[test]
fn unavailable_when_stack_is_down() {
    let c = conn();
    let api = MockApi {
        probe: Some(ApiError::new(ApiErrorKind::Unavailable, "connection refused")),
        ..Default::default()
    };
    let s = run_import(&api, &c, &local_vire(), &window());
    assert_eq!(s[0].health, HealthState::Unavailable);
}

#[test]
fn unknown_when_response_is_indeterminate() {
    let c = conn();
    let mut api = MockApi::default();
    api.traces_err.insert(
        "vire".into(),
        ApiError::new(ApiErrorKind::Indeterminate, "bad envelope"),
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(vire.health, HealthState::Unknown);
}

// ----- absence ≠ zero ----------------------------------------------------------------------

#[test]
fn absence_is_never_zero_cost_when_stack_down() {
    let c = conn();
    let api = MockApi {
        probe: Some(ApiError::new(ApiErrorKind::Unavailable, "down")),
        ..Default::default()
    };
    run_import(&api, &c, &local_vire(), &window());
    // No AI-evidence rows at all → no zero cost manufactured.
    let rows: i64 = c
        .query_row("SELECT COUNT(*) FROM langfuse_ai_evidence", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0);
    let snapshot = store::source_health_snapshot(&c, &local_vire()).unwrap();
    assert_eq!(snapshot.health, "unavailable");
}

#[test]
fn time_only_trace_has_null_cost_not_zero() {
    let c = conn();
    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_time_only("A", "vire", "2026-06-05T00:00:00Z")]],
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    assert_eq!(s[0].health, HealthState::Healthy);
    assert_eq!(
        evidence_cost(&c, "vire", "A"),
        None,
        "a trace with no generation observation has unknown, not zero, cost"
    );
}

// ----- usage/cost come from observations, not the trace body -------------------------------

#[test]
fn cost_is_read_from_observations_not_trace_body() {
    let c = conn();
    // Trace body says 999.0; the generation observation says 1.5 — the observation wins.
    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation("A", "vire", "2026-06-05T00:00:00Z", 1.5, 10)]],
    );
    run_import(&api, &c, &local_vire(), &window());
    assert_eq!(evidence_cost(&c, "vire", "A"), Some(1.5));
}

#[test]
fn observations_are_fetched_when_not_embedded() {
    let c = conn();
    let mut api = MockApi::with_pages(
        "vire",
        vec![vec![trace_time_only("A", "vire", "2026-06-05T00:00:00Z")]],
    );
    // The list payload had no embedded observations; the importer fetches them.
    api.observations.insert(
        "A".into(),
        vec![Observation {
            obs_type: Some("GENERATION".into()),
            model: Some("claude".into()),
            start_time: None,
            end_time: None,
            prompt_tokens: Some(4),
            completion_tokens: Some(6),
            total_tokens: Some(10),
            usage: None,
            calculated_total_cost: Some(2.25),
        }],
    );
    run_import(&api, &c, &local_vire(), &window());
    assert_eq!(evidence_cost(&c, "vire", "A"), Some(2.25));
    assert!(api
        .calls
        .borrow()
        .iter()
        .any(|call| call == "get_observations:A"));
}

// ----- read-only / no raw-activity egress --------------------------------------------------

#[test]
fn importer_only_issues_read_calls() {
    let c = conn();
    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_time_only("A", "vire", "2026-06-05T00:00:00Z")]],
    );
    run_import(&api, &c, &local_vire(), &window());
    // The trait exposes only probe/get_traces/get_observations — there is no write/push path.
    for call in api.calls.borrow().iter() {
        assert!(
            call == "probe"
                || call.starts_with("get_traces:")
                || call.starts_with("get_observations:"),
            "unexpected non-read call: {call}"
        );
    }
}

// ----- credential redaction ----------------------------------------------------------------

#[test]
fn secret_and_credentials_never_render_their_values() {
    let secret = Secret::new("sk-lf-supersecret-value");
    assert_eq!(format!("{secret:?}"), "***redacted***");

    let creds = Credentials {
        public_key: "pk-lf-public-id".into(),
        secret_key: Secret::new("sk-lf-supersecret-value"),
    };
    let rendered = format!("{creds:?}");
    assert!(!rendered.contains("supersecret"));
    assert!(!rendered.contains("pk-lf-public-id"));

    let config = ImporterConfig::new(
        DEFAULT_BASE_URL,
        Source::Local,
        vec!["vire".into()],
        Some(creds),
    );
    let rendered = format!("{config:?}");
    assert!(!rendered.contains("supersecret"));
}

#[test]
fn import_run_table_has_no_credential_columns() {
    let c = conn();
    let mut stmt = c.prepare("PRAGMA table_info(langfuse_import_runs)").unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for col in &cols {
        let lc = col.to_ascii_lowercase();
        assert!(
            !(lc.contains("secret") || lc.contains("token") || lc.contains("password") || lc.contains("key")),
            "import_runs must not have a credential-bearing column, found {col}"
        );
    }
}

// ----- loopback default + Cloud-explicit-override allowlist --------------------------------

#[test]
fn loopback_is_the_default_and_allowed_for_local() {
    let config = local_vire();
    assert_eq!(config.base_url, "http://127.0.0.1:3000");
    assert_eq!(config.source, Source::Local);
    assert!(config.validate_target().is_ok());
    let url = config
        .build_url(&ApiPath::Traces {
            environment: "vire",
            from: "2026-06-01T00:00:00Z",
            to: "2026-06-10T00:00:00Z",
            page: 1,
            limit: 50,
        })
        .unwrap();
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(url.scheme(), "http");
    assert!(url.path().starts_with("/api/public/traces"));
    assert!(url.query().unwrap().contains("environment=vire"));
}

#[test]
fn local_source_refuses_off_host_targets() {
    let config = ImporterConfig::new(
        "http://example.com:3000",
        Source::Local,
        vec!["vire".into()],
        None,
    );
    assert!(config.validate_target().is_err());
    assert!(config.build_url(&ApiPath::Health).is_err());
}

#[test]
fn cloud_is_an_explicit_override_for_off_host_only() {
    // Cloud override reaches an off-host endpoint...
    let cloud = ImporterConfig::new(
        "https://cloud.langfuse.com",
        Source::Cloud,
        vec!["vire".into()],
        None,
    );
    assert!(cloud.validate_target().is_ok());
    let url = cloud.build_url(&ApiPath::Health).unwrap();
    assert_eq!(url.host_str(), Some("cloud.langfuse.com"));
    assert_eq!(url.scheme(), "https");
    // ...and Cloud pointed at loopback is rejected (it must be a deliberate off-host target).
    let bad = ImporterConfig::new(DEFAULT_BASE_URL, Source::Cloud, vec!["vire".into()], None);
    assert!(bad.validate_target().is_err());
}

#[test]
fn non_http_scheme_is_refused() {
    let config = ImporterConfig::new("ftp://127.0.0.1:3000", Source::Local, vec!["vire".into()], None);
    assert!(config.parsed_base().is_err());
    assert!(config.build_url(&ApiPath::Health).is_err());
}

// ----- snapshot ----------------------------------------------------------------------------

#[test]
fn snapshot_is_unknown_before_any_import() {
    let c = conn();
    let snap = store::source_health_snapshot(&c, &local_vire()).unwrap();
    assert_eq!(snap.health, "unknown");
    assert_eq!(snap.base_url, "http://127.0.0.1:3000");
    assert_eq!(snap.source, "local");
    assert!(snap.last_import_at.is_none());
}

// ----- S-3: atomic persistence -------------------------------------------------------------

/// A mid-run write failure (forced via a SQLite trigger that aborts the AI-evidence insert *after*
/// the raw-trace rows were written in the same transaction) must leave NO partial state: the
/// transaction rolls back both the raw and evidence writes. (S-3 + S-4.)
#[test]
fn persistence_failure_mid_run_leaves_no_partial_state_and_is_surfaced() {
    let c = conn();
    c.execute_batch(
        "CREATE TRIGGER force_evidence_failure BEFORE INSERT ON langfuse_ai_evidence \
         BEGIN SELECT RAISE(ABORT, 'forced test failure'); END;",
    )
    .unwrap();
    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation("A", "vire", "2026-06-05T00:00:00Z", 1.5, 12)]],
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();

    // S-3: the run's transaction rolled back — neither the raw row nor the evidence row survived.
    let raw: i64 = c
        .query_row("SELECT COUNT(*) FROM langfuse_raw_traces", [], |r| r.get(0))
        .unwrap();
    let ev: i64 = c
        .query_row("SELECT COUNT(*) FROM langfuse_ai_evidence", [], |r| r.get(0))
        .unwrap();
    assert_eq!(raw, 0, "raw-trace rows must roll back with the failed run");
    assert_eq!(ev, 0, "AI-evidence rows must never be committed for a failed run");

    // S-4: the failure is surfaced (non-healthy + secret-free warning), never healthy, never zero.
    assert_eq!(vire.health, HealthState::Unknown);
    assert!(vire.warnings.iter().any(|w| w.contains("persist")));
    assert!(
        !vire.warnings.iter().any(|w| w.contains("sk-") || w.contains("forced test failure")),
        "surfaced warning must be secret-free and must not echo the raw driver string"
    );
    let snap = store::source_health_snapshot(&c, &local_vire()).unwrap();
    assert_ne!(snap.health, "healthy", "a failed persist must not read as healthy");
}

/// A run that persists without error commits its raw rows, evidence rows, and run record together,
/// all keyed to one consistent `run_id`, and becomes visible to the read-only snapshot. (S-3.)
#[test]
fn successful_run_commits_as_one_consistent_unit() {
    let c = conn();
    let api = MockApi::with_pages(
        "vire",
        vec![vec![
            trace_with_generation("A", "vire", "2026-06-05T00:00:00Z", 1.5, 12),
            trace_with_generation("B", "vire", "2026-06-06T00:00:00Z", 2.0, 20),
        ]],
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    assert_eq!(
        s.iter().find(|x| x.environment == "vire").unwrap().health,
        HealthState::Healthy
    );
    let raw: i64 = c
        .query_row("SELECT COUNT(*) FROM langfuse_raw_traces", [], |r| r.get(0))
        .unwrap();
    let ev: i64 = c
        .query_row("SELECT COUNT(*) FROM langfuse_ai_evidence", [], |r| r.get(0))
        .unwrap();
    assert_eq!(raw, 2);
    assert_eq!(ev, 2);
    // The raw rows, evidence rows, and run record all share one run id.
    let run_id: String = c
        .query_row(
            "SELECT id FROM langfuse_import_runs WHERE status='healthy'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let raw_run: String = c
        .query_row("SELECT DISTINCT import_run_id FROM langfuse_raw_traces", [], |r| r.get(0))
        .unwrap();
    let ev_run: String = c
        .query_row("SELECT DISTINCT import_run_id FROM langfuse_ai_evidence", [], |r| r.get(0))
        .unwrap();
    assert_eq!(raw_run, run_id);
    assert_eq!(ev_run, run_id);
    let snap = store::source_health_snapshot(&c, &local_vire()).unwrap();
    assert_eq!(snap.health, "healthy");
}

// ----- S-5: uniform UTC RFC3339 timestamps -------------------------------------------------

#[test]
fn importer_emitted_timestamps_are_utc_rfc3339() {
    let c = conn();
    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation("A", "vire", "2026-06-05T00:00:00Z", 1.5, 12)]],
    );
    run_import(&api, &c, &local_vire(), &window());
    let (started, finished): (String, String) = c
        .query_row(
            "SELECT started_at, finished_at FROM langfuse_import_runs LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let imported: String = c
        .query_row("SELECT imported_at FROM langfuse_raw_traces LIMIT 1", [], |r| r.get(0))
        .unwrap();
    for ts in [&started, &finished, &imported] {
        assert!(
            chrono::DateTime::parse_from_rfc3339(ts).is_ok(),
            "importer timestamp {ts} must parse as RFC3339"
        );
        assert!(ts.ends_with('Z'), "importer timestamp {ts} must be UTC (trailing Z)");
        assert!(!ts.contains(' '), "importer timestamp {ts} must not be a space-separated local time");
    }
}

/// The documented ordering-key migration: an RFC3339 `…T…Z` row always out-sorts a legacy
/// space-separated row of the *same day*, even when the legacy local time is numerically later,
/// because `'T'` (0x54) > `' '` (0x20). So "latest run" stays correct across the format transition
/// with no data migration. (S-5.)
#[test]
fn rfc3339_run_sorts_after_legacy_space_format_run_same_day() {
    let c = conn();
    let legacy = store::ImportRunRecord {
        id: "legacy".into(),
        environment: "vire".into(),
        window_from: None,
        window_to: None,
        cursor_ts: None,
        status: HealthState::Healthy,
        pages_walked: 0,
        traces_seen: 0,
        duplicates_suppressed: 0,
        warnings: vec![],
        started_at: "2026-06-12 23:59:59".into(),
        finished_at: "2026-06-12 23:59:59".into(),
    };
    let modern = store::ImportRunRecord {
        id: "modern".into(),
        started_at: "2026-06-12T00:00:01Z".into(),
        finished_at: "2026-06-12T00:00:01Z".into(),
        ..legacy.clone()
    };
    store::insert_import_run(&c, &legacy).unwrap();
    store::insert_import_run(&c, &modern).unwrap();
    let latest = store::latest_run(&c).unwrap().unwrap();
    assert_eq!(
        latest.id, "modern",
        "an RFC3339 run must sort latest over a legacy space-format run even when the legacy local time is numerically later"
    );
}
