//! TASK-032 A — suggestion engine unit tests (design §7-A). No network: the engine is pure SQLite
//! over imported evidence + the read-time env→project join. Covers gap clustering (at/over/under),
//! multi-session blocks, absence-≠-zero for untimed rows and absent tokens/cost, unmapped exclusion +
//! reporting, mixed-currency cost nulling, idempotent regeneration that preserves accepted/dismissed
//! blocks, the no-auto-post guarantee (DEC-006), and secret-free surfaces (SEC-012).
//!
//! Timestamps use offset-less local wall-clock strings so local dates/durations are deterministic on
//! any machine timezone; the engine's RFC-3339 path (imported `Z` timestamps) is exercised by the
//! `parse_local` accepting-both contract in `engine.rs`.

use rusqlite::{params, Connection};

use super::*;
use crate::env_mapping::set_env_mapping_repo;
use crate::{create_project_repo, init_db, ProjectInput};

/// A fully-migrated in-memory DB (projects + langfuse + env_mapping + suggestions) via real `init_db`.
fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    init_db(&c).unwrap();
    c
}

fn make_project(c: &Connection, name: &str) -> String {
    create_project_repo(
        c,
        ProjectInput {
            name: name.into(),
            notes: None,
        },
    )
    .unwrap()
    .id
}

/// Map an environment to a project so its evidence joins to a block (else it is "unmapped").
fn map_env(c: &Connection, env: &str, project_id: &str) {
    set_env_mapping_repo(c, env.into(), project_id.into()).unwrap();
}

#[allow(clippy::too_many_arguments)]
fn add_evidence(
    c: &Connection,
    env: &str,
    trace_id: &str,
    session: Option<&str>,
    start: Option<&str>,
    end: Option<&str>,
    total_tokens: Option<i64>,
    cost: Option<f64>,
    currency: Option<&str>,
    health: &str,
) {
    c.execute(
        "INSERT INTO langfuse_ai_evidence
            (environment, trace_id, session_id, ai_start_ts, ai_end_ts, prompt_tokens,
             completion_tokens, total_tokens, cost_total, cost_currency, health, import_run_id)
         VALUES (?1,?2,?3,?4,?5,NULL,NULL,?6,?7,?8,?9,'run-test')",
        params![
            env,
            trace_id,
            session,
            start,
            end,
            total_tokens,
            cost,
            currency,
            health
        ],
    )
    .unwrap();
}

fn count(c: &Connection, sql: &str) -> i64 {
    c.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn migrate_is_idempotent() {
    let c = conn();
    super::store::migrate(&c).unwrap();
    super::store::migrate(&c).unwrap();
    assert_eq!(count(&c, "SELECT COUNT(*) FROM time_entry_suggestions"), 0);
}

#[test]
fn empty_evidence_yields_no_suggestions_and_no_unmapped() {
    let c = conn();
    let out = generate(&c).unwrap();
    assert!(out.suggestions.is_empty());
    assert!(out.unmapped.is_empty());
}

#[test]
fn timed_rows_within_one_window_form_a_single_block_with_aggregates() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:30:00"),
        Some(10),
        Some(1.5),
        None,
        "healthy",
    );
    add_evidence(
        &c,
        "veronavi",
        "t2",
        Some("s2"),
        Some("2026-06-05 09:40:00"),
        Some("2026-06-05 10:00:00"),
        Some(20),
        Some(2.5),
        None,
        "healthy",
    );

    let out = generate(&c).unwrap();
    assert_eq!(
        out.suggestions.len(),
        1,
        "two near rows cluster into one block"
    );
    let s = &out.suggestions[0];
    assert_eq!(s.project_name, "Vire");
    assert_eq!(s.date, "2026-06-05");
    assert_eq!(s.trace_count, 2);
    assert_eq!(s.session_count, 2, "two distinct sessions");
    assert_eq!(s.duration_minutes, Some(60), "09:00–10:00 == 60 min");
    assert_eq!(s.total_tokens, Some(30));
    assert_eq!(s.cost_total, Some(4.0));
    assert_eq!(s.health, "healthy");
    assert_eq!(s.confidence, "high", "all timed + healthy + single env");
    assert_eq!(s.source, "langfuse:veronavi");
    assert!(s.block_start_ts.is_some() && s.block_end_ts.is_some());
}

