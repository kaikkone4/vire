//! Settings resolver + IPC repo tests (TASK-026 / A5 backend). No real Keychain, no network: the
//! secret store is the in-memory fake and env is an injected map, so precedence is deterministic and
//! no test races on process-global env vars.

use std::collections::HashMap;

use rusqlite::Connection;

use super::secret_store::{MemorySecretStore, SecretStoreError};
use super::*;
use crate::langfuse::config::Source;

const SECRET: &str = "sk-lf-supersecret-canary";
const PUBLIC: &str = "pk-lf-public-canary";

/// An injected, race-free process-env stand-in.
struct MapEnv(HashMap<String, String>);

impl MapEnv {
    fn new() -> Self {
        MapEnv(HashMap::new())
    }
    fn with(mut self, key: &str, value: &str) -> Self {
        self.0.insert(key.to_string(), value.to_string());
        self
    }
}

impl EnvSource for MapEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0
            .get(key)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }
}

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    crate::init_db(&c).unwrap();
    c
}

fn store_settings(
    c: &Connection,
    secrets: &dyn SecretStore,
    base_url: &str,
    source: &str,
    envs: &[&str],
    enabled: bool,
) {
    set_langfuse_settings_repo(
        c,
        secrets,
        LangfuseSettingsInput {
            base_url: base_url.to_string(),
            source: source.to_string(),
            environments: envs.iter().map(|s| s.to_string()).collect(),
            langfuse_enabled: enabled,
        },
    )
    .unwrap();
}

// ----- resolver precedence: settings > env > code defaults --------------------------------

#[test]
fn resolver_uses_code_defaults_when_no_settings_or_env() {
    let c = conn();
    let cfg = resolve_public_config_with(&c, &MapEnv::new());
    assert_eq!(cfg.base_url, "http://127.0.0.1:3000");
    assert_eq!(cfg.source, Source::Local);
    assert_eq!(cfg.allowed_environments, vec!["vire".to_string()]);
    assert!(cfg.validate_target().is_ok());
}

#[test]
fn resolver_falls_back_to_env_when_no_settings() {
    let c = conn();
    let env = MapEnv::new()
        .with("VIRE_LANGFUSE_BASE_URL", "http://localhost:3000")
        .with("VIRE_LANGFUSE_SOURCE", "local")
        .with("VIRE_LANGFUSE_ENVIRONMENTS", "alpha, beta");
    let cfg = resolve_public_config_with(&c, &env);
    assert_eq!(cfg.base_url, "http://localhost:3000");
    assert_eq!(
        cfg.allowed_environments,
        vec!["alpha".to_string(), "beta".to_string()]
    );
}

#[test]
fn stored_settings_win_over_env() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    store_settings(
        &c,
        &secrets,
        "http://127.0.0.1:4000",
        "local",
        &["stored"],
        true,
    );
    let env = MapEnv::new()
        .with("VIRE_LANGFUSE_BASE_URL", "http://127.0.0.1:9999")
        .with("VIRE_LANGFUSE_ENVIRONMENTS", "envonly");
    let cfg = resolve_public_config_with(&c, &env);
    assert_eq!(
        cfg.base_url, "http://127.0.0.1:4000",
        "stored base_url wins over env"
    );
    assert_eq!(cfg.allowed_environments, vec!["stored".to_string()]);
}

#[test]
fn credentials_resolve_keychain_first_then_env_dev_fallback() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    let env = MapEnv::new()
        .with("VIRE_LANGFUSE_PUBLIC_KEY", "pk-env")
        .with("VIRE_LANGFUSE_SECRET_KEY", "sk-env-secret");
    // No Keychain creds → env dev fallback supplies them.
    let from_env = resolve_config_with(&c, &secrets, &env)
        .unwrap()
        .credentials
        .expect("env credentials present");
    assert_eq!(from_env.public_key, "pk-env");
    assert_eq!(from_env.secret_key.expose(), "sk-env-secret");
    // Store the pair (public→settings, secret→Keychain) → the stored pair wins over env.
    set_langfuse_secret_repo(&c, &secrets, "pk-stored".into(), "sk-stored-secret".into()).unwrap();
    let from_store = resolve_config_with(&c, &secrets, &env)
        .unwrap()
        .credentials
        .expect("stored credentials present");
    assert_eq!(from_store.public_key, "pk-stored");
    assert_eq!(from_store.secret_key.expose(), "sk-stored-secret");
}

