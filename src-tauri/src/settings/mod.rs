//! In-app Langfuse settings (TASK-026 / DEC-026 / SEC-009).
//!
//! Non-secret settings (`base_url`, `source`, `environments`, `langfuse_enabled`) live in the
//! existing SQLite `settings` key/value table as additive rows. The Langfuse secret key (and the
//! credential-adjacent public key) live in the macOS Keychain behind [`SecretStore`]. The importer
//! resolves its config **settings-first**, with process env retained as a clearly-marked developer
//! fallback, then the existing code defaults. All credential material flows through the existing
//! redacting [`Secret`]/[`Credentials`] types — there is no second, un-redacted credential path.

pub mod secret_store;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::langfuse::config::{
    Credentials, ImporterConfig, Secret, Source, DEFAULT_BASE_URL, DEFAULT_ENVIRONMENT,
};
use crate::langfuse::store::{self, SourceHealthSnapshot};
use secret_store::{SecretStore, SecretStoreError};

/// Keychain account names (service = bundle id `dev.vire.app`); distinct entries per key.
pub const PUBLIC_KEY_ACCOUNT: &str = "langfuse_public_key";
pub const SECRET_KEY_ACCOUNT: &str = "langfuse_secret_key";

/// `settings` table keys for the non-secret Langfuse configuration. No credential is ever a key
/// or a value here (SEC-009).
const KEY_BASE_URL: &str = "langfuse_base_url";
const KEY_SOURCE: &str = "langfuse_source";
const KEY_ENVIRONMENTS: &str = "langfuse_environments";
const KEY_ENABLED: &str = "langfuse_enabled";

type CmdResult<T> = Result<T, String>;

/// Renderer-facing settings view: non-secret fields plus secret **presence** flags only. Never
/// carries any secret value (SEC-009) — the form shows "set / not set", never the stored key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LangfuseSettings {
    pub base_url: String,
    pub source: String,
    pub environments: Vec<String>,
    pub langfuse_enabled: bool,
    pub has_public_key: bool,
    pub has_secret_key: bool,
}

/// Non-secret settings written by the renderer. Credentials are set through a separate command so
/// no secret ever travels in or out of this struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangfuseSettingsInput {
    pub base_url: String,
    pub source: String,
    pub environments: Vec<String>,
    pub langfuse_enabled: bool,
}

/// Lookup of process environment variables, abstracted so the resolver is a pure function of
/// `(settings rows, keychain, env)` and tests inject a fixed env without racing on process globals.
pub trait EnvSource {
    fn get(&self, key: &str) -> Option<String>;
}

/// The real process environment (production). Trims and treats blank as unset, matching the
/// existing env loaders in `langfuse/config.rs`.
pub struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }
}

// ----- non-secret settings persistence ------------------------------------------------------

fn read_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
    .map(|v| v.trim().to_string())
    .filter(|v| !v.is_empty())
}

fn write_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn normalize_environments(values: &[String]) -> Vec<String> {
    let envs: Vec<String> = values
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if envs.is_empty() {
        vec![DEFAULT_ENVIRONMENT.to_string()]
    } else {
        envs
    }
}

fn parse_environments_csv(csv: &str) -> Vec<String> {
    normalize_environments(&csv.split(',').map(|s| s.to_string()).collect::<Vec<_>>())
}

// ----- config resolution (settings-first, env dev fallback, code defaults) ------------------

/// Resolve the master enable switch: stored setting → `VIRE_LANGFUSE_ENABLED` env → default `true`
/// (preserving the pre-TASK-026 behavior where the integration was always active).
pub fn langfuse_enabled(conn: &Connection) -> bool {
    langfuse_enabled_with(conn, &ProcessEnv)
}

pub fn langfuse_enabled_with(conn: &Connection, env: &dyn EnvSource) -> bool {
    match read_setting(conn, KEY_ENABLED) {
        Some(v) => parse_bool(&v),
        None => env
            .get("VIRE_LANGFUSE_ENABLED")
            .map(|v| parse_bool(&v))
            .unwrap_or(true),
    }
}

/// Non-secret config (no credentials) for the health snapshot. Settings-first, then env, then the
/// unchanged code defaults (`http://127.0.0.1:3000`, `local`, `vire`).
pub fn resolve_public_config(conn: &Connection) -> ImporterConfig {
    resolve_public_config_with(conn, &ProcessEnv)
}

pub fn resolve_public_config_with(conn: &Connection, env: &dyn EnvSource) -> ImporterConfig {
    let base_url = read_setting(conn, KEY_BASE_URL)
        .or_else(|| env.get("VIRE_LANGFUSE_BASE_URL"))
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let source = read_setting(conn, KEY_SOURCE)
        .or_else(|| env.get("VIRE_LANGFUSE_SOURCE"))
        .map(|s| Source::parse(&s))
        .unwrap_or(Source::Local);
    let environments = read_setting(conn, KEY_ENVIRONMENTS)
        .or_else(|| env.get("VIRE_LANGFUSE_ENVIRONMENTS"))
        .map(|csv| parse_environments_csv(&csv))
        .unwrap_or_else(|| vec![DEFAULT_ENVIRONMENT.to_string()]);
    ImporterConfig::new(base_url, source, environments, None)
}

