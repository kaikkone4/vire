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
use super::importer::{
    incremental_window, run_backfill, run_import, run_import_with, ImportReport,
};
use super::model::{ApiError, ApiErrorKind, HealthState, ImportWindow, Observation, TracePage};
use super::store;
use super::ImportRange;

// ----- mock --------------------------------------------------------------------------------

#[derive(Default)]
struct MockApi {
    probe: Option<ApiError>,
    pages: HashMap<String, Vec<Vec<Value>>>,
    observations: HashMap<String, Vec<Observation>>,
    traces_err: HashMap<String, ApiError>,
    /// Pages returned by the discovery scan (`get_traces_any_env`) — the no-environment-filter
    /// view that spans every environment. Independent of `pages` so a test can model a list that
    /// crosses environments without the per-environment filter.
    any_env_pages: Vec<Vec<Value>>,
    /// Optional error for the discovery scan, to model a failed (best-effort) discovery.
    any_env_err: Option<ApiError>,
    calls: RefCell<Vec<String>>,
    /// Records the `(environment, from, to)` window of every `get_traces` call, so a test can assert
    /// which window the importer resolved (incremental cursor / backfill chunk) — the mock otherwise
    /// ignores the window when selecting pages (TASK-029 C).
    trace_windows: RefCell<Vec<(String, String, String)>>,
    /// When set, every `get_traces` page returns this single trace and reports an effectively
    /// unbounded `total_pages`, so a window runs until it hits the importer's `MAX_PAGES` backstop —
    /// used to prove the bounded-run (`reached_page_limit`) surfacing (TASK-029 C4).
    infinite_pages: Option<Value>,
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
        from: &str,
        to: &str,
        page: u32,
        limit: u32,
        _order_by: &str,
    ) -> Result<TracePage, ApiError> {
        self.calls
            .borrow_mut()
            .push(format!("get_traces:{environment}:{page}"));
        self.trace_windows.borrow_mut().push((
            environment.to_string(),
            from.to_string(),
            to.to_string(),
        ));
        if let Some(e) = self.traces_err.get(environment) {
            return Err(e.clone());
        }
        if let Some(trace) = &self.infinite_pages {
            return Ok(TracePage {
                data: vec![trace.clone()],
                meta: super::model::PageMeta {
                    page,
                    limit,
                    total_items: 0,
                    total_pages: u32::MAX,
                },
            });
        }
        let pages = self.pages.get(environment).cloned().unwrap_or_default();
        let total_pages = pages.len() as u32;
        let data = pages
            .get((page.saturating_sub(1)) as usize)
            .cloned()
            .unwrap_or_default();
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

    fn get_traces_any_env(
        &self,
        _from: &str,
        _to: &str,
        page: u32,
        limit: u32,
    ) -> Result<TracePage, ApiError> {
        self.calls
            .borrow_mut()
            .push(format!("get_traces_any_env:{page}"));
        if let Some(e) = &self.any_env_err {
            return Err(e.clone());
        }
        let total_pages = self.any_env_pages.len() as u32;
        let data = self
            .any_env_pages
            .get((page.saturating_sub(1)) as usize)
            .cloned()
            .unwrap_or_default();
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
        vec![vec![trace_with_generation(
            "A",
            "vire",
            "2026-06-05T00:00:00Z",
            1.5,
            12,
        )]],
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
        vec![vec![trace_with_generation(
            "A",
            "vire",
            "2026-06-01T00:00:00Z",
            1.0,
            4,
        )]],
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
        vec![vec![trace_with_generation(
            "D",
            "default",
            "2026-06-05T00:00:00Z",
            2.0,
            8,
        )]],
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
        vec![vec![trace_with_generation(
            "A",
            "vire",
            "2026-06-06T00:00:00Z",
            1.0,
            4,
        )]],
    );
    run_import(&api1, &c, &local_vire(), &window());
    // A new, older trace arrives after the checkpoint was set at 06-06.
    let api2 = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation(
            "Z",
            "vire",
            "2026-06-02T00:00:00Z",
            1.0,
            4,
        )]],
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
    let pages = vec![vec![trace_with_generation(
        "A",
        "vire",
        "2026-06-05T00:00:00Z",
        1.0,
        4,
    )]];
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
    api.traces_err.insert(
        "vire".into(),
        ApiError::new(ApiErrorKind::RateLimited, "429"),
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(vire.health, HealthState::AuthOrNetworkError);
}