#[test]
fn credentials_absent_when_neither_keychain_nor_env() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    let cfg = resolve_config_with(&c, &secrets, &MapEnv::new()).unwrap();
    assert!(
        cfg.credentials.is_none(),
        "no creds → None, never fabricated as empty"
    );
}

// ----- SEC-002 loopback boundary holds for settings-sourced values ------------------------

#[test]
fn loopback_boundary_holds_for_settings_sourced_local() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    // `set` accepts a well-formed URL; `validate_target` enforces loopback at use time.
    store_settings(
        &c,
        &secrets,
        "http://example.com:3000",
        "local",
        &["vire"],
        true,
    );
    let cfg = resolve_public_config_with(&c, &MapEnv::new());
    assert!(
        cfg.validate_target().is_err(),
        "local + non-loopback must be refused, identical to an env-sourced value"
    );
}

#[test]
fn cloud_off_host_allowed_only_on_explicit_source() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    store_settings(
        &c,
        &secrets,
        "https://cloud.langfuse.com",
        "cloud",
        &["vire"],
        true,
    );
    let cfg = resolve_public_config_with(&c, &MapEnv::new());
    assert_eq!(cfg.source, Source::Cloud);
    assert!(
        cfg.validate_target().is_ok(),
        "cloud + off-host is permitted on explicit source"
    );
}

#[test]
fn set_rejects_a_malformed_base_url_with_a_secret_free_error() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    let err = set_langfuse_settings_repo(
        &c,
        &secrets,
        LangfuseSettingsInput {
            base_url: "not a url".into(),
            source: "local".into(),
            environments: vec!["vire".into()],
            langfuse_enabled: true,
        },
    )
    .unwrap_err();
    assert!(!err.is_empty());
    assert!(!err.contains("sk-") && !err.contains("pk-"));
}

// ----- SEC-009 secret non-leak ------------------------------------------------------------

#[test]
fn get_settings_returns_presence_flags_never_secret_values() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    set_langfuse_secret_repo(&c, &secrets, PUBLIC.into(), SECRET.into()).unwrap();
    let view = get_langfuse_settings_repo(&c, &secrets).unwrap();
    assert!(view.has_public_key);
    assert!(view.has_secret_key);
    let json = serde_json::to_string(&view).unwrap();
    for needle in [SECRET, PUBLIC, "supersecret", "canary"] {
        assert!(!json.contains(needle), "settings view leaked {needle}");
    }
}

#[test]
fn presence_flags_false_when_keychain_empty() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    let view = get_langfuse_settings_repo(&c, &secrets).unwrap();
    assert!(!view.has_public_key);
    assert!(!view.has_secret_key);
}

#[test]
fn clearing_secret_removes_it_and_flips_presence() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    set_langfuse_secret_repo(&c, &secrets, PUBLIC.into(), SECRET.into()).unwrap();
    assert!(
        get_langfuse_settings_repo(&c, &secrets)
            .unwrap()
            .has_secret_key
    );
    clear_langfuse_secret_repo(&c, &secrets).unwrap();
    let view = get_langfuse_settings_repo(&c, &secrets).unwrap();
    assert!(!view.has_secret_key);
    assert!(!view.has_public_key);
    // With no env fallback, a cleared secret resolves to no credentials → auth_or_network_error at
    // import time, never zero. (The credential is simply absent.)
    assert!(resolve_config_with(&c, &secrets, &MapEnv::new())
        .unwrap()
        .credentials
        .is_none());
    // Clearing again is idempotent.
    clear_langfuse_secret_repo(&c, &secrets).unwrap();
}

#[test]
fn secret_is_never_written_to_the_settings_table() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    store_settings(
        &c,
        &secrets,
        "http://127.0.0.1:3000",
        "local",
        &["vire"],
        true,
    );
    set_langfuse_secret_repo(&c, &secrets, PUBLIC.into(), SECRET.into()).unwrap();
    let mut stmt = c.prepare("SELECT key, value FROM settings").unwrap();
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for (key, value) in &rows {
        // SEC-009 / C2: the SECRET key value never enters the plaintext settings table, and no
        // settings key name is credential-bearing. The PUBLIC key is non-secret and, as of TASK-044,
        // is intentionally stored here — so it is NOT asserted absent.
        assert!(
            !value.contains(SECRET),
            "settings[{key}] leaked the secret key"
        );
        assert!(
            !key.to_ascii_lowercase().contains("secret"),
            "no credential-bearing settings key"
        );
    }
    // The secret never reaches settings under any key; the public key does (TASK-044 relocation).
    assert!(
        !rows.iter().any(|(_, value)| value.contains(SECRET)),
        "the secret key must never appear as a settings value"
    );
    assert_eq!(
        read_setting_strict(&c, KEY_PUBLIC_KEY).unwrap().as_deref(),
        Some(PUBLIC),
        "the public key is stored in settings after TASK-044"
    );
}