#[test]
fn clustering_respects_the_gap_boundary() {
    // Exactly at the 30-min gap → one block; one second over → two blocks; under → one block.
    for (case, second_start, second_end, expected_blocks) in [
        ("at 30:00", "2026-06-05 10:00:00", "2026-06-05 10:10:00", 1),
        (
            "over 30:01",
            "2026-06-05 10:00:01",
            "2026-06-05 10:10:00",
            2,
        ),
        (
            "under 29:00",
            "2026-06-05 09:59:00",
            "2026-06-05 10:10:00",
            1,
        ),
    ] {
        let c = conn();
        let pid = make_project(&c, "Vire");
        map_env(&c, "veronavi", &pid);
        add_evidence(
            &c,
            "veronavi",
            "t1",
            Some("s1"),
            Some("2026-06-05 09:00:00"),
            Some("2026-06-05 09:30:00"),
            Some(1),
            None,
            None,
            "healthy",
        );
        add_evidence(
            &c,
            "veronavi",
            "t2",
            Some("s1"),
            Some(second_start),
            Some(second_end),
            Some(1),
            None,
            None,
            "healthy",
        );
        let out = generate(&c).unwrap();
        assert_eq!(
            out.suggestions.len(),
            expected_blocks,
            "gap case {case}: expected {expected_blocks} block(s)"
        );
    }
}

#[test]
fn untimed_rows_yield_unknown_duration_never_zero() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    // Rows with a start but no end cannot form a duration → "needs manual time", never a 0-min block.
    // Both carry a start (so they share the same local-date bucket) but no end.
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        None,
        Some(5),
        None,
        None,
        "healthy",
    );
    add_evidence(
        &c,
        "veronavi",
        "t2",
        Some("s2"),
        Some("2026-06-05 14:00:00"),
        None,
        None,
        None,
        None,
        "healthy",
    );

    let out = generate(&c).unwrap();
    assert_eq!(
        out.suggestions.len(),
        1,
        "untimed rows in one (project, date) bucket fold into one block"
    );
    let s = &out.suggestions[0];
    assert_eq!(s.trace_count, 2);
    assert!(
        s.duration_minutes.is_none(),
        "unknown duration is NULL, never 0"
    );
    assert!(s.block_start_ts.is_none() && s.block_end_ts.is_none());
    assert_eq!(s.confidence, "low");
    assert!(s.reason.contains("needs manual time"));
    assert_eq!(
        s.total_tokens,
        Some(5),
        "present token survives even when untimed"
    );
}

#[test]
fn absent_tokens_and_cost_stay_null_not_zero() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:20:00"),
        None,
        None,
        None,
        "healthy",
    );

    let out = generate(&c).unwrap();
    let s = &out.suggestions[0];
    assert!(
        s.total_tokens.is_none(),
        "no tokens anywhere ⇒ NULL, never 0"
    );
    assert!(s.cost_total.is_none(), "no cost anywhere ⇒ NULL, never 0");
    assert!(s.cost_currency.is_none());
}

#[test]
fn unmapped_environment_evidence_is_excluded_and_reported() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:30:00"),
        Some(10),
        None,
        None,
        "healthy",
    );
    // Two traces in an environment that maps to no project.
    add_evidence(
        &c,
        "ghost",
        "g1",
        Some("s9"),
        None,
        None,
        None,
        None,
        None,
        "wrong_env",
    );
    add_evidence(
        &c,
        "ghost",
        "g2",
        Some("s9"),
        None,
        None,
        None,
        None,
        None,
        "wrong_env",
    );

    let out = generate(&c).unwrap();
    assert_eq!(
        out.suggestions.len(),
        1,
        "only the mapped env yields a block"
    );
    assert!(out.suggestions.iter().all(|s| s.project_id == pid));
    assert_eq!(out.unmapped.len(), 1);
    assert_eq!(out.unmapped[0].environment, "ghost");
    assert_eq!(
        out.unmapped[0].trace_count, 2,
        "unmapped counted, never dropped"
    );
}