#[test]
fn unavailable_when_stack_is_down() {
    let c = conn();
    let api = MockApi {
        probe: Some(ApiError::new(
            ApiErrorKind::Unavailable,
            "connection refused",
        )),
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
        .query_row("SELECT COUNT(*) FROM langfuse_ai_evidence", [], |r| {
            r.get(0)
        })
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
        vec![vec![trace_with_generation(
            "A",
            "vire",
            "2026-06-05T00:00:00Z",
            1.5,
            10,
        )]],
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
            usage_details: None,
            cost_details: None,
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
    let mut stmt = c
        .prepare("PRAGMA table_info(langfuse_import_runs)")
        .unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for col in &cols {
        let lc = col.to_ascii_lowercase();
        assert!(
            !(lc.contains("secret")
                || lc.contains("token")
                || lc.contains("password")
                || lc.contains("key")),
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
            order_by: super::importer::TRACES_ORDER_BY,
        })
        .unwrap();
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(url.scheme(), "http");
    assert!(url.path().starts_with("/api/public/traces"));
    assert!(url.query().unwrap().contains("environment=vire"));
    // DEC-032: the trace-page request orders explicitly oldest → newest (the undocumented default
    // sort is never relied on), so the inclusive-`fromTimestamp` resume-cursor walks forward.
    assert!(
        url.query().unwrap().contains("orderBy=timestamp.asc"),
        "trace requests must pin orderBy=timestamp.asc (DEC-032), got {:?}",
        url.query()
    );
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
    let config = ImporterConfig::new(
        "ftp://127.0.0.1:3000",
        Source::Local,
        vec!["vire".into()],
        None,
    );
    assert!(config.parsed_base().is_err());
    assert!(config.build_url(&ApiPath::Health).is_err());
}

// ----- TASK-022 interface extension: session_id on evidence --------------------------------

/// The privacy-positive importer extension: a trace's hashed `sessionId` is surfaced onto the
/// normalized `langfuse_ai_evidence` row so the runtime observer matches the normalized row, never
/// the prompt-bearing raw payload. Absent on a trace ⇒ `NULL`, never a fabricated value.
#[test]
fn session_id_is_surfaced_onto_normalized_evidence() {
    let c = conn();
    let with_session = json!({
        "id": "WS", "environment": "vire", "timestamp": "2026-06-05T00:00:00Z",
        "sessionId": "session-abc123", "metadata": {},
        "observations": [{"type": "GENERATION", "startTime": "2026-06-05T00:00:00Z",
            "endTime": "2026-06-05T00:01:00Z", "totalTokens": 10, "calculatedTotalCost": 1.0}]
    });
    let api = MockApi::with_pages(
        "vire",
        vec![vec![
            with_session,
            trace_time_only("NS", "vire", "2026-06-05T00:02:00Z"),
        ]],
    );
    run_import(&api, &c, &local_vire(), &window());
    let ws: Option<String> = c
        .query_row(
            "SELECT session_id FROM langfuse_ai_evidence WHERE trace_id='WS'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let ns: Option<String> = c
        .query_row(
            "SELECT session_id FROM langfuse_ai_evidence WHERE trace_id='NS'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        ws.as_deref(),
        Some("session-abc123"),
        "present sessionId is surfaced"
    );
    assert_eq!(ns, None, "absent sessionId stays NULL, never fabricated");
}

// ----- TASK-026: Test connection verdict is coarse and secret-free -------------------------

fn assert_test_result_secret_free(result: &super::TestConnectionResult) {
    let serialized = serde_json::to_string(result).unwrap();
    for needle in [
        "sk-",
        "pk-",
        "canary",
        "Bearer",
        "Authorization",
        "password",
        "leak",
    ] {
        assert!(
            !serialized.contains(needle),
            "test-connection result must be secret-free, found {needle}"
        );
    }
}

#[test]
fn test_connection_refuses_non_loopback_local_without_a_network_call() {
    // local + off-host fails `validate_target` inside `ReqwestLangfuseApi::new`, so the verdict is
    // produced with no socket opened (deterministic in CI).
    let config = ImporterConfig::new(
        "http://example.com:3000",
        Source::Local,
        vec!["vire".into()],
        Some(Credentials {
            public_key: "pk-canary".into(),
            secret_key: Secret::new("sk-canary-secret"),
        }),
    );
    let result = super::test_connection(config);
    assert!(!result.ok);
    assert_eq!(result.verdict, "invalid_config");
    assert_test_result_secret_free(&result);
}

#[test]
fn test_connection_verdicts_are_coarse_and_never_echo_the_error_message() {
    use super::TestConnectionResult;
    // The source ApiError message is deliberately secret-shaped; the verdict must NOT echo it.
    let cases = [
        (ApiErrorKind::Unavailable, "unavailable"),
        (ApiErrorKind::Auth, "auth_or_network_error"),
        (ApiErrorKind::RateLimited, "auth_or_network_error"),
        (ApiErrorKind::Network, "auth_or_network_error"),
        (ApiErrorKind::Indeterminate, "unknown"),
    ];
    for (kind, expected) in cases {
        let error = ApiError::new(
            kind,
            "sk-leak-canary Authorization: Bearer pk-leak password",
        );
        let verdict = TestConnectionResult::from_api_error(&error);
        assert!(!verdict.ok);
        assert_eq!(verdict.verdict, expected);
        assert_test_result_secret_free(&verdict);
    }
    let reachable = TestConnectionResult::reachable();
    assert!(reachable.ok);
    assert_eq!(reachable.verdict, "reachable");
    assert_test_result_secret_free(&reachable);
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
        vec![vec![trace_with_generation(
            "A",
            "vire",
            "2026-06-05T00:00:00Z",
            1.5,
            12,
        )]],
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();

    // S-3: the run's transaction rolled back — neither the raw row nor the evidence row survived.
    let raw: i64 = c
        .query_row("SELECT COUNT(*) FROM langfuse_raw_traces", [], |r| r.get(0))
        .unwrap();
    let ev: i64 = c
        .query_row("SELECT COUNT(*) FROM langfuse_ai_evidence", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(raw, 0, "raw-trace rows must roll back with the failed run");
    assert_eq!(
        ev, 0,
        "AI-evidence rows must never be committed for a failed run"
    );

    // S-4: the failure is surfaced (non-healthy + secret-free warning), never healthy, never zero.
    assert_eq!(vire.health, HealthState::Unknown);
    assert!(vire.warnings.iter().any(|w| w.contains("persist")));
    assert!(
        !vire
            .warnings
            .iter()
            .any(|w| w.contains("sk-") || w.contains("forced test failure")),
        "surfaced warning must be secret-free and must not echo the raw driver string"
    );
    let snap = store::source_health_snapshot(&c, &local_vire()).unwrap();
    assert_ne!(
        snap.health, "healthy",
        "a failed persist must not read as healthy"
    );
}

/// TASK-021 — the both-writes-fail gap. When the run transaction fails **and** the durable
/// failure-marker insert fails under the same fault (a trigger aborts every write to
/// `langfuse_import_runs`), the persist failure must still surface to the import command **in-band**,
/// never as the prior, durably-persisted `healthy` snapshot. The marker-only S-4 channel could not
/// cover this because the marker write is precisely what the fault breaks.
#[test]
fn persist_failure_surfaces_in_band_even_when_marker_write_also_fails() {
    let c = conn();

    // Seed a prior, durably-persisted healthy run. If the failed import fell back to the DB snapshot,
    // it would read this stale `healthy` — the exact false-healthy the fix must prevent.
    let seed = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation(
            "SEED",
            "vire",
            "2026-06-09T00:00:00Z",
            1.0,
            8,
        )]],
    );
    run_import(&seed, &c, &local_vire(), &window());
    assert_eq!(
        store::source_health_snapshot(&c, &local_vire())
            .unwrap()
            .health,
        "healthy",
        "precondition: a prior healthy run is durably persisted"
    );

    // Make EVERY insert into langfuse_import_runs abort, so both persist_import_run's run-record
    // insert AND the best-effort marker insert fail under one fault. The RAISE message embeds a
    // secret-shaped token to prove the surfaced error never echoes the raw driver string.
    c.execute_batch(
        "CREATE TRIGGER force_import_run_failure BEFORE INSERT ON langfuse_import_runs \
         BEGIN SELECT RAISE(ABORT, 'forced failure sk-leak-canary token'); END;",
    )
    .unwrap();

    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation(
            "A",
            "vire",
            "2026-06-10T00:00:00Z",
            1.5,
            12,
        )]],
    );
    let summaries = run_import(&api, &c, &local_vire(), &window());
    let vire = summaries.iter().find(|s| s.environment == "vire").unwrap();

    // The in-memory summary degraded to a non-healthy Unknown carrying the persist sentinel.
    assert_eq!(vire.health, HealthState::Unknown);
    assert!(
        vire.warnings
            .iter()
            .any(|w| w == super::importer::PERSIST_FAILURE_MSG),
        "the persist-failure sentinel must be present so the in-band path can key on it"
    );

    // In-band surfacing: the import command's result is a secret-free Err — the fault-independent
    // channel — NOT the stale healthy snapshot.
    let result = super::import_result(&summaries);
    assert!(
        result.is_err(),
        "the manual import command must surface the failure in-band, never return the stale-healthy snapshot"
    );
    let err = result.expect_err("a persist failure must surface as Err, never Ok(stale snapshot)");
    for needle in [
        "sk-",
        "Bearer",
        "Authorization",
        "password",
        "token",
        "canary",
        "forced",
        "RAISE",
        "ABORT",
    ] {
        assert!(
            !err.contains(needle),
            "in-band persist error must be secret-free and not echo the driver string, found {needle}"
        );
    }

    // The durable snapshot still reads the prior healthy run — by definition, an unwritable store
    // cannot record a durable `unknown`. That is exactly why the in-band Err, not the snapshot, is
    // the authoritative surfacing channel here.
    let after = store::source_health_snapshot(&c, &local_vire()).unwrap();
    assert_eq!(
        after.health, "healthy",
        "the DB snapshot is stale-healthy under an unwritable store — proving it is NOT the surfacing channel"
    );
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
        .query_row("SELECT COUNT(*) FROM langfuse_ai_evidence", [], |r| {
            r.get(0)
        })
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
        .query_row(
            "SELECT DISTINCT import_run_id FROM langfuse_raw_traces",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let ev_run: String = c
        .query_row(
            "SELECT DISTINCT import_run_id FROM langfuse_ai_evidence",
            [],
            |r| r.get(0),
        )
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
        vec![vec![trace_with_generation(
            "A",
            "vire",
            "2026-06-05T00:00:00Z",
            1.5,
            12,
        )]],
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
        .query_row(
            "SELECT imported_at FROM langfuse_raw_traces LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    for ts in [&started, &finished, &imported] {
        assert!(
            chrono::DateTime::parse_from_rfc3339(ts).is_ok(),
            "importer timestamp {ts} must parse as RFC3339"
        );
        assert!(
            ts.ends_with('Z'),
            "importer timestamp {ts} must be UTC (trailing Z)"
        );
        assert!(
            !ts.contains(' '),
            "importer timestamp {ts} must not be a space-separated local time"
        );
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

// ----- TASK-027 A4: payload tolerance (legacy AND current Langfuse shapes) ------------------

/// A trace carrying a generation in the **current** Langfuse shape only: token usage in
/// `usageDetails` and cost in `costDetails` — NO legacy `promptTokens`/`usage`/`calculatedTotalCost`.
/// This is the shape the live v3 stack (verified at SW-2 against local 3.178.0) emits and that the
/// previous parser would have degraded to `schema_changed`.
fn trace_with_current_shape(
    id: &str,
    env: &str,
    ts: &str,
    total_cost: f64,
    total_tokens: i64,
) -> Value {
    json!({
        "id": id,
        "environment": env,
        "timestamp": ts,
        "name": "claude-code",
        "sessionId": null,
        "metadata": {},
        "observations": [{
            "type": "GENERATION",
            "model": "claude",
            "startTime": ts,
            "endTime": ts,
            "usageDetails": {"input": total_tokens / 2, "output": total_tokens / 2, "total": total_tokens},
            "costDetails": {"input": total_cost / 2.0, "output": total_cost / 2.0, "total": total_cost}
        }]
    })
}

#[test]
fn observation_reads_current_usage_and_cost_details() {
    let obs: Observation = serde_json::from_value(json!({
        "type": "GENERATION",
        "usageDetails": {"input": 12, "output": 8, "total": 20},
        "costDetails": {"input": 0.1, "output": 0.2, "total": 0.3}
    }))
    .unwrap();
    assert_eq!(obs.prompt(), Some(12));
    assert_eq!(obs.completion(), Some(8));
    assert_eq!(obs.total(), Some(20));
    assert_eq!(
        obs.cost(),
        Some(0.3),
        "cost is read from costDetails['total']"
    );
    assert!(
        !obs.lacks_usage_and_cost(),
        "a current-shape generation is recognized, not schema_changed"
    );
}

#[test]
fn observation_legacy_shape_still_parses() {
    let obs: Observation = serde_json::from_value(json!({
        "type": "GENERATION",
        "promptTokens": 4, "completionTokens": 6, "totalTokens": 10,
        "usage": {"input": 4, "output": 6, "total": 10, "unit": "TOKENS"},
        "calculatedTotalCost": 1.25
    }))
    .unwrap();
    assert_eq!(obs.prompt(), Some(4));
    assert_eq!(obs.completion(), Some(6));
    assert_eq!(obs.total(), Some(10));
    assert_eq!(obs.cost(), Some(1.25));
}

#[test]
fn observation_absent_usage_stays_none_not_zero() {
    // Empty detail maps with no legacy fields → every accessor is None (absence-≠-zero) and the
    // generation degrades to schema_changed, never a fabricated 0.
    let obs: Observation = serde_json::from_value(json!({
        "type": "GENERATION", "usageDetails": {}, "costDetails": {}
    }))
    .unwrap();
    assert_eq!(obs.prompt(), None);
    assert_eq!(obs.completion(), None);
    assert_eq!(obs.total(), None);
    assert_eq!(obs.cost(), None);
    assert!(obs.lacks_usage_and_cost());
}

#[test]
fn observation_present_zero_is_distinct_from_absence() {
    // A genuine 0 in usageDetails reads as Some(0); absence in costDetails stays None.
    let obs: Observation = serde_json::from_value(json!({
        "type": "GENERATION", "usageDetails": {"input": 0, "output": 0, "total": 0}
    }))
    .unwrap();
    assert_eq!(obs.prompt(), Some(0));
    assert_eq!(obs.total(), Some(0));
    assert_eq!(
        obs.cost(),
        None,
        "no cost anywhere stays None, even when token totals are a real 0"
    );
}

#[test]
fn current_shape_generation_is_healthy_and_cost_captured() {
    let c = conn();
    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_current_shape(
            "A",
            "vire",
            "2026-06-05T00:00:00Z",
            0.6,
            20,
        )]],
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(
        vire.health,
        HealthState::Healthy,
        "current usageDetails/costDetails usage+cost must be captured, not degraded to schema_changed"
    );
    assert_eq!(evidence_cost(&c, "vire", "A"), Some(0.6));
}

