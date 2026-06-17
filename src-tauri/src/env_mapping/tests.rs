//! TASK-027 D — env→project mapping unit tests. No network: the mapping layer is pure SQLite over
//! the additive `langfuse_env_project_map` table plus read-time joins against discovered
//! environments and imported evidence. Covers idempotent init, map persistence, suggest-create
//! (explicit-action only, never silent), the read-time evidence→project join, and secret-free
//! surfaces (SEC-010).

use rusqlite::Connection;

use super::*;
use crate::langfuse::model::{AiEvidence, HealthState};
use crate::langfuse::store::{upsert_ai_evidence, upsert_discovered_environment};
use crate::{create_project_repo, init_db, ProjectInput};

/// A fully-migrated in-memory DB (projects + langfuse + env_mapping tables) via the real `init_db`.
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

fn project_count(c: &Connection) -> i64 {
    c.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn migrate_is_idempotent_and_additive() {
    let c = conn();
    // init_db already ran migrate once; running it again must be a no-op, never an error.
    super::migrate(&c).unwrap();
    super::migrate(&c).unwrap();
    // The table exists and is empty on a fresh install.
    assert!(list_env_mappings_repo(&c).unwrap().is_empty());
}

#[test]
fn set_persists_a_mapping_and_list_reads_it_back() {
    let c = conn();
    let pid = make_project(&c, "Alpha");
    let mapping = set_env_mapping_repo(&c, "vire".into(), pid.clone()).unwrap();
    assert_eq!(mapping.environment, "vire");
    assert_eq!(mapping.project_id, pid);
    assert_eq!(mapping.project_name, "Alpha");

    let all = list_env_mappings_repo(&c).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].environment, "vire");
    assert_eq!(all[0].project_id, pid);
}

#[test]
fn remapping_updates_project_but_preserves_created_at() {
    let c = conn();
    let p1 = make_project(&c, "Alpha");
    let p2 = make_project(&c, "Beta");
    let first = set_env_mapping_repo(&c, "vire".into(), p1).unwrap();
    let second = set_env_mapping_repo(&c, "vire".into(), p2.clone()).unwrap();
    assert_eq!(
        second.project_id, p2,
        "the environment now maps to the new project"
    );
    assert_eq!(
        second.created_at, first.created_at,
        "created_at is preserved across a re-map"
    );
    // Still exactly one row for the environment (upsert, not insert).
    assert_eq!(list_env_mappings_repo(&c).unwrap().len(), 1);
}

#[test]
fn clear_is_idempotent() {
    let c = conn();
    let pid = make_project(&c, "Alpha");
    set_env_mapping_repo(&c, "vire".into(), pid).unwrap();
    clear_env_mapping_repo(&c, "vire".into()).unwrap();
    assert!(list_env_mappings_repo(&c).unwrap().is_empty());
    // Clearing an already-unmapped environment is a no-op, never an error.
    clear_env_mapping_repo(&c, "vire".into()).unwrap();
}

#[test]
fn mapping_to_a_missing_project_is_refused_and_creates_nothing() {
    let c = conn();
    let err = set_env_mapping_repo(&c, "vire".into(), "no-such-project".into()).unwrap_err();
    assert!(err.contains("does not exist"));
    assert_eq!(
        project_count(&c),
        0,
        "a failed map must not create a project (DEC-001)"
    );
    assert!(list_env_mappings_repo(&c).unwrap().is_empty());
}

#[test]
fn discovered_unmapped_suggests_create_then_explicit_action_maps_it() {
    let c = conn();
    // An environment is discovered but no project exists for it yet.
    upsert_discovered_environment(&c, "vire", "2026-06-05T00:00:00Z").unwrap();
    let before = list_discovered_environments_repo(&c).unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].environment, "vire");
    assert!(
        !before[0].mapped,
        "an unmapped discovered env is the suggest-create case"
    );
    assert!(before[0].project_id.is_none());
    // CRITICAL (DEC-006): reading the suggestion list never auto-creates a project.
    assert_eq!(
        project_count(&c),
        0,
        "no project is auto-created by surfacing a suggestion"
    );

    // The user explicitly accepts: create the project via the normal path, then map it.
    let pid = make_project(&c, "Vire");
    set_env_mapping_repo(&c, "vire".into(), pid.clone()).unwrap();

    let after = list_discovered_environments_repo(&c).unwrap();
    assert!(
        after[0].mapped,
        "after the explicit action the environment is mapped"
    );
    assert_eq!(after[0].project_id.as_deref(), Some(pid.as_str()));
    assert_eq!(after[0].project_name.as_deref(), Some("Vire"));
}

