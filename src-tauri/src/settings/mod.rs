//! In-app Langfuse settings (TASK-026 / DEC-026 / SEC-009; public key relocated in TASK-044 / F2a).
//!
//! Non-secret settings (`base_url`, `source`, `environments`, `langfuse_enabled`) live in the
//! existing SQLite `settings` key/value table as additive rows. The Langfuse **public** key (the
//! non-secret HTTP Basic-Auth username) now lives in that same `settings` table (TASK-044); only the
//! Langfuse **secret** key remains in the macOS Keychain behind [`SecretStore`]. Dropping the public
//! Keychain item removes one macOS Keychain-access prompt per fresh launch (2 → 1). The importer
//! resolves non-secret config **settings-first**, with process env retained as a clearly-marked
//! developer fallback, then the existing code defaults. The **credential pair** is resolved as a
//! unit (TASK-044 / arch-review Addendum Decision 1): both stores ⇒ the stored pair; neither store ⇒
//! a whole-pair env dev override (only when **both** env keys are set); exactly one store ⇒ `None`
//! (the lone key is discarded and env is not consulted), so a mixed-source pair is structurally
//! impossible. All credential material flows through the existing redacting
//! [`Secret`]/[`Credentials`] types — there is no second, un-redacted credential path.
//!
//! ## Cross-store consistency contract (arch-review Addendum Decision 2)
//!
//! The `settings` table and the Keychain are independent stores with no shared transaction, so true
//! cross-store atomicity is impossible. The app guarantees instead: (1) **error-atomicity** — every
//! returning `set`/`clear` leaves both-present (new or prior) or both-absent, *provided every local
//! SQLite op succeeds*; the fragile Keychain mutation is always performed **last** and compensated by
//! a reliable local SQLite op, and a strict-read failure aborts before any mutation. (2)
//! **catastrophic-failure disclosure** — the only one-store state reachable through a returned result
//! requires a local SQLite op to fail immediately after another succeeded (disk-full / corruption /
//! I/O); it is surfaced as a distinct, secret-free error (`INCONSISTENT_SET_ERR` /
//! `INCONSISTENT_CLEAR_ERR`) and the compensation error is never swallowed. (3) **no crash-atomicity,
//! made safe by the resolver** — a hard crash between the two mutations (or case 2) can leave one
//! store populated; Decision 1 renders it inert (one-store ⇒ `None`), fixed by one re-save. The
//! invariant the system depends on: *a one-store state is never a usable credential and never a
//! mixed-source pair.*

pub mod secret_store;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::langfuse::config::{
    Credentials, ImporterConfig, Secret, Source, DEFAULT_BASE_URL, DEFAULT_ENVIRONMENT,
};
use crate::langfuse::store::{self, SourceHealthSnapshot};
use crate::langfuse::ImportRange;
use secret_store::SecretStore;

/// Keychain account names (service = bundle id `dev.vire.app`). After TASK-044 only
/// [`SECRET_KEY_ACCOUNT`] is a live Keychain entry; [`PUBLIC_KEY_ACCOUNT`] is retained **only** to
/// best-effort delete the legacy public-key item written by pre-F2a builds (it is never read).
pub const PUBLIC_KEY_ACCOUNT: &str = "langfuse_public_key";
pub const SECRET_KEY_ACCOUNT: &str = "langfuse_secret_key";

/// `settings` table keys for the non-secret Langfuse configuration. The secret key is never a key
/// or a value here (SEC-009); the **public** key is non-secret and is stored here as of TASK-044.
const KEY_BASE_URL: &str = "langfuse_base_url";
const KEY_SOURCE: &str = "langfuse_source";
const KEY_ENVIRONMENTS: &str = "langfuse_environments";
const KEY_ENABLED: &str = "langfuse_enabled";
/// The non-secret Langfuse public key (Basic-Auth username), relocated from the Keychain into the
/// plaintext `settings` table (TASK-044 / F2a). The string deliberately matches the legacy Keychain
/// account name, but the two are different namespaces (a `settings` row key vs a Keychain account);
/// the Rust constants are kept distinct so the legacy-cleanup path stays unambiguous.
const KEY_PUBLIC_KEY: &str = "langfuse_public_key";
/// How far back imports reach (TASK-029 C). Non-secret, additive — one key-value row alongside the
/// other Langfuse settings; never a credential store.
const KEY_IMPORT_RANGE: &str = "langfuse_import_range";