#[test]
fn current_shape_with_empty_detail_maps_degrades_to_schema_changed() {
    let c = conn();
    let trace = json!({
        "id": "A", "environment": "vire", "timestamp": "2026-06-05T00:00:00Z", "metadata": {},
        "observations": [{"type": "GENERATION", "usageDetails": {}, "costDetails": {}}]
    });
    let api = MockApi::with_pages("vire", vec![vec![trace]]);
    let s = run_import(&api, &c, &local_vire(), &window());
    assert_eq!(s[0].health, HealthState::SchemaChanged);
    assert_eq!(
        s[0].skipped_schema, 1,
        "the empty-shape generation is counted as skipped"
    );
    assert_eq!(
        evidence_cost(&c, "vire", "A"),
        None,
        "an unrecognized shape never yields a zero cost"
    );
}

// ----- TASK-027 A2/A3: the import report (secret-free diagnostics, SEC-010) -----------------

#[test]
fn import_report_aggregates_per_env_and_total_counts() {
    let c = conn();
    // vire: two unique healthy traces; default (synthetic probe): one wrong-env trace surfaced.
    let mut api = MockApi::with_pages(
        "vire",
        vec![vec![
            trace_with_generation("A", "vire", "2026-06-05T00:00:00Z", 1.0, 10),
            trace_with_generation("B", "vire", "2026-06-06T00:00:00Z", 2.0, 20),
        ]],
    );
    api.pages.insert(
        "default".into(),
        vec![vec![trace_with_generation(
            "D",
            "default",
            "2026-06-05T00:00:00Z",
            1.0,
            8,
        )]],
    );
    let summaries = run_import(&api, &c, &local_vire(), &window());
    let report = ImportReport::from_summaries(&summaries);

    assert_eq!(report.environment_count, 2);
    let vire = report
        .environments
        .iter()
        .find(|e| e.environment == "vire")
        .unwrap();
    assert_eq!(vire.health, "healthy");
    assert_eq!(vire.unique, 2);
    assert_eq!(vire.skipped_schema, 0);
    let def = report
        .environments
        .iter()
        .find(|e| e.environment == "default")
        .unwrap();
    assert_eq!(def.health, "wrong_env");
    assert_eq!(def.unique, 1);

    assert_eq!(report.total_unique, 3, "totals sum across environments");
    assert_eq!(report.total_traces_seen, 3);
    assert_eq!(report.total_skipped_schema, 0);
}

#[test]
fn import_report_explains_an_empty_import_rather_than_blank() {
    let c = conn();
    let api = MockApi::default(); // nothing in vire, nothing in default
    let summaries = run_import(&api, &c, &local_vire(), &window());
    let report = ImportReport::from_summaries(&summaries);

    assert_eq!(report.total_unique, 0);
    // The configured environment is still surfaced with an explicit health state — never blank.
    let vire = report
        .environments
        .iter()
        .find(|e| e.environment == "vire")
        .unwrap();
    assert_eq!(vire.health, "missing");
    assert_eq!(vire.traces_seen, 0);
}

#[test]
fn import_report_counts_duplicates_and_skips_on_a_partial_run() {
    let c = conn();
    // page1: [A healthy, BAD unparseable]; page2: [A duplicate, B healthy].
    let bad = json!({ "environment": "vire" }); // no id → unparseable → skipped
    let api = MockApi::with_pages(
        "vire",
        vec![
            vec![
                trace_with_generation("A", "vire", "2026-06-02T00:00:00Z", 1.0, 10),
                bad,
            ],
            vec![
                trace_with_generation("A", "vire", "2026-06-02T00:00:00Z", 1.0, 10),
                trace_with_generation("B", "vire", "2026-06-03T00:00:00Z", 2.0, 20),
            ],
        ],
    );
    let summaries = run_import(&api, &c, &local_vire(), &window());
    let report = ImportReport::from_summaries(&summaries);
    let vire = report
        .environments
        .iter()
        .find(|e| e.environment == "vire")
        .unwrap();
    assert_eq!(vire.unique, 2, "A and B imported once each");
    assert_eq!(vire.duplicates, 1, "the cross-page A is suppressed");
    assert_eq!(
        vire.skipped_schema, 1,
        "the unparseable trace is counted as skipped"
    );
    assert_eq!(report.total_duplicates, 1);
    assert_eq!(report.total_skipped_schema, 1);
}

/// SEC-010: the import report MUST carry no credential, `Authorization` header, raw response body,
/// or trace prompt/session content — only counts, health, env names, and secret-free warnings. A
/// trace whose raw payload is deliberately stuffed with secret-shaped content proves none of it
/// leaks into the serialized report.
#[test]
fn import_report_is_secret_free() {
    let c = conn();
    let trace = json!({
        "id": "SECRET",
        "environment": "vire",
        "timestamp": "2026-06-05T00:00:00Z",
        "name": "sk-lf-supersecret-canary",
        "sessionId": "session-Bearer-leak",
        "metadata": {"Authorization": "Bearer pk-lf-leak", "secret": "sk-ant-oat01-leak"},
        "observations": [{
            "type": "GENERATION",
            "usageDetails": {"input": 4, "output": 6, "total": 10},
            "costDetails": {"total": 1.5}
        }]
    });
    let api = MockApi::with_pages("vire", vec![vec![trace]]);
    let summaries = run_import(&api, &c, &local_vire(), &window());
    let report = ImportReport::from_summaries(&summaries);

    // Sanity: the trace WAS imported, so the secret-free assertion below is not vacuous.
    assert_eq!(report.total_unique, 1);

    let serialized = serde_json::to_string(&report).unwrap();
    for needle in [
        "sk-",
        "pk-",
        "Bearer",
        "Authorization",
        "supersecret",
        "canary",
        "leak",
        "session-",
    ] {
        assert!(
            !serialized.contains(needle),
            "import report must be secret-free (SEC-010), found {needle}"
        );
    }
}

// ----- TASK-027 C: environment discovery (read-only, distinct names, secret-free) -----------

#[test]
fn discovery_collects_distinct_non_empty_environments_across_pages() {
    // A no-environment-filter scan that spans three environments across two pages, with one repeat
    // and one empty-name trace. Discovery must return the sorted distinct non-empty set.
    let mut api = MockApi::default();
    api.any_env_pages = vec![
        vec![
            trace_with_generation("A", "vire", "2026-06-05T00:00:00Z", 1.0, 10),
            trace_with_generation("B", "default", "2026-06-05T00:00:00Z", 1.0, 10),
        ],
        vec![
            // `staging` is new; `vire` repeats (must dedup); the empty-name trace contributes nothing.
            trace_with_generation("C", "staging", "2026-06-06T00:00:00Z", 1.0, 10),
            trace_with_generation("D", "vire", "2026-06-06T00:00:00Z", 1.0, 10),
            trace_with_generation("E", "", "2026-06-06T00:00:00Z", 1.0, 10),
        ],
    ];
    let envs = super::discovery::discover_environments(&api, &window()).unwrap();
    assert_eq!(
        envs,
        vec![
            "default".to_string(),
            "staging".to_string(),
            "vire".to_string()
        ],
        "distinct, non-empty, sorted environments"
    );
    // Discovery uses only the no-filter read path — never the per-environment `get_traces`.
    assert!(api
        .calls
        .borrow()
        .iter()
        .all(|c| c.starts_with("get_traces_any_env:")));
}

#[test]
fn discovery_returns_empty_when_no_traces_and_errors_propagate() {
    // No pages → empty set (not an error, not a fabricated environment).
    let api = MockApi::default();
    assert!(super::discovery::discover_environments(&api, &window())
        .unwrap()
        .is_empty());

    // A transport/API failure aborts with Err so the caller can treat discovery as best-effort.
    let mut failing = MockApi::default();
    failing.any_env_err = Some(ApiError::new(ApiErrorKind::Unavailable, "down"));
    assert!(super::discovery::discover_environments(&failing, &window()).is_err());
}

#[test]
fn discovered_environments_persist_additively_with_last_seen() {
    let c = conn();
    store::upsert_discovered_environment(&c, "vire", "2026-06-05T00:00:00Z").unwrap();
    store::upsert_discovered_environment(&c, "default", "2026-06-05T00:00:00Z").unwrap();
    // Re-seeing `vire` later advances last_seen but never duplicates the row or moves first_seen.
    store::upsert_discovered_environment(&c, "vire", "2026-06-07T00:00:00Z").unwrap();

    let rows = store::list_discovered_environments(&c).unwrap();
    assert_eq!(
        rows.len(),
        2,
        "idempotent on environment — no duplicate row"
    );
    let vire = rows.iter().find(|r| r.environment == "vire").unwrap();
    assert_eq!(
        vire.first_seen, "2026-06-05T00:00:00Z",
        "first_seen is preserved"
    );
    assert_eq!(
        vire.last_seen, "2026-06-07T00:00:00Z",
        "last_seen advances on re-discovery"
    );
}