#[test]
fn settings_sourced_credentials_stay_redacted_in_debug() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    set_langfuse_secret_repo(&c, &secrets, PUBLIC.into(), SECRET.into()).unwrap();
    let cfg = resolve_config_with(&c, &secrets, &MapEnv::new()).unwrap();
    let rendered = format!("{cfg:?}");
    for needle in [SECRET, PUBLIC, "supersecret", "canary"] {
        assert!(
            !rendered.contains(needle),
            "Debug(ImporterConfig) leaked {needle}"
        );
    }
}

// ----- langfuse_enabled gating: disabled short-circuits before network/Keychain -----------

#[test]
fn enabled_is_true_by_default_and_snapshot_is_unknown_before_import() {
    let c = conn();
    assert!(
        langfuse_enabled(&c),
        "default-on preserves the pre-TASK-026 always-active behavior"
    );
    let snap = source_health_snapshot(&c).unwrap();
    assert_eq!(snap.health, "unknown");
}

#[test]
fn disabled_short_circuits_to_a_disabled_snapshot_with_no_secret_store_access() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    set_langfuse_secret_repo(&c, &secrets, PUBLIC.into(), SECRET.into()).unwrap();
    store_settings(
        &c,
        &secrets,
        "http://127.0.0.1:3000",
        "local",
        &["vire"],
        false,
    );
    assert!(!langfuse_enabled(&c));
    // `source_health_snapshot` takes no SecretStore and fires no probe — disabled is resolved
    // structurally before any Keychain or network access.
    let snap = source_health_snapshot(&c).unwrap();
    assert_eq!(snap.health, "disabled");
    assert!(
        snap.message.contains("not zero AI usage or cost"),
        "disabled is never reported as zero"
    );
}

#[test]
fn enabled_env_fallback_can_disable_but_stored_setting_wins() {
    let c = conn();
    let env = MapEnv::new().with("VIRE_LANGFUSE_ENABLED", "false");
    assert!(
        !langfuse_enabled_with(&c, &env),
        "env dev fallback can disable when unset in settings"
    );
    write_setting(&c, KEY_ENABLED, "true").unwrap();
    assert!(
        langfuse_enabled_with(&c, &env),
        "stored setting wins over env"
    );
}

#[test]
fn round_trip_set_then_get_reflects_non_secret_settings() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    let view = set_langfuse_settings_repo(
        &c,
        &secrets,
        LangfuseSettingsInput {
            base_url: "http://127.0.0.1:3000".into(),
            source: "cloud".into(),
            environments: vec![" prod ".into(), "".into(), "staging".into()],
            langfuse_enabled: true,
        },
    )
    .unwrap();
    // Source normalized, blank environments dropped, trimmed.
    assert_eq!(view.source, "cloud");
    assert_eq!(
        view.environments,
        vec!["prod".to_string(), "staging".to_string()]
    );
    assert!(view.langfuse_enabled);
    assert_eq!(view, get_langfuse_settings_repo(&c, &secrets).unwrap());
}

// ----- Blocker 1: disabled Test connection short-circuits before any secret-store access ----

/// A secret store that fails loudly if *any* method runs — used to prove a code path never touches
/// the secret store (the disabled Test-connection short-circuit must not read the Keychain).
struct TripwireSecretStore;

impl SecretStore for TripwireSecretStore {
    fn get(&self, _account: &str) -> Result<Option<String>, SecretStoreError> {
        panic!("secret store get must not be called on the disabled path");
    }
    fn set(&self, _account: &str, _value: &str) -> Result<(), SecretStoreError> {
        panic!("secret store set must not be called on the disabled path");
    }
    fn delete(&self, _account: &str) -> Result<(), SecretStoreError> {
        panic!("secret store delete must not be called on the disabled path");
    }
}

