//! Observer configuration: the configurable runtime-log path (so the source is an *optional*
//! signal, never a hard pi-observe dependency), the read-byte cap, the match slop, and
//! project→environment resolution. No credentials, no network targets — the observer reaches
//! nothing off-host.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::langfuse::config::ImporterConfig;

/// Hard cap on bytes read from the session log. A file larger than this is refused (treated as no
/// usable runtime evidence), never partially ingested. Mirrors pi-observe's bounded marker reads.
pub const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;

/// Default bounded clock slop for env+time-window matching (seconds). Tolerates skew between the
/// local session clock and the imported trace timestamps without making matches promiscuous.
pub const DEFAULT_MATCH_SLOP_SECS: i64 = 300;

/// pi-observe's default state sub-path under `$HOME` when `PI_OBSERVE_STATE_DIR` is unset.
const DEFAULT_STATE_SUBPATH: &str = ".local/state/pi-observe/events.jsonl";

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Resolved runtime-log path, or `None` when no source can be located (→ no runtime evidence).
    pub log_path: Option<PathBuf>,
    /// Optional explicit `project_key → environment` overrides.
    pub env_map: HashMap<String, String>,
    /// The importer's allowed environments — a `project_key` equal to one of these resolves to it.
    pub allowed_environments: Vec<String>,
    pub slop_secs: i64,
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        RuntimeConfig {
            log_path: resolve_log_path(),
            env_map: parse_env_map(env_nonempty("VIRE_RUNTIME_ENV_MAP")),
            allowed_environments: ImporterConfig::public_from_env().allowed_environments,
            slop_secs: env_nonempty("VIRE_RUNTIME_MATCH_SLOP_SECS")
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|v| *v >= 0)
                .unwrap_or(DEFAULT_MATCH_SLOP_SECS),
        }
    }

    /// Resolve a session's safe project token to an importer environment. Explicit map wins; else a
    /// token that *is* an allowed environment maps to itself; else `None` (→ `unmatched_runtime`).
    /// No inference beyond these — an unmappable session is surfaced for review, never guessed.
    pub fn resolve_environment(&self, project_key: &str) -> Option<String> {
        let key = project_key.trim();
        if key.is_empty() {
            return None;
        }
        if let Some(env) = self.env_map.get(key) {
            return Some(env.clone());
        }
        if self.allowed_environments.iter().any(|e| e == key) {
            return Some(key.to_string());
        }
        None
    }
}

/// `VIRE_RUNTIME_LOG_PATH` (explicit) → `$PI_OBSERVE_STATE_DIR/events.jsonl` → `$HOME/<default>`.
/// Returns `None` when none can be formed — absence of a path is a *state*, never a conclusion.
fn resolve_log_path() -> Option<PathBuf> {
    if let Some(explicit) = env_nonempty("VIRE_RUNTIME_LOG_PATH") {
        return Some(PathBuf::from(explicit));
    }
    if let Some(state_dir) = env_nonempty("PI_OBSERVE_STATE_DIR") {
        return Some(PathBuf::from(state_dir).join("events.jsonl"));
    }
    env_nonempty("HOME").map(|home| PathBuf::from(home).join(DEFAULT_STATE_SUBPATH))
}

fn parse_env_map(raw: Option<String>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(raw) = raw else {
        return map;
    };
    for pair in raw.split(',') {
        if let Some((k, v)) = pair.split_once('=') {
            let (k, v) = (k.trim(), v.trim());
            if !k.is_empty() && !v.is_empty() {
                map.insert(k.to_string(), v.to_string());
            }
        }
    }
    map
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