#[test]
fn single_currency_sums_but_mixed_currency_nulls_the_cost() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:10:00"),
        None,
        Some(1.0),
        Some("USD"),
        "healthy",
    );
    add_evidence(
        &c,
        "veronavi",
        "t2",
        Some("s1"),
        Some("2026-06-05 09:12:00"),
        Some("2026-06-05 09:20:00"),
        None,
        Some(2.0),
        Some("EUR"),
        "healthy",
    );

    let out = generate(&c).unwrap();
    assert_eq!(out.suggestions.len(), 1);
    let s = &out.suggestions[0];
    assert!(
        s.cost_total.is_none(),
        "mixed currencies ⇒ cost NULL, not a wrong sum"
    );
    assert!(s.cost_currency.is_none());
    assert!(s.reason.contains("mixed currencies"));
}

#[test]
fn degraded_health_lowers_confidence_and_worst_health_wins() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:10:00"),
        Some(1),
        None,
        None,
        "healthy",
    );
    add_evidence(
        &c,
        "veronavi",
        "t2",
        Some("s1"),
        Some("2026-06-05 09:12:00"),
        Some("2026-06-05 09:20:00"),
        Some(1),
        None,
        None,
        "schema_changed",
    );

    let out = generate(&c).unwrap();
    let s = &out.suggestions[0];
    assert_eq!(
        s.health, "schema_changed",
        "worst health across the block wins"
    );
    assert_eq!(s.confidence, "medium", "any degraded health ⇒ not high");
}

#[test]
fn regeneration_preserves_accepted_and_dismissed_and_does_not_duplicate() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    // One timed block and one untimed block (start, no end) on the same day.
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:30:00"),
        Some(10),
        None,
        None,
        "healthy",
    );
    add_evidence(
        &c,
        "veronavi",
        "t2",
        Some("s2"),
        Some("2026-06-05 14:00:00"),
        None,
        None,
        None,
        None,
        "healthy",
    );

    let first = generate(&c).unwrap();
    assert_eq!(first.suggestions.len(), 2);
    let timed = first
        .suggestions
        .iter()
        .find(|s| s.block_start_ts.is_some())
        .unwrap();
    let untimed = first
        .suggestions
        .iter()
        .find(|s| s.block_start_ts.is_none())
        .unwrap();

    // Decide both: accept the timed one, dismiss the untimed one.
    c.execute(
        "UPDATE time_entry_suggestions SET status='accepted', accepted_entry_id='entry-1' WHERE id=?1",
        params![timed.id],
    )
    .unwrap();
    c.execute(
        "UPDATE time_entry_suggestions SET status='dismissed' WHERE id=?1",
        params![untimed.id],
    )
    .unwrap();

    // Regenerate over the same evidence.
    let second = generate(&c).unwrap();
    assert!(
        second.suggestions.is_empty(),
        "decided blocks are not re-proposed as new pending duplicates"
    );
    assert_eq!(
        count(
            &c,
            "SELECT COUNT(*) FROM time_entry_suggestions WHERE status='accepted'"
        ),
        1,
        "the accepted block survives regeneration"
    );
    assert_eq!(
        count(
            &c,
            "SELECT COUNT(*) FROM time_entry_suggestions WHERE status='dismissed'"
        ),
        1,
        "the dismissed block survives regeneration"
    );
    assert_eq!(
        count(&c, "SELECT COUNT(*) FROM time_entry_suggestions"),
        2,
        "no duplicate rows for the decided natural keys"
    );
    // The accepted entry id is untouched by regeneration.
    assert_eq!(
        c.query_row(
            "SELECT accepted_entry_id FROM time_entry_suggestions WHERE status='accepted'",
            [],
            |r| r.get::<_, Option<String>>(0)
        )
        .unwrap(),
        Some("entry-1".to_string())
    );
}