#[test]
fn disabled_test_connection_plan_short_circuits_without_touching_the_secret_store() {
    let c = conn();
    // Persist the integration as disabled (the credential, if any, is irrelevant to this path).
    store_settings(
        &c,
        &MemorySecretStore::default(),
        "http://127.0.0.1:3000",
        "local",
        &["vire"],
        false,
    );
    assert!(!langfuse_enabled(&c));
    // The tripwire panics on any secret-store method; reaching `Disabled` without a panic proves
    // the disabled path performs no Keychain read (and, by construction, no network probe).
    let plan = test_connection_plan(&c, &TripwireSecretStore).unwrap();
    assert!(matches!(plan, TestConnectionPlan::Disabled));
}

#[test]
fn enabled_test_connection_plan_resolves_config_for_a_probe() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    store_settings(
        &c,
        &secrets,
        "http://127.0.0.1:3000",
        "local",
        &["vire"],
        true,
    );
    set_langfuse_secret_repo(&c, &secrets, PUBLIC.into(), SECRET.into()).unwrap();
    match test_connection_plan(&c, &secrets).unwrap() {
        TestConnectionPlan::Probe(config) => {
            assert_eq!(config.base_url, "http://127.0.0.1:3000");
            assert!(
                config.credentials.is_some(),
                "enabled probe carries resolved credentials"
            );
        }
        TestConnectionPlan::Disabled => panic!("enabled integration must produce a probe plan"),
    }
}

// ----- Blocker 2: a Keychain read failure is distinguished from an absent credential -------

/// A secret store whose reads fail with a coarse backend error (a Keychain access failure). Proves
/// a read failure is not silently flattened to "no entry" and downgraded to the env dev fallback.
struct FailingSecretStore;

impl SecretStore for FailingSecretStore {
    fn get(&self, _account: &str) -> Result<Option<String>, SecretStoreError> {
        Err(SecretStoreError(
            "could not read the credential from the system keychain".into(),
        ))
    }
    fn set(&self, _account: &str, _value: &str) -> Result<(), SecretStoreError> {
        Err(SecretStoreError(
            "could not store the credential in the system keychain".into(),
        ))
    }
    fn delete(&self, _account: &str) -> Result<(), SecretStoreError> {
        Err(SecretStoreError(
            "could not remove the credential from the system keychain".into(),
        ))
    }
}

#[test]
fn keychain_read_failure_is_propagated_not_masked_as_missing_credentials() {
    let c = conn();
    // Env creds ARE present: the old `.ok().flatten()` would have wrongly fallen back to them.
    let env = MapEnv::new()
        .with("VIRE_LANGFUSE_PUBLIC_KEY", "pk-env")
        .with("VIRE_LANGFUSE_SECRET_KEY", "sk-env-secret");
    let err = resolve_config_with(&c, &FailingSecretStore, &env).unwrap_err();
    // A real Keychain failure surfaces as a coarse, secret-free error — never an env-fallback config.
    assert!(!err.is_empty());
    for needle in ["sk-", "pk-", "sk-env-secret", "pk-env", "canary"] {
        assert!(
            !err.contains(needle),
            "keychain failure error must be secret-free, found {needle}"
        );
    }
}

#[test]
fn keychain_read_failure_blocks_the_test_connection_plan_before_a_probe() {
    let c = conn();
    // Enabled, so the plan reaches the credential read — which fails coarsely instead of probing
    // with no/partial credentials.
    store_settings(
        &c,
        &MemorySecretStore::default(),
        "http://127.0.0.1:3000",
        "local",
        &["vire"],
        true,
    );
    let err = test_connection_plan(&c, &FailingSecretStore).unwrap_err();
    assert!(!err.is_empty());
    assert!(!err.contains("sk-") && !err.contains("pk-"));
}

// ----- TASK-044: two-store atomic set/clear + resolver (C1, C4) -----------------------------

/// A Keychain whose secret write fails — exercises the atomic-set rollback (TASK-044): the settings
/// public row is written first, then the Keychain secret; a failed secret write must roll the
/// settings public row back so no one-store pair remains.
#[derive(Default)]
struct SecretWriteFailsStore {
    inner: std::sync::Mutex<HashMap<String, String>>,
}

impl SecretStore for SecretWriteFailsStore {
    fn get(&self, account: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(self.inner.lock().unwrap().get(account).cloned())
    }
    fn set(&self, account: &str, value: &str) -> Result<(), SecretStoreError> {
        if account == SECRET_KEY_ACCOUNT {
            return Err(SecretStoreError(
                "could not store the credential in the system keychain".into(),
            ));
        }
        self.inner
            .lock()
            .unwrap()
            .insert(account.to_string(), value.to_string());
        Ok(())
    }
    fn delete(&self, account: &str) -> Result<(), SecretStoreError> {
        self.inner.lock().unwrap().remove(account);
        Ok(())
    }
}