/// Distinct, **secret-free** errors for the only one-store state reachable through a returned result
/// (TASK-044 / arch-review Addendum Decision 2): a local SQLite compensation op failing immediately
/// after the fragile Keychain mutation (disk-full / corruption / I/O). They are **never** swallowed,
/// so the user is told to re-save; the pair-level resolver renders the half-state inert meanwhile.
const INCONSISTENT_SET_ERR: &str =
    "Langfuse credentials are in an inconsistent state and could not be fully saved; please re-save your credentials.";
const INCONSISTENT_CLEAR_ERR: &str =
    "Langfuse credentials are in an inconsistent state and could not be fully cleared; please re-save, then clear again.";

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

/// Strict, error-propagating read for the **credential** path (TASK-044). Unlike the lenient
/// [`read_setting`] (which flattens a genuine DB error into `None` via `.ok().flatten()`, fine for
/// optional config), this maps a real read failure to `Err` and `Ok(None)` only for a genuinely
/// absent row. The resolver uses it so a broken `settings` read can never be silently downgraded to
/// the env public-key fallback (contract symmetry with the secret side's `?` short-circuit).
fn read_setting_strict(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    let value = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    Ok(value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty()))
}

/// Remove a `settings` row. Deleting an absent row is a success (idempotent), mirroring the
/// idempotent Keychain delete — used by the atomic credential set rollback and clear (TASK-044).
fn clear_setting(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
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

/// Resolve the configured Langfuse import range (TASK-029 C). Settings-first, then the marked dev
/// env fallback, then the default (`last_30d`). A stored or env value that fails to parse (an unknown
/// keyword or a malformed `since:` timestamp) resolves to the **default** rather than failing the
/// import — the import is never blocked by a bad range value.
pub fn resolve_import_range(conn: &Connection) -> ImportRange {
    resolve_import_range_with(conn, &ProcessEnv)
}

pub fn resolve_import_range_with(conn: &Connection, env: &dyn EnvSource) -> ImportRange {
    let raw =
        read_setting(conn, KEY_IMPORT_RANGE).or_else(|| env.get("VIRE_LANGFUSE_IMPORT_RANGE"));
    match raw {
        Some(value) => ImportRange::parse(&value).unwrap_or_else(|_| ImportRange::default_range()),
        None => ImportRange::default_range(),
    }
}

/// Read the import-range setting back as its canonical string for the Settings panel. Resolves the
/// same way the importer does, so a malformed stored value reads back as the default rather than an
/// invalid string.
pub fn get_langfuse_import_range_repo(conn: &Connection) -> CmdResult<String> {
    Ok(resolve_import_range(conn).as_setting())
}

/// Validate and persist the import-range setting (TASK-029 C1 / app-configuration). The value is
/// validated through [`ImportRange::parse`]; a malformed value is rejected with a **fixed,
/// secret-free** error that never echoes the input (so a malformed `since:` cannot leak content).
/// The stored value is the canonical normalized form, which is also returned for the form to
/// re-render from a single source.
pub fn set_langfuse_import_range_repo(conn: &Connection, value: String) -> CmdResult<String> {
    let range = ImportRange::parse(&value).map_err(|e| e.to_string())?;
    let canonical = range.as_setting();
    write_setting(conn, KEY_IMPORT_RANGE, &canonical).map_err(|e| e.to_string())?;
    Ok(canonical)
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

/// Full config including credentials, for an import run or a Test connection probe. The credential
/// **pair** is resolved as a unit (TASK-044 / arch-review Addendum Decision 1): the public key from
/// the `settings` table and the secret key from the Keychain. Both stored ⇒ the stored pair; neither
/// stored ⇒ the marked dev-fallback env pair (only when **both** env keys are set); exactly one store
/// ⇒ `None`. This mirrors the existing `from_env` "both keys present" contract and makes a
/// mixed-source pair impossible. The secret flows through the redacting [`Secret`]/[`Credentials`]
/// types — never a raw second path.
///
/// A genuine read failure of either store (distinct from "no entry") is propagated as a coarse,
/// secret-free `Err` — it must **not** be silently treated as an absent credential, because that
/// would let the env dev fallback override a failed store read.
pub fn resolve_config(conn: &Connection, secrets: &dyn SecretStore) -> CmdResult<ImporterConfig> {
    resolve_config_with(conn, secrets, &ProcessEnv)
}

pub fn resolve_config_with(
    conn: &Connection,
    secrets: &dyn SecretStore,
    env: &dyn EnvSource,
) -> CmdResult<ImporterConfig> {
    let mut config = resolve_public_config_with(conn, env);
    config.credentials = resolve_credentials(conn, secrets, env)?;
    Ok(config)
}

/// Resolve the credential pair across the two stores (TASK-044 / arch-review Addendum Decision 1).
/// Read **both** stores strictly — the public key from the `settings` table, the secret key from the
/// Keychain — then match the pair as a **unit**: both present ⇒ the stored pair (env never consulted);
/// both genuinely absent ⇒ the env dev pair iff **both** env keys are set, else `None`; **exactly one
/// store present ⇒ `None`** (the lone stored key is discarded and env is **not** consulted for the
/// missing side). This makes "no mixed-source pair" a structural property of the match (DEC-026), not
/// a per-field accident, and renders every half-state — including the existing-install upgrade state
/// (settings-public absent + Keychain-secret present) — inert. A real read failure of either store
/// short-circuits with a coarse, secret-free `Err` so a broken store is never mistaken for "absent"
/// and downgraded to env.
fn resolve_credentials(
    conn: &Connection,
    secrets: &dyn SecretStore,
    env: &dyn EnvSource,
) -> CmdResult<Option<Credentials>> {
    // Read both stores STRICTLY; a genuine read failure on either side surfaces as a coarse,
    // secret-free error and is NEVER downgraded to the env fallback.
    let public_stored = read_setting_strict(conn, KEY_PUBLIC_KEY)
        .map_err(|_| "could not read the Langfuse public key from local settings".to_string())?;
    let secret_stored = secrets.get(SECRET_KEY_ACCOUNT).map_err(|e| e.0)?;
    // Match the pair as a UNIT — env is a whole-pair (all-or-nothing) dev override, consulted only
    // when NEITHER store holds its key. Exactly one store ⇒ `None`: the lone key is discarded and env
    // is not consulted for the missing side, so a mixed real+env pair cannot form.
    let pair = match (public_stored, secret_stored) {
        (Some(public_key), Some(secret_key)) => Some((public_key, secret_key)),
        (None, None) => {
            let public_env = env
                .get("VIRE_LANGFUSE_PUBLIC_KEY")
                .or_else(|| env.get("LANGFUSE_PUBLIC_KEY"));
            let secret_env = env
                .get("VIRE_LANGFUSE_SECRET_KEY")
                .or_else(|| env.get("LANGFUSE_SECRET_KEY"));
            match (public_env, secret_env) {
                (Some(public_key), Some(secret_key)) => Some((public_key, secret_key)),
                _ => None,
            }
        }
        _ => None,
    };
    Ok(pair.map(|(public_key, secret_key)| Credentials {
        public_key,
        secret_key: Secret::new(secret_key),
    }))
}

// ----- IPC repo functions (secret-free) -----------------------------------------------------

/// Read the renderer-facing settings view. `has_public_key` is derived from the `settings` row
/// (no Keychain access — this is what removes the second fresh-launch prompt, TASK-044);
/// `has_secret_key` consults the Keychain for **presence** only. No secret value is ever read back
/// or returned (SEC-009); the public key stays a presence flag here to avoid frontend churn.
pub fn get_langfuse_settings_repo(
    conn: &Connection,
    secrets: &dyn SecretStore,
) -> CmdResult<LangfuseSettings> {
    let config = resolve_public_config(conn);
    let has_public_key = read_setting(conn, KEY_PUBLIC_KEY).is_some();
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

/// Store the credential pair: the **public** key in the `settings` table, the **secret** key in the
/// Keychain (TASK-044). The secret is accepted *in* and never returned. Both keys are entered
/// together (one clean credential surface); both must be non-empty.
///
/// The pair now spans two stores and is written atomically: the cheap, reliable, prompt-free store
/// (SQLite) is written **first**, then the fragile Keychain. If the Keychain secret write fails, the
/// settings public row is restored to its prior state, so a failed (re)write never leaves exactly
/// one store populated — which would let `resolve_credentials` combine an env-fallback value for the
/// missing store with a stale value from the other (a mixed-source pair, DEC-026 integrity). Writing
/// the fragile store last means the only compensation ever needed is a local SQLite rewrite/delete,
/// never a fragile Keychain call. That compensation is **reliable but not infallible**: if it too
/// fails (a catastrophic local SQLite failure), the error is **not** swallowed — the function returns
/// the distinct, secret-free `INCONSISTENT_SET_ERR` (Addendum Decision 2) rather than the raw
/// Keychain cause. Reading the public key here exposes no secret (SEC-009 guards the secret key
/// only). On success any **legacy** Keychain public-key item (pre-F2a) is best-effort deleted so it
/// is never read again and never re-triggers the second prompt.
pub fn set_langfuse_secret_repo(
    conn: &Connection,
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
    // Capture the prior settings public row first so a failed secret write can restore the exact
    // prior state (a true read failure here aborts before any write — no partial state).
    let prior_public = read_setting_strict(conn, KEY_PUBLIC_KEY).map_err(|e| e.to_string())?;
    // Write the cheap, reliable, prompt-free store (SQLite) first.
    write_setting(conn, KEY_PUBLIC_KEY, public_key).map_err(|e| e.to_string())?;
    if let Err(secret_err) = secrets.set(SECRET_KEY_ACCOUNT, secret_key) {
        // The Keychain secret write failed (its entry is untouched — still the prior value, if any).
        // Restore the settings public row to its prior state: reinstate the previous value when one
        // existed (back to the prior consistent pair), else delete the row we just wrote (back to
        // both-absent). Either way the pair is left consistent, so a failed replacement can never
        // produce a mixed-source pair. The compensation is a reliable local SQLite op, but if it ALSO
        // fails (catastrophic local SQLite failure) we surface the distinct, secret-free
        // INCONSISTENT_SET_ERR instead of masking it — the compensation error is never swallowed.
        let rollback = match &prior_public {
            Some(prior) => write_setting(conn, KEY_PUBLIC_KEY, prior),
            None => clear_setting(conn, KEY_PUBLIC_KEY),
        };
        return match rollback {
            Ok(()) => Err(secret_err.0),
            Err(_) => Err(INCONSISTENT_SET_ERR.into()),
        };
    }
    // Drop any legacy Keychain public-key item written by a pre-F2a build (§4). Best-effort and
    // idempotent — a no-op when absent, and its failure must never fail an otherwise-good save.
    let _ = secrets.delete(PUBLIC_KEY_ACCOUNT);
    Ok(())
}

/// Remove the credential pair: the `settings` public row and the Keychain secret (TASK-044 /
/// arch-review Addendum Decision 2). Idempotent. The cheap, reliable store (SQLite) is cleared
/// **first**; on its failure the function aborts **before** touching the Keychain so both stores
/// remain (the prior consistent pair). The fragile Keychain delete is performed **last** and, on its
/// failure, the public row is **restored** from the captured prior value — a deleted Keychain secret
/// cannot be restored (and must not be re-read; that re-introduces the removed prompt, against
/// SEC-009), so SQLite-first is what keeps the only recoverable artifact (the non-secret public key)
/// restorable, leaving the prior pair intact. If that restore itself fails (catastrophic local SQLite
/// failure) the distinct, secret-free `INCONSISTENT_CLEAR_ERR` is returned rather than swallowed. Any
/// legacy Keychain public-key item is then best-effort deleted.
pub fn clear_langfuse_secret_repo(conn: &Connection, secrets: &dyn SecretStore) -> CmdResult<()> {
    // Capture the prior public row before any mutation; a genuine read failure aborts before either
    // store is touched (both remain ⇒ prior pair).
    let prior_public = read_setting_strict(conn, KEY_PUBLIC_KEY).map_err(|e| e.to_string())?;
    // Clear the cheap, reliable, prompt-free store (SQLite) FIRST. On failure, abort before touching
    // the Keychain ⇒ both stores remain ⇒ prior consistent pair (safe).
    clear_setting(conn, KEY_PUBLIC_KEY).map_err(|e| e.to_string())?;
    if let Err(secret_err) = secrets.delete(SECRET_KEY_ACCOUNT) {
        // The fragile Keychain delete failed. The recoverable artifact is the non-secret public key:
        // restore it from prior_public ⇒ back to the prior pair. If that restore itself fails
        // (catastrophic local SQLite failure), surface the distinct, secret-free INCONSISTENT_CLEAR_ERR
        // — the compensation error is never swallowed.
        let restore = match &prior_public {
            Some(prior) => write_setting(conn, KEY_PUBLIC_KEY, prior),
            None => Ok(()),
        };
        return match restore {
            Ok(()) => Err(secret_err.0),
            Err(_) => Err(INCONSISTENT_CLEAR_ERR.into()),
        };
    }
    // Remove any legacy Keychain public-key item (pre-F2a); best-effort, idempotent.
    let _ = secrets.delete(PUBLIC_KEY_ACCOUNT);
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