#[test]
fn evidence_is_associated_to_a_project_at_read_time_without_rewrite() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    // Two evidence rows: one in a mapped env, one in an unmapped env.
    let run_id = "run-1";
    upsert_ai_evidence(
        &c,
        &AiEvidence {
            trace_id: "T1".into(),
            environment: "vire".into(),
            session_id: Some("session-abc".into()),
            ai_start_ts: None,
            ai_end_ts: None,
            prompt_tokens: Some(4),
            completion_tokens: Some(6),
            total_tokens: Some(10),
            cost_total: Some(1.5),
            health: HealthState::Healthy,
        },
        run_id,
    )
    .unwrap();
    upsert_ai_evidence(
        &c,
        &AiEvidence {
            trace_id: "T2".into(),
            environment: "default".into(),
            session_id: None,
            ai_start_ts: None,
            ai_end_ts: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            cost_total: None,
            health: HealthState::WrongEnv,
        },
        run_id,
    )
    .unwrap();
    set_env_mapping_repo(&c, "vire".into(), pid.clone()).unwrap();

    let joined = list_evidence_projects_repo(&c).unwrap();
    let t1 = joined.iter().find(|e| e.trace_id == "T1").unwrap();
    let t2 = joined.iter().find(|e| e.trace_id == "T2").unwrap();
    assert_eq!(
        t1.project_id.as_deref(),
        Some(pid.as_str()),
        "mapped env's evidence joins to its project"
    );
    assert!(
        t2.project_id.is_none(),
        "unmapped env's evidence has no project at read time"
    );

    // Clearing the mapping changes only the join result — the evidence rows are untouched.
    clear_env_mapping_repo(&c, "vire".into()).unwrap();
    let after = list_evidence_projects_repo(&c).unwrap();
    assert!(after
        .iter()
        .find(|e| e.trace_id == "T1")
        .unwrap()
        .project_id
        .is_none());
    let surviving: i64 = c
        .query_row("SELECT COUNT(*) FROM langfuse_ai_evidence", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        surviving, 2,
        "evidence rows are preserved across a mapping change (no rewrite)"
    );
}

#[test]
fn mapping_surfaces_carry_no_secrets() {
    let c = conn();
    let pid = make_project(&c, "Vire");
    // An evidence row carrying a real session id, token counts, and cost — none of which the join
    // surface is allowed to expose (SEC-010): the join returns only keys + the project reference.
    upsert_ai_evidence(
        &c,
        &AiEvidence {
            trace_id: "T1".into(),
            environment: "vire".into(),
            session_id: Some("session-Bearer-leak".into()),
            ai_start_ts: None,
            ai_end_ts: None,
            prompt_tokens: Some(123),
            completion_tokens: Some(456),
            total_tokens: Some(579),
            cost_total: Some(9.99),
            health: HealthState::Healthy,
        },
        "run-1",
    )
    .unwrap();
    upsert_discovered_environment(&c, "vire", "2026-06-05T00:00:00Z").unwrap();
    set_env_mapping_repo(&c, "vire".into(), pid).unwrap();

    // Serialize every renderer-facing surface this module produces and assert none leaks the session
    // id, token, or cost value (only env names, project refs, and state may cross the boundary).
    let surfaces = format!(
        "{}{}{}",
        serde_json::to_string(&list_evidence_projects_repo(&c).unwrap()).unwrap(),
        serde_json::to_string(&list_discovered_environments_repo(&c).unwrap()).unwrap(),
        serde_json::to_string(&list_env_mappings_repo(&c).unwrap()).unwrap(),
    );
    for needle in ["session-", "Bearer", "leak", "sk-", "pk-", "9.99", "579"] {
        assert!(
            !surfaces.contains(needle),
            "mapping/discovery surface must be secret-free (SEC-010), found {needle}"
        );
    }
}