/// T1 — happy path: set writes the public key to `settings` and the secret to the Keychain (no
/// public Keychain item), and the resolver returns the pair. A legacy Keychain public item is
/// best-effort deleted on set.
#[test]
fn set_writes_public_to_settings_secret_to_keychain_and_resolves_the_pair() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    // Simulate a pre-F2a install that still holds a legacy public-key Keychain item.
    secrets
        .set(PUBLIC_KEY_ACCOUNT, "pk-legacy-keychain")
        .unwrap();

    set_langfuse_secret_repo(&c, &secrets, PUBLIC.into(), SECRET.into()).unwrap();

    // Public key landed in settings; no public-key Keychain item exists (the legacy one is cleaned).
    assert_eq!(
        read_setting_strict(&c, KEY_PUBLIC_KEY).unwrap().as_deref(),
        Some(PUBLIC)
    );
    assert!(
        secrets.get(PUBLIC_KEY_ACCOUNT).unwrap().is_none(),
        "no public-key Keychain item remains after set (legacy item best-effort deleted)"
    );
    assert_eq!(
        secrets.get(SECRET_KEY_ACCOUNT).unwrap().as_deref(),
        Some(SECRET)
    );

    let creds = resolve_config_with(&c, &secrets, &MapEnv::new())
        .unwrap()
        .credentials
        .expect("stored pair resolves");
    assert_eq!(creds.public_key, PUBLIC);
    assert_eq!(creds.secret_key.expose(), SECRET);
}

/// T2 — atomic set rollback (C1): no prior pair; the settings public write succeeds, the Keychain
/// secret write fails, so the settings public row is rolled back (deleted) — never a one-store pair.
#[test]
fn secret_write_failure_rolls_back_the_public_key_write() {
    let c = conn();
    let store = SecretWriteFailsStore::default();
    let err = set_langfuse_secret_repo(&c, &store, PUBLIC.into(), SECRET.into()).unwrap_err();
    assert!(!err.is_empty());
    assert!(
        !err.contains(SECRET) && !err.contains(PUBLIC),
        "rollback error must be secret-free"
    );
    // No misleading partial state: the settings public row is rolled back (deleted, since there was
    // no prior value) and the Keychain secret was never written — neither store is populated.
    assert!(
        read_setting_strict(&c, KEY_PUBLIC_KEY).unwrap().is_none(),
        "settings public key write must be rolled back when the secret write fails"
    );
    assert!(store.get(SECRET_KEY_ACCOUNT).unwrap().is_none());
}

/// T2/T3 regression (C1): a failed **replacement** of an existing pair must restore the prior pair
/// (prior settings public + prior Keychain secret), never leaving the new settings public beside the
/// stale Keychain secret. The resolver then returns the prior pair and — even with an env public-key
/// fallback set — never combines the env public key with the surviving Keychain secret.
#[test]
fn failed_replacement_restores_the_prior_pair_and_never_mixes_stores_with_env() {
    const P_OLD: &str = "pk-old-stored";
    const S_OLD: &str = "sk-old-keychain-secret";
    const P_NEW: &str = "pk-new-stored";
    const S_NEW: &str = "sk-new-keychain-secret";

    let c = conn();
    // Seed the prior pair: public in settings, secret in the Keychain (the store's `set` refuses the
    // secret account, so the prior secret is injected directly into its backing map).
    write_setting(&c, KEY_PUBLIC_KEY, P_OLD).unwrap();
    let store = SecretWriteFailsStore::default();
    store
        .inner
        .lock()
        .unwrap()
        .insert(SECRET_KEY_ACCOUNT.to_string(), S_OLD.to_string());

    // Attempt to replace the pair; the secret-key write fails mid-replacement.
    let err = set_langfuse_secret_repo(&c, &store, P_NEW.into(), S_NEW.into()).unwrap_err();
    assert!(!err.is_empty());
    for needle in [S_OLD, S_NEW, P_OLD, P_NEW] {
        assert!(
            !err.contains(needle),
            "rollback error must be secret-free, found {needle}"
        );
    }

    // The pair is restored to the prior, consistent state — settings public back to P_OLD, Keychain
    // secret still S_OLD (NOT the new public beside the stale secret, NOT a deleted public row).
    assert_eq!(
        read_setting_strict(&c, KEY_PUBLIC_KEY).unwrap().as_deref(),
        Some(P_OLD)
    );
    assert_eq!(
        store.get(SECRET_KEY_ACCOUNT).unwrap().as_deref(),
        Some(S_OLD)
    );

    // The decisive check: even with an env public-key fallback available, the resolver returns the
    // prior pair. It can NOT combine the env public key with the surviving Keychain secret.
    let env = MapEnv::new()
        .with("VIRE_LANGFUSE_PUBLIC_KEY", "pk-env-must-not-be-used")
        .with("LANGFUSE_PUBLIC_KEY", "pk-env-must-not-be-used");
    let creds = resolve_config_with(&c, &store, &env)
        .unwrap()
        .credentials
        .expect("the restored prior pair resolves to credentials");
    assert_eq!(
        creds.public_key, P_OLD,
        "public key must come from settings, not the env fallback"
    );
    assert_eq!(
        creds.secret_key.expose(),
        S_OLD,
        "secret key must remain the prior Keychain entry"
    );
    assert_ne!(
        creds.public_key, "pk-env-must-not-be-used",
        "no mixed pair: the env public key must never pair with the Keychain secret"
    );
}

