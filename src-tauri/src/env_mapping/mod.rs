//! Environment → Vire-project mapping (project-env-mapping capability, TASK-027 D / DEC-028).
//!
//! Realizes the architecture plan's environment-first `PROJECT_MAPPING`: each discovered Langfuse
//! environment can be mapped to an existing Vire project. The mapping is **Vire-authoritative**
//! (DEC-001) — the environment is an external key; the project record is the source of truth — and
//! is **suggestion-first** (DEC-006): a project is created only by an explicit user action through
//! the normal `create_project` path, never auto-created or auto-mapped here.
//!
//! The additive `langfuse_env_project_map` table holds only `(environment, project_id, timestamps)`;
//! imported evidence is associated with its project at **read time** via a join (no evidence row is
//! ever rewritten). Every surface this module produces carries only environment names, project
//! references, and mapping state — no credential, raw payload, or trace content (SEC-010).

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::langfuse::store::list_discovered_environments;

type CmdResult<T> = Result<T, String>;

/// Local wall-clock timestamp, matching the core `projects`/`time_entries` convention in `lib.rs`.
fn now() -> String {
    chrono::Local::now()
        .naive_local()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Create the mapping table if absent (idempotent, additive). Called from `init_db` **after** the
/// `projects` table exists so the `project_id` foreign key resolves. Reverting to a prior build
/// simply leaves this table unused — no destructive migration.
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS langfuse_env_project_map (
            environment TEXT PRIMARY KEY,
            project_id  TEXT NOT NULL REFERENCES projects(id),
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );",
    )
}

/// One environment → project mapping row, joined to the project's current name for display. Carries
/// no secret — environment name, project reference, and timestamps only.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EnvMapping {
    pub environment: String,
    pub project_id: String,
    pub project_name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A discovered environment with its mapping state, for the Settings surface. `mapped == false`
/// (and `project_id == None`) is the "suggest creating a project for `<environment>`" case — the
/// app surfaces the suggestion but creates nothing until the user explicitly accepts (DEC-006).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DiscoveredEnvState {
    pub environment: String,
    pub last_seen: String,
    pub mapped: bool,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
}

/// Imported AI evidence associated with its mapped project at **read time** (D3). `project_id` is
/// `None` for an unmapped environment. No token/cost/session value is included — only the keys and
/// the joined project reference (SEC-010).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvidenceProject {
    pub environment: String,
    pub trace_id: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
}

fn project_exists(conn: &Connection, project_id: &str) -> CmdResult<bool> {
    conn.query_row(
        "SELECT 1 FROM projects WHERE id = ?1",
        params![project_id],
        |_| Ok(true),
    )
    .optional()
    .map_err(|e| e.to_string())
    .map(|v| v.unwrap_or(false))
}