#[test]
fn discovery_url_keeps_the_allowlist_and_loopback_gate_without_an_env_param() {
    // The discovery path is still rooted under /api/public/traces on the loopback host, and carries
    // NO `environment` query param (that filter is what discovery drops to span all environments).
    let config = local_vire();
    let url = config
        .build_url(&ApiPath::TracesAllEnvironments {
            from: "2026-06-01T00:00:00Z",
            to: "2026-06-10T00:00:00Z",
            page: 1,
            limit: 50,
        })
        .unwrap();
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(url.scheme(), "http");
    assert!(url.path().starts_with("/api/public/traces"));
    let query = url.query().unwrap();
    assert!(
        !query.contains("environment="),
        "discovery omits the environment filter"
    );
    assert!(query.contains("fromTimestamp="));

    // A non-loopback `local` target is refused for discovery exactly as for trace import (SEC-002).
    let off_host = ImporterConfig::new(
        "http://example.com:3000",
        Source::Local,
        vec!["vire".into()],
        None,
    );
    assert!(off_host
        .build_url(&ApiPath::TracesAllEnvironments {
            from: "x",
            to: "y",
            page: 1,
            limit: 50,
        })
        .is_err());
}

// ===== TASK-029 =============================================================================
// Workstream A: forensic secret-free skip diagnostics; B: tolerant identification + v3 widening;
// C: configurable range + incremental cursor + resumable chunked backfill.

// ----- helpers ------------------------------------------------------------------------------

/// A trace whose `observations` is the current Langfuse v3 LIST shape: an array of observation ID
/// STRINGS, not embedded objects. The ID strings are deliberately secret-shaped to prove no value
/// leaks into the diagnostics (SEC-011).
fn trace_idlist_observations(id: &str, env: &str, ts: &str) -> Value {
    json!({
        "id": id,
        "environment": env,
        "timestamp": ts,
        "name": "claude-code",
        "sessionId": null,
        "metadata": {},
        "observations": ["sk-obs-leak-aaa", "pk-obs-leak-bbb"]
    })
}

fn generation_obs(cost: f64, total_tokens: i64) -> Observation {
    Observation {
        obs_type: Some("GENERATION".into()),
        model: Some("claude".into()),
        start_time: Some("2026-06-05T00:00:00Z".into()),
        end_time: Some("2026-06-05T00:01:00Z".into()),
        prompt_tokens: Some(total_tokens / 2),
        completion_tokens: Some(total_tokens / 2),
        total_tokens: Some(total_tokens),
        usage: None,
        calculated_total_cost: Some(cost),
        usage_details: None,
        cost_details: None,
    }
}

fn reason_count(reasons: &[super::model::SkipReasonCount], reason: &str) -> usize {
    reasons
        .iter()
        .find(|rc| rc.reason == reason)
        .map(|rc| rc.count)
        .unwrap_or(0)
}

fn raw_trace_rows(c: &Connection, env: &str) -> i64 {
    c.query_row(
        "SELECT COUNT(*) FROM langfuse_raw_traces WHERE environment = ?1",
        rusqlite::params![env],
        |r| r.get(0),
    )
    .unwrap()
}

/// The persisted raw-trace ids for an environment, sorted, so a test can assert exactly which traces a
/// run reached (not just how many).
fn trace_id_rows(c: &Connection, env: &str) -> Vec<String> {
    let mut stmt = c
        .prepare(
            "SELECT trace_id FROM langfuse_raw_traces WHERE environment = ?1 ORDER BY trace_id",
        )
        .unwrap();
    let rows = stmt
        .query_map(rusqlite::params![env], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<String>, _>>()
        .unwrap();
    rows
}

/// Parse an RFC3339 timestamp to `DateTime<Utc>` for the mocks/tests that compare instants directly.
fn ts(value: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&chrono::Utc)
}