/// T3 (C1 resolver): a genuine `settings` read failure on the credential public-key path must
/// short-circuit to a coarse, secret-free `Err` — never be downgraded to the env public-key fallback
/// (contract symmetry with the secret side). A connection without the `settings` table makes the
/// strict read fail with a real DB error (distinct from an absent row).
#[test]
fn settings_read_failure_short_circuits_and_never_downgrades_to_env_public() {
    // No `init_db` ⇒ the `settings` table does not exist ⇒ the strict read errors (a true DB error,
    // not "absent row"). The lenient public-config reads swallow it to defaults; the credential read
    // must NOT.
    let c = Connection::open_in_memory().unwrap();
    let secrets = MemorySecretStore::default();
    secrets.set(SECRET_KEY_ACCOUNT, SECRET).unwrap();
    // Env supplies both keys — the old lenient read would have wrongly produced an env public key.
    let env = MapEnv::new()
        .with("VIRE_LANGFUSE_PUBLIC_KEY", "pk-env-must-not-be-used")
        .with("VIRE_LANGFUSE_SECRET_KEY", "sk-env-must-not-be-used");
    let err = resolve_config_with(&c, &secrets, &env).unwrap_err();
    assert!(!err.is_empty());
    for needle in ["pk-", "sk-", "pk-env-must-not-be-used", "canary"] {
        assert!(
            !err.contains(needle),
            "settings read failure must be secret-free, found {needle}"
        );
    }
}

/// T4 (C4): clear removes both stores and best-effort deletes a legacy Keychain public item.
#[test]
fn clear_removes_both_stores_and_deletes_legacy_keychain_public() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    set_langfuse_secret_repo(&c, &secrets, PUBLIC.into(), SECRET.into()).unwrap();
    // Simulate a leftover legacy public-key Keychain item alongside the new storage.
    secrets
        .set(PUBLIC_KEY_ACCOUNT, "pk-legacy-keychain")
        .unwrap();

    clear_langfuse_secret_repo(&c, &secrets).unwrap();

    assert!(
        read_setting_strict(&c, KEY_PUBLIC_KEY).unwrap().is_none(),
        "settings public row removed on clear"
    );
    assert!(
        secrets.get(SECRET_KEY_ACCOUNT).unwrap().is_none(),
        "Keychain secret removed on clear"
    );
    assert!(
        secrets.get(PUBLIC_KEY_ACCOUNT).unwrap().is_none(),
        "legacy Keychain public item best-effort deleted on clear"
    );
}

/// T4 (C4): when the Keychain secret delete fails, clear aborts **before** touching settings, so the
/// prior consistent pair is preserved (no one-store hazard).
#[test]
fn clear_aborts_before_settings_when_keychain_delete_fails() {
    let c = conn();
    write_setting(&c, KEY_PUBLIC_KEY, PUBLIC).unwrap();
    // `FailingSecretStore::delete` errors, standing in for a Keychain delete failure.
    let err = clear_langfuse_secret_repo(&c, &FailingSecretStore).unwrap_err();
    assert!(!err.is_empty());
    assert!(!err.contains("sk-") && !err.contains("pk-"));
    assert_eq!(
        read_setting_strict(&c, KEY_PUBLIC_KEY).unwrap().as_deref(),
        Some(PUBLIC),
        "settings public row untouched when the Keychain secret delete fails"
    );
}