fn get_env_mapping(conn: &Connection, environment: &str) -> CmdResult<EnvMapping> {
    conn.query_row(
        "SELECT m.environment, m.project_id, p.name, m.created_at, m.updated_at
           FROM langfuse_env_project_map m
           JOIN projects p ON p.id = m.project_id
          WHERE m.environment = ?1",
        params![environment],
        |r| {
            Ok(EnvMapping {
                environment: r.get(0)?,
                project_id: r.get(1)?,
                project_name: r.get(2)?,
                created_at: r.get(3)?,
                updated_at: r.get(4)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

/// List all environment → project mappings, ordered by environment.
pub fn list_env_mappings_repo(conn: &Connection) -> CmdResult<Vec<EnvMapping>> {
    let mut stmt = conn
        .prepare(
            "SELECT m.environment, m.project_id, p.name, m.created_at, m.updated_at
               FROM langfuse_env_project_map m
               JOIN projects p ON p.id = m.project_id
              ORDER BY m.environment",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(EnvMapping {
                environment: r.get(0)?,
                project_id: r.get(1)?,
                project_name: r.get(2)?,
                created_at: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// Map an environment to an **existing** project (upsert on `environment`). The project must already
/// exist — this function never creates one (DEC-001/DEC-006); creation goes through the normal
/// `create_project` path and the caller then maps the result. Re-mapping an environment updates the
/// `project_id` and `updated_at` only; `created_at` is preserved. Changing a mapping does not touch
/// any evidence row (read-time join, D3).
pub fn set_env_mapping_repo(
    conn: &Connection,
    environment: String,
    project_id: String,
) -> CmdResult<EnvMapping> {
    let environment = environment.trim().to_string();
    if environment.is_empty() {
        return Err("Environment cannot be empty".into());
    }
    if !project_exists(conn, &project_id)? {
        return Err("Cannot map an environment to a project that does not exist".into());
    }
    let ts = now();
    conn.execute(
        "INSERT INTO langfuse_env_project_map (environment, project_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(environment) DO UPDATE SET
            project_id = excluded.project_id,
            updated_at = excluded.updated_at",
        params![environment, project_id, ts],
    )
    .map_err(|e| e.to_string())?;
    get_env_mapping(conn, &environment)
}

/// Remove an environment's mapping. Idempotent — clearing an unmapped environment is a no-op. The
/// previously imported evidence rows are preserved; only the association changes (read-time join).
pub fn clear_env_mapping_repo(conn: &Connection, environment: String) -> CmdResult<()> {
    conn.execute(
        "DELETE FROM langfuse_env_project_map WHERE environment = ?1",
        params![environment.trim()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// List every environment that needs a project mapping, with its mapping state, for the Settings
/// surface (TASK-045 A / DEC-038). The environment universe is the **union** of three additive,
/// secret-free tables already in this DB:
///
/// - `langfuse_discovered_environments` — names the recent discovery scan enumerated (today's only
///   source);
/// - distinct `langfuse_ai_evidence.environment` — **the fix**: every environment that has imported
///   evidence must be mappable, so a backfilled environment older than the discovery window surfaces
///   with **no re-import**;
/// - distinct `langfuse_env_project_map.environment` — an already-mapped environment stays visible
///   (with its Clear action) even after its traces age out of the discovery scan.
///
/// `last_seen` is best-known display metadata: the discovered `last_seen` when present, else the most
/// recent evidence timestamp (`MAX(ai_end_ts)`, else `MAX(ai_start_ts)`), else an empty string — a
/// missing timestamp never drops a row. Output is de-duplicated by environment and ordered by
/// environment (today's deterministic render order). The shape stays `Vec<DiscoveredEnvState>` — no
/// IPC/TS contract change. An environment present but unmapped comes back `mapped == false` (the
/// suggest-create case); no project is created as a side effect of reading this list (DEC-006).
pub fn list_discovered_environments_repo(conn: &Connection) -> CmdResult<Vec<DiscoveredEnvState>> {
    // Build the environment → best-known `last_seen` universe from the three sources, applying the
    // precedence discovered → evidence → empty. A `BTreeMap` keeps each environment once and yields
    // them in sorted order — the same deterministic order as today's `ORDER BY environment`.
    let mut last_seen_by_env: BTreeMap<String, String> = BTreeMap::new();

    // Source 1 (highest precedence): the discovered names + their recorded `last_seen`, read through
    // the canonical discovered-environments accessor.
    for d in list_discovered_environments(conn).map_err(|e| e.to_string())? {
        last_seen_by_env.insert(d.environment, d.last_seen);
    }

    // Source 2 — **the fix**: every environment that has imported AI evidence must be mappable, so a
    // backfilled environment older than the discovery window surfaces with no re-import. `last_seen`
    // derives from the evidence (`MAX(ai_end_ts)`, else `MAX(ai_start_ts)`); `or_insert` means a
    // discovered `last_seen` already in the map wins.
    {
        let mut stmt = conn
            .prepare(
                "SELECT environment, COALESCE(MAX(ai_end_ts), MAX(ai_start_ts), '')
                   FROM langfuse_ai_evidence
                  GROUP BY environment",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for (environment, last_seen) in rows {
            last_seen_by_env.entry(environment).or_insert(last_seen);
        }
    }

    // Source 3: every already-mapped environment stays visible (with its Clear action) even after its
    // traces age out of both the discovery scan and the evidence. No timestamp is available, so its
    // `last_seen` is empty — display-only metadata that never blocks the row from rendering.
    {
        let mut stmt = conn
            .prepare("SELECT DISTINCT environment FROM langfuse_env_project_map")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for environment in rows {
            last_seen_by_env.entry(environment).or_default();
        }
    }

    let mut out = Vec::with_capacity(last_seen_by_env.len());
    for (environment, last_seen) in last_seen_by_env {
        // Same per-environment mapping join the current code does — `langfuse_env_project_map ⋈
        // projects` — unchanged, so `mapped`/`project_id`/`project_name` carry the same meaning.
        let mapping: Option<(String, String)> = conn
            .query_row(
                "SELECT m.project_id, p.name
                   FROM langfuse_env_project_map m
                   JOIN projects p ON p.id = m.project_id
                  WHERE m.environment = ?1",
                params![environment],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let (mapped, project_id, project_name) = match mapping {
            Some((pid, pname)) => (true, Some(pid), Some(pname)),
            None => (false, None, None),
        };
        out.push(DiscoveredEnvState {
            environment,
            last_seen,
            mapped,
            project_id,
            project_name,
        });
    }
    Ok(out)
}

/// Associate imported AI evidence with its mapped project at **read time** via a LEFT JOIN on the
/// env→project map (D3) — no evidence row is rewritten (DEC-001). Evidence for an unmapped
/// environment returns `project_id = None`; changing or clearing a mapping changes only this join
/// result. Exposes only the keys and the project reference — never a token/cost/session value.
pub fn list_evidence_projects_repo(conn: &Connection) -> CmdResult<Vec<EvidenceProject>> {
    let mut stmt = conn
        .prepare(
            "SELECT e.environment, e.trace_id, m.project_id, p.name
               FROM langfuse_ai_evidence e
               LEFT JOIN langfuse_env_project_map m ON m.environment = e.environment
               LEFT JOIN projects p ON p.id = m.project_id
              ORDER BY e.environment, e.trace_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(EvidenceProject {
                environment: r.get(0)?,
                trace_id: r.get(1)?,
                project_id: r.get(2)?,
                project_name: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[cfg(test)]
mod tests;