/// An in-memory connection with the importer tables AND a minimal `settings` table, for the
/// range-resolution tests (which read/write the key-value `settings` rows).
fn conn_with_settings() -> Connection {
    let c = conn();
    c.execute(
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .unwrap();
    c
}

// ----- Workstream B: tolerant identification + v3 observations widening ----------------------

#[test]
fn identifiable_trace_with_idlist_observations_is_imported() {
    let c = conn();
    let mut api = MockApi::with_pages(
        "vire",
        vec![vec![trace_idlist_observations(
            "A",
            "vire",
            "2026-06-05T00:00:00Z",
        )]],
    );
    // Usage/cost live on the fetched observations (the v3 list payload only carried IDs).
    api.observations
        .insert("A".into(), vec![generation_obs(3.5, 20)]);

    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();

    // The trace is IMPORTED (not dropped) and healthy, with usage/cost read from the fetch.
    assert_eq!(
        vire.unique, 1,
        "the ID-list trace is identified and imported"
    );
    assert_eq!(vire.health, HealthState::Healthy);
    assert_eq!(
        vire.skipped_schema, 0,
        "ID-list shape is not a genuine skip"
    );
    assert_eq!(evidence_cost(&c, "vire", "A"), Some(3.5));
    // The ID-list shape is recorded as an INFORMATIONAL reason, not a drop.
    assert_eq!(
        reason_count(&vire.skip_reasons, "observations_not_embedded"),
        1
    );
    // The importer fell through to the observations fetch for usage/cost.
    assert!(api
        .calls
        .borrow()
        .iter()
        .any(|call| call == "get_observations:A"));
}

#[test]
fn generation_with_no_usage_anywhere_is_schema_changed_absence_preserved() {
    let c = conn();
    // Embedded generation with NO usage and NO cost in any supported location.
    let trace = json!({
        "id": "A", "environment": "vire", "timestamp": "2026-06-05T00:00:00Z",
        "observations": [{"type": "GENERATION", "model": "claude"}]
    });
    let api = MockApi::with_pages("vire", vec![vec![trace]]);
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();

    // Imported and surfaced as schema_changed — never silently dropped, never a zero total.
    assert_eq!(vire.unique, 1, "the trace is imported, not dropped");
    assert_eq!(vire.health, HealthState::SchemaChanged);
    assert_eq!(
        evidence_cost(&c, "vire", "A"),
        None,
        "absent cost stays NULL"
    );
    assert_eq!(
        reason_count(&vire.skip_reasons, "generation_lacks_usage_and_cost"),
        1
    );
    assert_eq!(vire.skipped_schema, 1);
}

#[test]
fn trace_with_no_id_is_missing_trace_id_counted_not_crashed() {
    let c = conn();
    let no_id = json!({ "environment": "vire", "timestamp": "2026-06-05T00:00:00Z" });
    let api = MockApi::with_pages("vire", vec![vec![no_id]]);
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();

    assert_eq!(vire.unique, 0, "an unidentifiable entry is not imported");
    assert_eq!(reason_count(&vire.skip_reasons, "missing_trace_id"), 1);
    assert_eq!(vire.skipped_schema, 1);
    assert_eq!(vire.health, HealthState::SchemaChanged);
    assert_eq!(
        raw_trace_rows(&c, "vire"),
        0,
        "no row for the dropped entry"
    );
}

#[test]
fn peripheral_field_type_mismatch_still_imports_identifiable_trace() {
    let c = conn();
    // `timestamp` is a NUMBER, not a string — a peripheral type mismatch that used to drop the whole
    // trace. The id is usable, so the trace must still be imported (B1 / DEC-031).
    let trace = json!({ "id": "A", "environment": "vire", "timestamp": 1717545600 });
    let api = MockApi::with_pages("vire", vec![vec![trace]]);
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();

    assert_eq!(
        vire.unique, 1,
        "the identifiable trace is imported despite the mismatch"
    );
    assert_eq!(reason_count(&vire.skip_reasons, "field_type_mismatch"), 1);
    // A tolerated peripheral mismatch is informational — it does not force schema_changed.
    assert_eq!(vire.skipped_schema, 0);
    assert_eq!(raw_trace_rows(&c, "vire"), 1);
}

// ----- Workstream A: classification, aggregation, SEC-011 ------------------------------------

#[test]
fn skip_reasons_aggregate_per_env_and_in_total() {
    let c = conn();
    // vire: one missing-id drop + one ID-list (informational). default(synthetic): one missing-id.
    let mut api = MockApi::with_pages(
        "vire",
        vec![vec![
            json!({ "environment": "vire" }),
            trace_idlist_observations("A", "vire", "2026-06-05T00:00:00Z"),
        ]],
    );
    api.observations
        .insert("A".into(), vec![generation_obs(1.0, 10)]);
    api.pages.insert(
        "default".into(),
        vec![vec![json!({ "environment": "default" })]],
    );

    let summaries = run_import(&api, &c, &local_vire(), &window());
    let report = ImportReport::from_summaries(&summaries);

    let vire = report
        .environments
        .iter()
        .find(|e| e.environment == "vire")
        .unwrap();
    assert_eq!(reason_count(&vire.skip_reasons, "missing_trace_id"), 1);
    assert_eq!(
        reason_count(&vire.skip_reasons, "observations_not_embedded"),
        1
    );
    // Total sums the missing-id across vire + default.
    assert_eq!(
        reason_count(&report.total_skip_reasons, "missing_trace_id"),
        2
    );
    assert_eq!(
        reason_count(&report.total_skip_reasons, "observations_not_embedded"),
        1
    );
}

#[test]
fn skip_samples_are_bounded_per_reason() {
    let c = conn();
    // Five missing-id entries; at most MAX_SAMPLES_PER_REASON (3) samples are kept.
    let page: Vec<Value> = (0..5).map(|_| json!({ "environment": "vire" })).collect();
    let api = MockApi::with_pages("vire", vec![page]);
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();

    assert_eq!(reason_count(&vire.skip_reasons, "missing_trace_id"), 5);
    let samples = vire
        .skip_samples
        .iter()
        .filter(|s| s.reason == "missing_trace_id")
        .count();
    assert!(
        samples <= 3,
        "samples are bounded per reason, got {samples}"
    );
    assert!(samples >= 1, "at least one sample is kept");
}

/// SEC-011: the grouped reasons AND the structural samples carry no secret/value/content. A skipped
/// trace and a degraded trace, each stuffed with secret-shaped values, prove none of it leaks — and
/// the ID-list sample carries only the field's JSON TYPE name (`string`), never the ID value.
#[test]
fn skip_diagnostics_are_secret_free() {
    let c = conn();
    let degraded = json!({
        "id": "deg",
        "environment": "vire",
        "timestamp": "2026-06-05T00:00:00Z",
        "name": "sk-name-canary",
        "sessionId": "session-Bearer-leak",
        "metadata": {"Authorization": "Bearer pk-meta-leak", "secret": "sk-ant-oat01-leak"},
        "observations": [{"type": "GENERATION", "model": "claude"}]
    });
    let idlist = trace_idlist_observations("idl", "vire", "2026-06-05T00:00:00Z");
    let api = MockApi::with_pages("vire", vec![vec![degraded, idlist]]);
    let summaries = run_import(&api, &c, &local_vire(), &window());
    let report = ImportReport::from_summaries(&summaries);

    // Not vacuous: a degrade and an informational reason were both recorded.
    let vire = report
        .environments
        .iter()
        .find(|e| e.environment == "vire")
        .unwrap();
    assert!(reason_count(&vire.skip_reasons, "generation_lacks_usage_and_cost") >= 1);
    assert!(reason_count(&vire.skip_reasons, "observations_not_embedded") >= 1);

    // The ID-list sample names only the field type, never the ID value.
    let idlist_sample = vire
        .skip_samples
        .iter()
        .find(|s| s.reason == "observations_not_embedded")
        .expect("an observations_not_embedded sample is kept");
    assert_eq!(idlist_sample.field.as_deref(), Some("observations"));
    assert_eq!(idlist_sample.field_type.as_deref(), Some("array"));
    assert_eq!(idlist_sample.element_type.as_deref(), Some("string"));

    let serialized = serde_json::to_string(&report).unwrap();
    for needle in [
        "sk-",
        "pk-",
        "Bearer",
        "Authorization",
        "canary",
        "leak",
        "session-",
        "oat01",
    ] {
        assert!(
            !serialized.contains(needle),
            "skip diagnostics must be secret-free (SEC-011), found {needle}"
        );
    }
}

// ----- Workstream C: import range, incremental cursor, resumable backfill --------------------

#[test]
fn import_range_parses_validates_and_floors() {
    let now = chrono::DateTime::parse_from_rfc3339("2026-06-30T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    assert_eq!(ImportRange::parse("last_7d").unwrap(), ImportRange::Last7d);
    assert_eq!(
        ImportRange::parse("LAST_30D").unwrap(),
        ImportRange::Last30d
    );
    assert_eq!(ImportRange::parse("all").unwrap(), ImportRange::All);
    // A valid `since:` is accepted and normalized to UTC RFC3339.
    let since = ImportRange::parse("since:2026-01-15T12:00:00Z").unwrap();
    assert_eq!(since, ImportRange::Since("2026-01-15T12:00:00Z".into()));

    // Malformed / unknown values are rejected with a fixed, secret-free error.
    assert!(ImportRange::parse("since:not-a-timestamp").is_err());
    assert!(ImportRange::parse("since:").is_err());
    assert!(ImportRange::parse("yesterday").is_err());
    let err = ImportRange::parse("since:secret-value-xyz").unwrap_err();
    assert!(
        !err.contains("secret-value-xyz"),
        "error never echoes the input"
    );

    // Floors: last_7d → now-7d; all → epoch; since → the timestamp.
    assert_eq!(ImportRange::Last7d.floor(now), "2026-06-23T00:00:00Z");
    assert_eq!(ImportRange::All.floor(now), "1970-01-01T00:00:00Z");
    assert_eq!(since.floor(now), "2026-01-15T12:00:00Z");
}

#[test]
fn incremental_window_resumes_from_cursor_floored_by_range() {
    let now = "2026-06-30T00:00:00Z";
    let floor = "2026-06-01T00:00:00Z";

    // No cursor → first import starts at the range floor.
    let w = incremental_window(floor, None, now);
    assert_eq!(w.from, floor);
    assert_eq!(w.to, now);

    // Cursor newer than the floor → from = cursor − 1h overlap (re-sees late traces).
    let w = incremental_window(floor, Some("2026-06-20T10:00:00Z"), now);
    assert_eq!(w.from, "2026-06-20T09:00:00Z");

    // Cursor older than the floor → the floor wins (never reach below the configured range).
    let w = incremental_window(floor, Some("2026-05-01T00:00:00Z"), now);
    assert_eq!(w.from, floor);
}

#[test]
fn cursor_advances_by_instant_not_lexically_across_offsets_and_precision() {
    // Two traces whose RFC3339 strings sort the OPPOSITE way lexically vs chronologically:
    //   X: 2026-06-10T10:00:00+02:00  == 08:00:00Z  (earlier instant, but lexically "T10…" > "T09…")
    //   Y: 2026-06-10T09:30:00Z                       (later instant)
    // A lexicographic max picks X; the chronological max must pick Y.
    let c = conn();
    let x = trace_with_generation("X", "vire", "2026-06-10T10:00:00+02:00", 1.0, 10);
    let y = trace_with_generation("Y", "vire", "2026-06-10T09:30:00Z", 2.0, 20);
    let api = MockApi::with_pages("vire", vec![vec![x, y]]);
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|e| e.environment == "vire").unwrap();
    assert_eq!(
        vire.cursor_ts.as_deref(),
        Some("2026-06-10T09:30:00Z"),
        "the cursor is the chronologically-latest instant, not the lexically-largest string"
    );

    // Fractional precision: a value WITH a fraction is a later instant than the whole-second value it
    // extends, even though 'Z' (0x5A) sorts after '.' (0x2E) lexically.
    let c2 = conn();
    let p = trace_with_generation("P", "vire", "2026-06-10T09:30:05Z", 1.0, 10);
    let q = trace_with_generation("Q", "vire", "2026-06-10T09:30:05.250Z", 2.0, 20);
    let api2 = MockApi::with_pages("vire", vec![vec![p, q]]);
    let s2 = run_import(&api2, &c2, &local_vire(), &window());
    let vire2 = s2.iter().find(|e| e.environment == "vire").unwrap();
    assert_eq!(
        vire2.cursor_ts.as_deref(),
        Some("2026-06-10T09:30:05.250Z"),
        "fractional-second precision orders by instant, not by string bytes"
    );
}

#[test]
fn delayed_classification_and_cursor_compare_instants_across_offsets() {
    let c = conn();
    // First import establishes a cursor at 09:30:00Z.
    let api1 = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation(
            "base",
            "vire",
            "2026-06-10T09:30:00Z",
            1.0,
            10,
        )]],
    );
    run_import(&api1, &c, &local_vire(), &window());

    // Second import: a NEW trace whose instant is EARLIER than the cursor (10:00:00+02:00 == 08:00:00Z)
    // but whose RFC3339 string sorts AFTER it lexically. It is a late/delayed arrival a lexicographic
    // compare would miss, and the cursor must not regress to it.
    let api2 = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation(
            "late",
            "vire",
            "2026-06-10T10:00:00+02:00",
            2.0,
            20,
        )]],
    );
    let s = run_import(&api2, &c, &local_vire(), &window());
    let vire = s.iter().find(|e| e.environment == "vire").unwrap();
    assert_eq!(
        vire.health,
        HealthState::Delayed,
        "an earlier-instant arrival is classified delayed, not healthy"
    );
    assert_eq!(
        vire.cursor_ts.as_deref(),
        Some("2026-06-10T09:30:00Z"),
        "the cursor never regresses to the earlier-instant late trace"
    );
}

#[test]
fn first_import_uses_range_floor_then_resumes_from_cursor() {
    let c = conn();
    let floor = "2026-06-01T00:00:00Z";
    let api = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation(
            "A",
            "vire",
            "2026-06-20T00:00:00Z",
            1.0,
            10,
        )]],
    );
    let resolver = |_env: &str, cursor: Option<&str>| {
        incremental_window(floor, cursor, "2026-06-30T00:00:00Z")
    };

    // First import: no cursor → window.from is the range floor.
    run_import_with(&api, &c, &local_vire(), &resolver);
    let first_from = api
        .trace_windows
        .borrow()
        .iter()
        .find(|(env, _, _)| env == "vire")
        .map(|(_, from, _)| from.clone())
        .unwrap();
    assert_eq!(
        first_from, floor,
        "first import floors at the configured range"
    );

    // Second import: cursor is now established → window.from resumes from cursor − overlap.
    api.trace_windows.borrow_mut().clear();
    run_import_with(&api, &c, &local_vire(), &resolver);
    let second_from = api
        .trace_windows
        .borrow()
        .iter()
        .find(|(env, _, _)| env == "vire")
        .map(|(_, from, _)| from.clone())
        .unwrap();
    assert_eq!(
        second_from, "2026-06-19T23:00:00Z",
        "second import resumes from the cursor less the overlap"
    );
}