/// Full config including credentials, for an import run or a Test connection probe. Credentials are
/// read from the Keychain first (per field), then the marked dev-fallback env vars. Both keys must
/// be present for credentials to be attached, mirroring the existing `from_env` contract. The
/// secret flows through the redacting [`Secret`]/[`Credentials`] types — never a raw second path.
///
/// A genuine secret-store backend failure (distinct from "no entry") is propagated as a coarse,
/// secret-free `Err` — it must **not** be silently treated as an absent credential, because that
/// would let the env dev fallback override a failed settings-first read (the resolver contract is
/// env-only-when-stored-credentials-are-absent).
pub fn resolve_config(conn: &Connection, secrets: &dyn SecretStore) -> CmdResult<ImporterConfig> {
    resolve_config_with(conn, secrets, &ProcessEnv)
}

pub fn resolve_config_with(
    conn: &Connection,
    secrets: &dyn SecretStore,
    env: &dyn EnvSource,
) -> CmdResult<ImporterConfig> {
    let mut config = resolve_public_config_with(conn, env);
    config.credentials = resolve_credentials(secrets, env).map_err(|e| e.0)?;
    Ok(config)
}

/// Resolve the credential pair. Each key is read from the secret store first; the env dev fallback
/// is consulted **only** when the store reports no entry (`Ok(None)`). A real read failure
/// (`Err`) short-circuits with a coarse, secret-free error so a Keychain outage can never be
/// mistaken for "no credential" and silently downgraded to the env fallback.
fn resolve_credentials(
    secrets: &dyn SecretStore,
    env: &dyn EnvSource,
) -> Result<Option<Credentials>, SecretStoreError> {
    let public_key = match secrets.get(PUBLIC_KEY_ACCOUNT)? {
        Some(value) => Some(value),
        None => env
            .get("VIRE_LANGFUSE_PUBLIC_KEY")
            .or_else(|| env.get("LANGFUSE_PUBLIC_KEY")),
    };
    let secret_key = match secrets.get(SECRET_KEY_ACCOUNT)? {
        Some(value) => Some(value),
        None => env
            .get("VIRE_LANGFUSE_SECRET_KEY")
            .or_else(|| env.get("LANGFUSE_SECRET_KEY")),
    };
    Ok(match (public_key, secret_key) {
        (Some(public_key), Some(secret_key)) => Some(Credentials {
            public_key,
            secret_key: Secret::new(secret_key),
        }),
        _ => None,
    })
}

// ----- IPC repo functions (secret-free) -----------------------------------------------------

/// Read the renderer-facing settings view. The secret store is consulted only for **presence**
/// (`has_public_key` / `has_secret_key`); no secret value is ever read back or returned.
pub fn get_langfuse_settings_repo(
    conn: &Connection,
    secrets: &dyn SecretStore,
) -> CmdResult<LangfuseSettings> {
    let config = resolve_public_config(conn);
    let has_public_key = secrets.get(PUBLIC_KEY_ACCOUNT).map_err(|e| e.0)?.is_some();
    let has_secret_key = secrets.get(SECRET_KEY_ACCOUNT).map_err(|e| e.0)?.is_some();
    Ok(LangfuseSettings {
        base_url: config.base_url,
        source: config.source.as_str().to_string(),
        environments: config.allowed_environments,
        langfuse_enabled: langfuse_enabled(conn),
        has_public_key,
        has_secret_key,
    })
}

/// Persist the non-secret settings (additive `settings` rows). Validates the base URL is a
/// well-formed http/https URL with a host; the loopback boundary (SEC-002) is enforced at
/// import/Test-connection time by `validate_target`, identically for settings- and env-sourced
/// values. Returns the refreshed view so the form re-renders from a single source.
pub fn set_langfuse_settings_repo(
    conn: &Connection,
    secrets: &dyn SecretStore,
    input: LangfuseSettingsInput,
) -> CmdResult<LangfuseSettings> {
    let base_url = input.base_url.trim().to_string();
    if base_url.is_empty() {
        return Err("Langfuse base URL cannot be empty".into());
    }
    let source = Source::parse(&input.source);
    // Reject a malformed URL up front with the existing secret-free parse error.
    ImporterConfig::new(base_url.clone(), source, Vec::new(), None)
        .parsed_base()
        .map_err(|e| e.0)?;
    let environments = normalize_environments(&input.environments);

    write_setting(conn, KEY_BASE_URL, &base_url).map_err(|e| e.to_string())?;
    write_setting(conn, KEY_SOURCE, source.as_str()).map_err(|e| e.to_string())?;
    write_setting(conn, KEY_ENVIRONMENTS, &environments.join(",")).map_err(|e| e.to_string())?;
    write_setting(
        conn,
        KEY_ENABLED,
        if input.langfuse_enabled {
            "true"
        } else {
            "false"
        },
    )
    .map_err(|e| e.to_string())?;

    get_langfuse_settings_repo(conn, secrets)
}

