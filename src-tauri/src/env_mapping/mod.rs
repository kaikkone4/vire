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

/// List discovered environments with their mapping state (D2). An environment present in the
/// discovered set but absent from the map comes back `mapped == false` — the suggest-create case.
/// No project is created as a side effect of reading this list.
pub fn list_discovered_environments_repo(conn: &Connection) -> CmdResult<Vec<DiscoveredEnvState>> {
    let discovered = list_discovered_environments(conn).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(discovered.len());
    for d in discovered {
        let mapping: Option<(String, String)> = conn
            .query_row(
                "SELECT m.project_id, p.name
                   FROM langfuse_env_project_map m
                   JOIN projects p ON p.id = m.project_id
                  WHERE m.environment = ?1",
                params![d.environment],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let (mapped, project_id, project_name) = match mapping {
            Some((pid, pname)) => (true, Some(pid), Some(pname)),
            None => (false, None, None),
        };
        out.push(DiscoveredEnvState {
            environment: d.environment,
            last_seen: d.last_seen,
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