#[test]
fn backfill_imports_history_in_chunks_and_is_resumable() {
    let c = conn();
    // A wide range → multiple atomic chunks. Two traces returned regardless of window (the mock
    // ignores the window), so chunk 1 imports both and later chunks re-see them as duplicates.
    let api = MockApi::with_pages(
        "vire",
        vec![vec![
            trace_with_generation("A", "vire", "2026-05-15T00:00:00Z", 1.0, 10),
            trace_with_generation("B", "vire", "2026-05-16T00:00:00Z", 2.0, 20),
        ]],
    );
    let floor = "2026-03-01T00:00:00Z";
    let now = "2026-06-01T00:00:00Z";

    let summaries = run_backfill(&api, &c, &local_vire(), floor, now);
    let vire = summaries.iter().find(|x| x.environment == "vire").unwrap();

    // More than one chunk ran (oldest→newest, DEC-032), proving the backfill is NOT one giant window.
    let vire_calls = api
        .calls
        .borrow()
        .iter()
        .filter(|call| call.as_str() == "get_traces:vire:1")
        .count();
    assert!(
        vire_calls >= 2,
        "backfill ran multiple bounded chunks, got {vire_calls}"
    );

    // Both traces imported exactly once (durable dedup across chunks); duplicates from later chunks.
    assert_eq!(vire.unique, 2);
    assert!(
        vire.duplicates >= 2,
        "later chunks re-see imported traces as duplicates"
    );
    assert_eq!(raw_trace_rows(&c, "vire"), 2, "no duplicate rows persisted");
    let cursor_after_first = vire.cursor_ts.clone();
    assert!(cursor_after_first.is_some());

    // Re-running the backfill converges: nothing new, no duplicate rows, cursor does not regress.
    let summaries2 = run_backfill(&api, &c, &local_vire(), floor, now);
    let vire2 = summaries2.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(vire2.unique, 0, "a re-run imports nothing new");
    assert_eq!(
        raw_trace_rows(&c, "vire"),
        2,
        "still exactly two rows after re-run"
    );
    assert_eq!(
        vire2.cursor_ts, cursor_after_first,
        "the cursor does not regress on re-run"
    );
}

#[test]
fn backfill_reports_bounded_run_rather_than_truncating_silently() {
    let c = conn();
    // A page source that always reports far more pages than it will ever serve, with non-empty data,
    // so a single window hits the MAX_PAGES backstop. The reached_page_limit flag must surface it.
    let api = MockApi {
        infinite_pages: Some(trace_with_generation(
            "A",
            "vire",
            "2026-06-05T00:00:00Z",
            1.0,
            10,
        )),
        ..Default::default()
    };
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert!(
        vire.reached_page_limit,
        "a window that hits the page backstop reports it (no silent truncation)"
    );
    let report = ImportReport::from_summaries(&s);
    assert!(report.reached_page_limit);
}

// ----- DEC-032: ascending sweep + inclusive-`fromTimestamp` resume-cursor (arch-review §8.3–§8.5) -
//
// These replace the descending / exclusive-`toTimestamp` / second-oldest-instant (`min_ts2`) tests, which
// asserted the data-loss the redesign eliminates. Each new mock is discriminated on the resolved inclusive
// `from` bound (the resume-cursor), NOT an exclusive `to` ceiling.

/// Models a window wider than one page-depth can drain, swept oldest → newest with an inclusive
/// `fromTimestamp` cursor:
///   * `from < boundary` (run 1, before the cursor advanced): the oldest history is dense across MANY
///     instants. Page 1 yields a small finite OLDER block (distinct instants below `boundary`); later pages
///     yield one filler trace stamped at `boundary` with `total_pages = u32::MAX`, so the importer walks to
///     its `MAX_PAGES` backstop with `max_reached == boundary` (>1 instant ⇒ bounded, not saturated).
///   * `from >= boundary` (run 2, resumed inclusive at `max_reached`): re-reads the boundary instant
///     (durable dedup suppresses it) then drains the finite strictly-NEWER history in one page, so the
///     cursor clears.
struct ForwardSweepMock {
    env: String,
    boundary: chrono::DateTime<chrono::Utc>,
    older_block: Vec<Value>,
    boundary_filler: Value,
    newer_block: Vec<Value>,
}

impl LangfuseApi for ForwardSweepMock {
    fn probe(&self) -> Result<(), ApiError> {
        Ok(())
    }
    fn get_traces(
        &self,
        environment: &str,
        from: &str,
        _to: &str,
        page: u32,
        limit: u32,
        _order_by: &str,
    ) -> Result<TracePage, ApiError> {
        let meta = |total_pages| super::model::PageMeta {
            page,
            limit,
            total_items: 0,
            total_pages,
        };
        if environment != self.env {
            return Ok(TracePage {
                data: vec![],
                meta: meta(0),
            });
        }
        if ts(from) < self.boundary {
            // Run 1: dense oldest history across many instants → walks to the backstop.
            let data = if page == 1 {
                self.older_block.clone()
            } else {
                vec![self.boundary_filler.clone()]
            };
            return Ok(TracePage {
                data,
                meta: meta(u32::MAX),
            });
        }
        // Run 2 (inclusive resume at/above the boundary): re-read the boundary instant, then the finite
        // strictly-newer history; drains in one page (no backstop → the cursor clears).
        let data = if page == 1 {
            let mut d = vec![self.boundary_filler.clone()];
            d.extend(self.newer_block.clone());
            d
        } else {
            vec![]
        };
        Ok(TracePage {
            data,
            meta: meta(1),
        })
    }
    fn get_traces_any_env(
        &self,
        _from: &str,
        _to: &str,
        page: u32,
        limit: u32,
    ) -> Result<TracePage, ApiError> {
        Ok(TracePage {
            data: vec![],
            meta: super::model::PageMeta {
                page,
                limit,
                total_items: 0,
                total_pages: 0,
            },
        })
    }
    fn get_observations(&self, _trace_id: &str) -> Result<Vec<Observation>, ApiError> {
        Ok(vec![])
    }
}

#[test]
fn backfill_page_limited_resumes_forward_by_inclusive_from_cursor() {
    let c = conn();
    let range_floor = "2026-05-01T00:00:00Z";
    let now = "2026-05-20T00:00:00Z"; // < 30 days → a single chunk per run.
    let boundary = "2026-05-10T00:00:00Z";
    let api = ForwardSweepMock {
        env: "vire".into(),
        boundary: ts(boundary),
        older_block: vec![
            trace_with_generation("OLD1", "vire", "2026-05-02T00:00:00Z", 1.0, 10),
            trace_with_generation("OLD2", "vire", "2026-05-03T00:00:00Z", 2.0, 20),
        ],
        boundary_filler: trace_with_generation("MID", "vire", boundary, 3.0, 30),
        newer_block: vec![
            trace_with_generation("NEW1", "vire", "2026-05-12T00:00:00Z", 4.0, 40),
            trace_with_generation("NEW2", "vire", "2026-05-14T00:00:00Z", 5.0, 50),
        ],
    };

    // Run 1: the oldest history is dense across many instants → the page backstop is hit. The run imports
    // the reachable oldest traces (OLD1/OLD2/MID) and persists `resume_from = max_reached` (the NEWEST
    // instant it returned = the boundary). The strictly-newer history is not reached yet.
    let s1 = run_backfill(&api, &c, &local_vire(), range_floor, now);
    let vire1 = s1.iter().find(|x| x.environment == "vire").unwrap();
    assert!(vire1.reached_page_limit, "run 1 hits the page backstop");
    assert!(
        !vire1.instant_saturated,
        "run 1 spans many instants (min_seen != max_seen) → NOT the single-instant terminal"
    );
    assert_eq!(
        trace_id_rows(&c, "vire"),
        vec!["MID".to_string(), "OLD1".to_string(), "OLD2".to_string()],
        "run 1 reaches only the oldest history up to the boundary; newer history is still beyond the page limit"
    );
    assert_eq!(
        store::backfill_resume_from(&c).unwrap().as_deref(),
        Some(boundary),
        "run 1 persists the inclusive resume-cursor at max_reached (the newest instant it returned)"
    );

    // Run 2: resumes with `fromTimestamp = boundary` (inclusive), re-reads the boundary instant (deduped)
    // and pages into the strictly-NEWER history — forward, monotonic progress, not a re-walk of run 1.
    let s2 = run_backfill(&api, &c, &local_vire(), range_floor, now);
    let vire2 = s2.iter().find(|x| x.environment == "vire").unwrap();
    assert!(
        vire2.unique >= 2,
        "the re-run imports the strictly-newer history beyond the previous page limit, got {}",
        vire2.unique
    );
    assert!(
        vire2.duplicates >= 1,
        "the re-run re-reads the boundary instant and durable dedup suppresses it (≥1 duplicate)"
    );
    // Union over both runs == the full source set, every trace imported EXACTLY once (PK-enforced rows).
    assert_eq!(
        trace_id_rows(&c, "vire"),
        vec![
            "MID".to_string(),
            "NEW1".to_string(),
            "NEW2".to_string(),
            "OLD1".to_string(),
            "OLD2".to_string(),
        ],
        "after the re-run the full source set is imported exactly once (no skip, no duplicate row)"
    );
    assert!(
        store::backfill_resume_from(&c).unwrap().is_none(),
        "a re-run that drains without a page limit clears the inclusive cursor (range fully covered)"
    );
}

/// Models the SW-4 regression DEC-032 fixes: a block of equal-`timestamp` traces straddling the page
/// limit. The boundary instant `B` holds N (< D) traces; run 1's page depth admits only an OLDER trace
/// plus the FIRST part of the equal block, page-limiting before the rest. Discriminated on the inclusive
/// `from` bound:
///   * `from < B` (run 1): page 1 yields an OLDER trace (so the run spans >1 instant, NOT saturated) plus
///     the first two equal-`B` traces; later pages repeat one (dedup filler) with `total_pages = u32::MAX`
///     so the backstop is hit with `max_reached == B`, the remaining equal-`B` traces unread.
///   * `from >= B` (run 2, inclusive resume at `B`): re-reads the WHOLE equal-`B` block in one page; dedup
///     suppresses the already-imported part and the previously-unread equal-`timestamp` traces are imported
///     — never skipped.
struct EqualTimestampBoundaryMock {
    env: String,
    boundary: chrono::DateTime<chrono::Utc>,
    older: Value,
    equal_block: Vec<Value>,
}