/// Store the credential pair in the Keychain. The secret is accepted *in* and never returned. Both
/// keys are entered together (one clean credential surface); both must be non-empty. The two
/// Keychain entries are written atomically: if the secret write fails the public entry is rolled
/// back to its prior state, so a failed (re)write never leaves exactly one entry — which would let
/// the resolver combine an env-fallback public key with a stale Keychain secret.
pub fn set_langfuse_secret_repo(
    secrets: &dyn SecretStore,
    public_key: String,
    secret_key: String,
) -> CmdResult<()> {
    let public_key = public_key.trim();
    let secret_key = secret_key.trim();
    if public_key.is_empty() {
        return Err("Public key cannot be empty".into());
    }
    if secret_key.is_empty() {
        return Err("Secret key cannot be empty".into());
    }
    // Capture the prior public-key entry first so a failed secret write can restore the exact prior
    // Keychain state. The two Keychain entries are one atomic credential pair: after this function
    // the public and secret entries must be EITHER both the new pair OR both the prior state — never
    // one entry deleted/replaced beside a stale other. If a failed *replacement* of an existing pair
    // deleted the public entry while the old secret survived, `resolve_credentials` would fill the
    // now-missing public key from the `VIRE_LANGFUSE_PUBLIC_KEY` / `LANGFUSE_PUBLIC_KEY` env fallback
    // and combine it with that stale Keychain secret — a mixed-source credential pair (DEC-026
    // credential-pair integrity). Reading the public key here exposes no secret (SEC-009 guards the
    // secret key only).
    let prior_public_key = secrets.get(PUBLIC_KEY_ACCOUNT).map_err(|e| e.0)?;
    secrets
        .set(PUBLIC_KEY_ACCOUNT, public_key)
        .map_err(|e| e.0)?;
    if let Err(e) = secrets.set(SECRET_KEY_ACCOUNT, secret_key) {
        // The secret write failed, so the secret entry is untouched (still the prior value, if any).
        // Restore the public entry to its prior state — put the previous value back when one existed
        // (reinstating the prior consistent pair), else remove the entry we just wrote (back to both
        // absent). Either way the pair is left consistent, so a failed replacement can never produce
        // a Keychain/env mixed pair. The restore is best-effort and idempotent; its own failure must
        // not mask the original secret-free error.
        match prior_public_key {
            Some(prior) => {
                let _ = secrets.set(PUBLIC_KEY_ACCOUNT, &prior);
            }
            None => {
                let _ = secrets.delete(PUBLIC_KEY_ACCOUNT);
            }
        }
        return Err(e.0);
    }
    Ok(())
}

/// Remove the credential pair from the Keychain. Idempotent.
pub fn clear_langfuse_secret_repo(secrets: &dyn SecretStore) -> CmdResult<()> {
    secrets.delete(PUBLIC_KEY_ACCOUNT).map_err(|e| e.0)?;
    secrets.delete(SECRET_KEY_ACCOUNT).map_err(|e| e.0)?;
    Ok(())
}

/// Outcome of resolving what a **Test connection** action should do. A disabled integration is
/// resolved **before** the secret store or network is touched, so the caller returns an explicit
/// disabled verdict without a Keychain read or probe; only an enabled integration carries the
/// resolved [`ImporterConfig`] (with its one-shot credential read) forward to the bounded probe.
/// `Debug` is safe to derive: the embedded `ImporterConfig`/`Credentials` redact key material in
/// their own `Debug` impls (`langfuse/config.rs`), so a plan never renders a secret.
#[derive(Debug)]
pub enum TestConnectionPlan {
    Disabled,
    Probe(ImporterConfig),
}

/// Decide what a Test connection should do. Mirrors the `import_langfuse_now` / health-snapshot
/// short-circuit: when `langfuse_enabled == false` this returns [`TestConnectionPlan::Disabled`]
/// **before** any `SecretStore` access (the secret store is consulted only on the `Probe` path), so
/// a disabled integration can never trigger a Keychain read or a network probe. A real Keychain
/// read failure on the enabled path surfaces as a coarse, secret-free `Err`.
pub fn test_connection_plan(
    conn: &Connection,
    secrets: &dyn SecretStore,
) -> CmdResult<TestConnectionPlan> {
    if !langfuse_enabled(conn) {
        return Ok(TestConnectionPlan::Disabled);
    }
    Ok(TestConnectionPlan::Probe(resolve_config(conn, secrets)?))
}

/// Read-only source health for `get_langfuse_source_health` and post-import. When the integration
/// is disabled this short-circuits to an explicit **disabled** snapshot **before** any network or
/// Keychain access — note this function takes no `SecretStore`, so the disabled (and the persisted-
/// state) path structurally cannot read a credential.
pub fn source_health_snapshot(conn: &Connection) -> CmdResult<SourceHealthSnapshot> {
    let config = resolve_public_config(conn);
    if !langfuse_enabled(conn) {
        return Ok(store::disabled_snapshot(&config));
    }
    store::source_health_snapshot(conn, &config)
}

#[cfg(test)]
mod tests;