#[test]
fn failed_regeneration_preserves_the_original_pending_set() {
    // Atomic replace-set (SW-4 B1): regeneration deletes the prior pending rows, re-inserts the
    // recomputed blocks, and reads the fresh set inside ONE transaction. If any insert fails the whole
    // replace rolls back, so the ORIGINAL pending set survives — Refresh is never destructive under a
    // partial write.
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    // Two timed blocks on one day (a >30-min gap splits them) → two pending suggestions.
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:30:00"),
        Some(10),
        None,
        None,
        "healthy",
    );
    add_evidence(
        &c,
        "veronavi",
        "t2",
        Some("s2"),
        Some("2026-06-05 11:00:00"),
        Some("2026-06-05 11:30:00"),
        Some(20),
        None,
        None,
        "healthy",
    );

    // First generation establishes the original pending set.
    let first = generate(&c).unwrap();
    assert_eq!(first.suggestions.len(), 2);
    let mut original_ids: Vec<String> = first.suggestions.iter().map(|s| s.id.clone()).collect();
    original_ids.sort();

    // Force the next regeneration's inserts to fail: a BEFORE INSERT trigger that always aborts. The
    // delete runs first inside the transaction, then the first insert aborts → the transaction is
    // dropped without commit and rolls back.
    c.execute_batch(
        "CREATE TRIGGER fail_suggestion_insert BEFORE INSERT ON time_entry_suggestions
         BEGIN SELECT RAISE(ABORT, 'forced insert failure'); END;",
    )
    .unwrap();

    let result = generate(&c);
    assert!(
        result.is_err(),
        "regeneration must fail when an insert aborts"
    );

    // The original pending set survives unchanged: same count, same ids — nothing partially replaced
    // and nothing left deleted.
    assert_eq!(
        count(
            &c,
            "SELECT COUNT(*) FROM time_entry_suggestions WHERE status='pending'"
        ),
        2,
        "failed regeneration must not leave a partial or emptied pending set"
    );
    let mut surviving_ids: Vec<String> = {
        let mut stmt = c
            .prepare("SELECT id FROM time_entry_suggestions WHERE status='pending'")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    surviving_ids.sort();
    assert_eq!(
        surviving_ids, original_ids,
        "the original pending rows survive a failed regeneration intact"
    );
}

#[test]
fn generation_never_writes_a_time_entry() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("s1"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:30:00"),
        Some(10),
        Some(1.0),
        Some("USD"),
        "healthy",
    );

    generate(&c).unwrap();
    assert_eq!(
        count(&c, "SELECT COUNT(*) FROM time_entries"),
        0,
        "no auto-posting: only accept (Workstream B) writes time_entries (DEC-006)"
    );
}

#[test]
fn surfaces_carry_no_secrets() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    map_env(&c, "veronavi", &pid);
    add_evidence(
        &c,
        "veronavi",
        "t1",
        Some("session-Bearer-sk-leak"),
        Some("2026-06-05 09:00:00"),
        Some("2026-06-05 09:30:00"),
        Some(123),
        Some(9.99),
        Some("USD"),
        "healthy",
    );

    let out = generate(&c).unwrap();
    let serialized = serde_json::to_string(&out).unwrap();
    for needle in ["session-", "Bearer", "sk-", "leak", "t1"] {
        assert!(
            !serialized.contains(needle),
            "suggestion surface must be secret-free (SEC-012); found {needle}"
        );
    }
    // Aggregate numbers (tokens/cost) are allowed and expected.
    assert!(serialized.contains("123") && serialized.contains("9.99"));
}