impl LangfuseApi for EqualTimestampBoundaryMock {
    fn probe(&self) -> Result<(), ApiError> {
        Ok(())
    }
    fn get_traces(
        &self,
        environment: &str,
        from: &str,
        _to: &str,
        page: u32,
        limit: u32,
        _order_by: &str,
    ) -> Result<TracePage, ApiError> {
        let meta = |total_pages| super::model::PageMeta {
            page,
            limit,
            total_items: 0,
            total_pages,
        };
        if environment != self.env {
            return Ok(TracePage {
                data: vec![],
                meta: meta(0),
            });
        }
        if ts(from) < self.boundary {
            // Run 1: an older trace + the first part of the equal block, then dense filler to the backstop.
            let data = if page == 1 {
                let mut d = vec![self.older.clone()];
                d.extend(self.equal_block.iter().take(2).cloned());
                d
            } else {
                vec![self.equal_block[0].clone()]
            };
            return Ok(TracePage {
                data,
                meta: meta(u32::MAX),
            });
        }
        // Run 2: re-read the WHOLE equal-instant block in one page.
        let data = if page == 1 {
            self.equal_block.clone()
        } else {
            vec![]
        };
        Ok(TracePage {
            data,
            meta: meta(1),
        })
    }
    fn get_traces_any_env(
        &self,
        _from: &str,
        _to: &str,
        page: u32,
        limit: u32,
    ) -> Result<TracePage, ApiError> {
        Ok(TracePage {
            data: vec![],
            meta: super::model::PageMeta {
                page,
                limit,
                total_items: 0,
                total_pages: 0,
            },
        })
    }
    fn get_observations(&self, _trace_id: &str) -> Result<Vec<Observation>, ApiError> {
        Ok(vec![])
    }
}

#[test]
fn backfill_equal_timestamp_block_at_boundary_is_fully_reimported_not_skipped() {
    let c = conn();
    let floor = "2026-05-01T00:00:00Z";
    let now = "2026-05-20T00:00:00Z";
    let boundary = "2026-05-10T00:00:00Z";
    let equal_block: Vec<Value> = (1..=5)
        .map(|n| trace_with_generation(&format!("EQ{n}"), "vire", boundary, 1.0, 10))
        .collect();
    let api = EqualTimestampBoundaryMock {
        env: "vire".into(),
        boundary: ts(boundary),
        older: trace_with_generation("OLD", "vire", "2026-05-09T00:00:00Z", 2.0, 20),
        equal_block,
    };

    // Run 1: the page limit cuts THROUGH the equal-`boundary` block — only OLD + EQ1 + EQ2 are reached;
    // EQ3..EQ5 sit beyond the backstop. The window spans OLD < boundary (>1 instant) → bounded, not saturated.
    let s1 = run_backfill(&api, &c, &local_vire(), floor, now);
    let vire1 = s1.iter().find(|x| x.environment == "vire").unwrap();
    assert!(
        vire1.reached_page_limit,
        "run 1 hits the page backstop mid equal-timestamp block"
    );
    assert!(
        !vire1.instant_saturated,
        "the window spans OLD + boundary (>1 instant) → not the single-instant terminal"
    );
    assert_eq!(
        trace_id_rows(&c, "vire"),
        vec!["EQ1".to_string(), "EQ2".to_string(), "OLD".to_string()],
        "run 1 reaches only the first part of the equal-timestamp block"
    );
    assert_eq!(
        store::backfill_resume_from(&c).unwrap().as_deref(),
        Some(boundary),
        "run 1 persists the inclusive resume-cursor at the boundary instant (max_reached)"
    );

    // Run 2: `fromTimestamp = boundary` (inclusive) re-reads the WHOLE boundary instant; durable dedup
    // suppresses EQ1/EQ2 and the previously-unread equal-timestamp traces EQ3/EQ4/EQ5 are imported — the
    // exact SW-4 "skips unread equal-timestamp traces" regression, now fixed.
    let s2 = run_backfill(&api, &c, &local_vire(), floor, now);
    let vire2 = s2.iter().find(|x| x.environment == "vire").unwrap();
    assert!(
        vire2.unique >= 3,
        "the re-run imports the previously-unread equal-timestamp traces, got {}",
        vire2.unique
    );
    assert_eq!(
        trace_id_rows(&c, "vire"),
        vec![
            "EQ1".to_string(),
            "EQ2".to_string(),
            "EQ3".to_string(),
            "EQ4".to_string(),
            "EQ5".to_string(),
            "OLD".to_string(),
        ],
        "every equal-timestamp trace is imported exactly once (none skipped, no duplicate row)"
    );
    assert!(
        store::backfill_resume_from(&c).unwrap().is_none(),
        "draining the boundary instant without a page limit clears the cursor"
    );
}

/// Models the one genuinely-unreachable DEC-032 corner: a SINGLE `timestamp` instant holding ≥ D traces.
/// Every page returns a fresh trace id all stamped at the same instant with `total_pages = u32::MAX`, so a
/// run drains the oldest D within that instant and stops with `max_reached == from` — the cursor cannot
/// advance past the instant. It is detected and surfaced as a DISTINCT terminal diagnostic
/// (`instant_saturated`), never skipped (the cursor is not pushed past unread data) and never looped
/// (re-running re-reads the same instant and does NOT falsely report progress / converge). `page_calls`
/// proves bounded iteration (≤ `MAX_PAGES` per run — no infinite loop).
struct SaturatedInstantAscMock {
    env: String,
    instant_str: String,
    page_calls: RefCell<u32>,
}

impl LangfuseApi for SaturatedInstantAscMock {
    fn probe(&self) -> Result<(), ApiError> {
        Ok(())
    }
    fn get_traces(
        &self,
        environment: &str,
        _from: &str,
        _to: &str,
        page: u32,
        limit: u32,
        _order_by: &str,
    ) -> Result<TracePage, ApiError> {
        let meta = |total_pages| super::model::PageMeta {
            page,
            limit,
            total_items: 0,
            total_pages,
        };
        if environment != self.env {
            return Ok(TracePage {
                data: vec![],
                meta: meta(0),
            });
        }
        *self.page_calls.borrow_mut() += 1;
        // A fresh id every page, all at the SAME instant → ≥ D traces collide on one timestamp.
        let id = format!("SAT-{page}");
        Ok(TracePage {
            data: vec![trace_with_generation(
                &id,
                &self.env,
                &self.instant_str,
                1.0,
                10,
            )],
            meta: meta(u32::MAX),
        })
    }
    fn get_traces_any_env(
        &self,
        _from: &str,
        _to: &str,
        page: u32,
        limit: u32,
    ) -> Result<TracePage, ApiError> {
        Ok(TracePage {
            data: vec![],
            meta: super::model::PageMeta {
                page,
                limit,
                total_items: 0,
                total_pages: 0,
            },
        })
    }
    fn get_observations(&self, _trace_id: &str) -> Result<Vec<Observation>, ApiError> {
        Ok(vec![])
    }
}

#[test]
fn backfill_single_instant_at_or_above_page_depth_is_surfaced_terminal_not_looping() {
    let c = conn();
    let floor = "2026-05-01T00:00:00Z";
    let now = "2026-05-20T00:00:00Z";
    let instant = "2026-05-15T00:00:00Z";
    let api = SaturatedInstantAscMock {
        env: "vire".into(),
        instant_str: instant.into(),
        page_calls: RefCell::new(0),
    };

    // Run 1: the whole reachable window collapses to ONE instant (≥ D traces) → the page backstop is hit
    // and the cursor CANNOT advance (max_reached == resume_from). Surfaced as the DISTINCT saturation
    // terminal, not ordinary "re-run to continue".
    let s1 = run_backfill(&api, &c, &local_vire(), floor, now);
    let vire1 = s1.iter().find(|x| x.environment == "vire").unwrap();
    assert!(
        vire1.reached_page_limit,
        "the saturated window hits the page backstop"
    );
    assert!(
        vire1.instant_saturated,
        "a single instant ≥ page-depth is the DISTINCT saturation terminal"
    );
    let report1 = ImportReport::from_summaries(&s1);
    assert!(
        report1.instant_saturated,
        "the report surfaces saturation distinctly from reached_page_limit"
    );
    // (a) the cursor parks AT the instant — never advanced past unread data (no skip past the saturation).
    assert_eq!(
        store::backfill_resume_from(&c).unwrap().as_deref(),
        Some(instant),
        "the cursor parks at the saturated instant; it is never pushed past unread data"
    );
    // (c) the run is bounded — it walked to the MAX_PAGES backstop and stopped (no infinite loop).
    let calls_run1 = *api.page_calls.borrow();
    assert_eq!(
        calls_run1,
        super::importer::MAX_PAGES,
        "the run is bounded at the MAX_PAGES backstop, not an infinite loop"
    );
    let rows_after_run1 = trace_id_rows(&c, "vire");

    // Run 2: re-running re-reads the SAME saturated instant. It must NOT falsely report convergence: the
    // cursor stays parked, the diagnostic stays terminal, and no NEW data is reported as progress.
    let s2 = run_backfill(&api, &c, &local_vire(), floor, now);
    let vire2 = s2.iter().find(|x| x.environment == "vire").unwrap();
    assert!(
        vire2.instant_saturated,
        "the re-run stays the terminal saturation state (not 'progress')"
    );
    assert_eq!(
        vire2.unique, 0,
        "the re-run imports nothing new — it does not falsely report progress"
    );
    assert_eq!(
        store::backfill_resume_from(&c).unwrap().as_deref(),
        Some(instant),
        "the cursor is NOT cleared (no false convergence) and does NOT advance"
    );
    assert_eq!(
        trace_id_rows(&c, "vire"),
        rows_after_run1,
        "no new rows on re-run; the instant is stably terminal"
    );
    let calls_run2 = *api.page_calls.borrow() - calls_run1;
    assert_eq!(
        calls_run2,
        super::importer::MAX_PAGES,
        "the re-run is again bounded, never an infinite loop"
    );
}

#[test]
fn backfill_boundary_timestamp_is_robustly_parsed_else_imported_but_excluded_from_cursor() {
    // --- Robust parse + decoupled import: millisecond and offset timestamps the server accepted parse and
    //     drive the cursor; a genuinely garbage value is still IMPORTED (identification decoupled, B1) but
    //     EXCLUDED from the cursor (never lexically ordered). ---
    let c = conn();
    let api = MockApi::with_pages(
        "vire",
        vec![vec![
            // Millisecond precision — the chronological newest; parsed and kept verbatim as the cursor.
            trace_with_generation("MS", "vire", "2026-06-05T12:30:45.250Z", 1.0, 10),
            // Non-UTC offset (08:00Z) — parses by instant (older than MS); proves offsets are handled.
            trace_with_generation("OFFSET", "vire", "2026-06-05T10:00:00+02:00", 2.0, 20),
            // Unparseable — still identifiable, so still imported; excluded from the cursor.
            trace_with_generation("GARBAGE", "vire", "not-a-timestamp", 3.0, 30),
        ]],
    );
    let s = run_import(&api, &c, &local_vire(), &window());
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    assert_eq!(
        trace_id_rows(&c, "vire"),
        vec![
            "GARBAGE".to_string(),
            "MS".to_string(),
            "OFFSET".to_string(),
        ],
        "every identifiable trace is imported, including the one with an unparseable timestamp (decoupled identification, B1)"
    );
    assert_eq!(
        vire.cursor_ts.as_deref(),
        Some("2026-06-05T12:30:45.250Z"),
        "the millisecond timestamp is robustly parsed and kept verbatim as the cursor; the garbage value never wins lexically"
    );

    // --- All-unparseable degenerate (unreachable for a real time-windowed scan): a page-limited backfill
    //     whose traces carry NO usable timestamp yields no high-water. It must PRESERVE any existing cursor
    //     (never clear → never restart from `now`/floor) and be surfaced terminal, not looped. ---
    let c2 = conn();
    store::set_backfill_resume_from(&c2, "2026-05-10T00:00:00Z", "2026-06-01T00:00:00Z").unwrap();
    let api2 = MockApi {
        infinite_pages: Some(json!({
            "id": "no-ts",
            "environment": "vire",
            "observations": []
        })),
        ..Default::default()
    };
    let floor = "2026-05-01T00:00:00Z";
    let now = "2026-06-01T00:00:00Z";
    let s2 = run_backfill(&api2, &c2, &local_vire(), floor, now);
    let vire2 = s2.iter().find(|x| x.environment == "vire").unwrap();
    assert!(
        vire2.reached_page_limit,
        "the run still truthfully reports it was page-limited (incomplete)"
    );
    assert!(
        vire2.instant_saturated,
        "a page-limited run with no usable high-water is surfaced terminal (not 'progress'), so it is never looped"
    );
    assert_eq!(
        store::backfill_resume_from(&c2).unwrap().as_deref(),
        Some("2026-05-10T00:00:00Z"),
        "no usable timestamp preserves the prior inclusive cursor — never cleared, never restarts from now/floor"
    );
}

/// SEC-011 negative: the saturation/page-limit DIAGNOSTIC and the inclusive resume-cursor never leak a
/// timestamp VALUE (or any secret) into the serialized report. The report carries flags + counts only; the
/// resume cursor (`page_limit_resume_ts`) is internal to the importer and excluded from the report.
#[test]
fn backfill_saturation_diagnostic_and_cursor_carry_no_timestamp_value() {
    let c = conn();
    // A page-limiting source at a DISTINCTIVE instant, so we can prove that value never appears in the report.
    let instant = "2031-09-17T04:05:06.789Z";
    let api = MockApi {
        infinite_pages: Some(trace_with_generation("SAT", "vire", instant, 1.0, 10)),
        ..Default::default()
    };
    let s = run_backfill(
        &api,
        &c,
        &local_vire(),
        "2031-09-01T00:00:00Z",
        "2031-10-01T00:00:00Z",
    );
    let vire = s.iter().find(|x| x.environment == "vire").unwrap();
    // Sanity: the diagnostic actually fired (otherwise the secret-free assertion is vacuous).
    assert!(
        vire.reached_page_limit && vire.instant_saturated,
        "the single-instant saturation terminal fired"
    );
    // The cursor value EXISTS internally (so the no-leak assertion is meaningful).
    assert_eq!(
        store::backfill_resume_from(&c).unwrap().as_deref(),
        Some(instant)
    );

    let report = ImportReport::from_summaries(&s);
    let serialized = serde_json::to_string(&report).unwrap();
    for needle in ["2031-09-17", "04:05:06", "789Z"] {
        assert!(
            !serialized.contains(needle),
            "the report must not leak the resume-cursor / boundary timestamp value (SEC-011), found {needle}"
        );
    }
    // The terminal condition is still surfaced — as a boolean flag, not a value.
    assert!(
        report.instant_saturated,
        "saturation is surfaced as a secret-free flag"
    );
    assert!(report.reached_page_limit);
}

// ----- SW-4 Blocker 2: continuation persistence failures surface in-band -------------------

#[test]
fn continuation_boundary_persistence_failure_surfaces_in_band_not_a_false_resumable_claim() {
    let c = conn();
    // Drop the continuation table so every backfill_resume_from read AND write/clear errors. This models
    // a store that cannot durably record the inclusive resume-cursor.
    c.execute("DROP TABLE langfuse_backfill_progress", [])
        .unwrap();
    // A page-limiting source so the driver attempts to WRITE a boundary (exercising the set path, on top
    // of the failed initial read).
    let api = MockApi {
        infinite_pages: Some(trace_with_generation(
            "A",
            "vire",
            "2026-05-15T00:00:00Z",
            1.0,
            10,
        )),
        ..Default::default()
    };
    let s = run_backfill(
        &api,
        &c,
        &local_vire(),
        "2026-05-01T00:00:00Z",
        "2026-06-01T00:00:00Z",
    );
    assert!(
        s.iter().any(|x| x
            .warnings
            .iter()
            .any(|w| w == super::importer::PERSIST_FAILURE_MSG)),
        "a continuation read/write failure injects the in-band persist-failure sentinel"
    );
    assert!(
        super::import_result(&s).is_err(),
        "the manual-import IPC collapses the run to an Err, so the UI/report cannot claim resumability \
         after a failed boundary persist"
    );
}

// ----- SW-4 Blocker 3: malformed timestamps are ignored, never lexically ordered -----------

#[test]
fn malformed_trace_timestamp_is_ignored_for_cursor_and_delayed_not_lexically_ordered() {
    let c = conn();
    // First import establishes a valid cursor at 09:30:00Z.
    let api1 = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation(
            "base",
            "vire",
            "2026-06-10T09:30:00Z",
            1.0,
            10,
        )]],
    );
    run_import(&api1, &c, &local_vire(), &window());

    // A new trace whose timestamp is GARBAGE. A lexical compare would rank it ('n' 0x6E > '2' 0x32),
    // corrupting the cursor and/or the delayed classification. It must be IGNORED for both decisions.
    let api2 = MockApi::with_pages(
        "vire",
        vec![vec![trace_with_generation(
            "garbage",
            "vire",
            "not-a-timestamp",
            2.0,
            20,
        )]],
    );
    let s = run_import(&api2, &c, &local_vire(), &window());
    let vire = s.iter().find(|e| e.environment == "vire").unwrap();
    assert_eq!(
        vire.cursor_ts.as_deref(),
        Some("2026-06-10T09:30:00Z"),
        "an unparseable timestamp never advances or regresses the cursor (no lexical fallback)"
    );
    assert_ne!(
        vire.health,
        HealthState::Delayed,
        "an unparseable timestamp is not classified delayed (the comparison yields None, not a byte order)"
    );
}

// ----- range setting persistence (app-configuration) ----------------------------------------

#[test]
fn import_range_setting_persists_validates_and_defaults() {
    let c = conn_with_settings();

    // Absent → default last_30d.
    assert_eq!(
        crate::settings::resolve_import_range(&c),
        ImportRange::Last30d
    );
    assert_eq!(
        crate::settings::get_langfuse_import_range_repo(&c).unwrap(),
        "last_30d"
    );

    // Set + read back a valid value.
    let stored = crate::settings::set_langfuse_import_range_repo(&c, "last_90d".into()).unwrap();
    assert_eq!(stored, "last_90d");
    assert_eq!(
        crate::settings::resolve_import_range(&c),
        ImportRange::Last90d
    );

    // A malformed value is rejected with a secret-free error and does not overwrite the stored value.
    let err =
        crate::settings::set_langfuse_import_range_repo(&c, "since:nope-leak".into()).unwrap_err();
    assert!(!err.contains("nope-leak"), "error never echoes the input");
    assert_eq!(
        crate::settings::resolve_import_range(&c),
        ImportRange::Last90d,
        "the prior valid value is unchanged after a rejected write"
    );

    // A malformed value written directly resolves to the default rather than failing the import.
    crate::settings::set_langfuse_import_range_repo(&c, "last_7d".into()).unwrap();
    c.execute(
        "UPDATE settings SET value = 'garbage' WHERE key = 'langfuse_import_range'",
        [],
    )
    .unwrap();
    assert_eq!(
        crate::settings::resolve_import_range(&c),
        ImportRange::Last30d,
        "a malformed stored value resolves to the default"
    );
}
