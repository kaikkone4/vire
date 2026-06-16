//! Settings resolver + IPC repo tests (TASK-026 / A5 backend). No real Keychain, no network: the
//! secret store is the in-memory fake and env is an injected map, so precedence is deterministic and
//! no test races on process-global env vars.

use std::collections::HashMap;

use rusqlite::Connection;

use super::secret_store::MemorySecretStore;
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

fn store_settings(c: &Connection, secrets: &dyn SecretStore, base_url: &str, source: &str, envs: &[&str], enabled: bool) {
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
    assert_eq!(cfg.allowed_environments, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn stored_settings_win_over_env() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    store_settings(&c, &secrets, "http://127.0.0.1:4000", "local", &["stored"], true);
    let env = MapEnv::new()
        .with("VIRE_LANGFUSE_BASE_URL", "http://127.0.0.1:9999")
        .with("VIRE_LANGFUSE_ENVIRONMENTS", "envonly");
    let cfg = resolve_public_config_with(&c, &env);
    assert_eq!(cfg.base_url, "http://127.0.0.1:4000", "stored base_url wins over env");
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
        .credentials
        .expect("env credentials present");
    assert_eq!(from_env.public_key, "pk-env");
    assert_eq!(from_env.secret_key.expose(), "sk-env-secret");
    // Store in the Keychain → Keychain wins over env.
    set_langfuse_secret_repo(&secrets, "pk-keychain".into(), "sk-keychain-secret".into()).unwrap();
    let from_keychain = resolve_config_with(&c, &secrets, &env)
        .credentials
        .expect("keychain credentials present");
    assert_eq!(from_keychain.public_key, "pk-keychain");
    assert_eq!(from_keychain.secret_key.expose(), "sk-keychain-secret");
}

#[test]
fn credentials_absent_when_neither_keychain_nor_env() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    let cfg = resolve_config_with(&c, &secrets, &MapEnv::new());
    assert!(cfg.credentials.is_none(), "no creds → None, never fabricated as empty");
}

// ----- SEC-002 loopback boundary holds for settings-sourced values ------------------------

#[test]
fn loopback_boundary_holds_for_settings_sourced_local() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    // `set` accepts a well-formed URL; `validate_target` enforces loopback at use time.
    store_settings(&c, &secrets, "http://example.com:3000", "local", &["vire"], true);
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
    store_settings(&c, &secrets, "https://cloud.langfuse.com", "cloud", &["vire"], true);
    let cfg = resolve_public_config_with(&c, &MapEnv::new());
    assert_eq!(cfg.source, Source::Cloud);
    assert!(cfg.validate_target().is_ok(), "cloud + off-host is permitted on explicit source");
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
    set_langfuse_secret_repo(&secrets, PUBLIC.into(), SECRET.into()).unwrap();
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
    set_langfuse_secret_repo(&secrets, PUBLIC.into(), SECRET.into()).unwrap();
    assert!(get_langfuse_settings_repo(&c, &secrets).unwrap().has_secret_key);
    clear_langfuse_secret_repo(&secrets).unwrap();
    let view = get_langfuse_settings_repo(&c, &secrets).unwrap();
    assert!(!view.has_secret_key);
    assert!(!view.has_public_key);
    // With no env fallback, a cleared secret resolves to no credentials → auth_or_network_error at
    // import time, never zero. (The credential is simply absent.)
    assert!(resolve_config_with(&c, &secrets, &MapEnv::new()).credentials.is_none());
    // Clearing again is idempotent.
    clear_langfuse_secret_repo(&secrets).unwrap();
}

#[test]
fn secret_is_never_written_to_the_settings_table() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    store_settings(&c, &secrets, "http://127.0.0.1:3000", "local", &["vire"], true);
    set_langfuse_secret_repo(&secrets, PUBLIC.into(), SECRET.into()).unwrap();
    let mut stmt = c.prepare("SELECT key, value FROM settings").unwrap();
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for (key, value) in &rows {
        assert!(!value.contains(SECRET), "settings[{key}] leaked the secret key");
        assert!(!value.contains(PUBLIC), "settings[{key}] leaked the public key");
        assert!(!key.to_ascii_lowercase().contains("secret"), "no credential-bearing settings key");
    }
}

#[test]
fn settings_sourced_credentials_stay_redacted_in_debug() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    set_langfuse_secret_repo(&secrets, PUBLIC.into(), SECRET.into()).unwrap();
    let cfg = resolve_config_with(&c, &secrets, &MapEnv::new());
    let rendered = format!("{cfg:?}");
    for needle in [SECRET, PUBLIC, "supersecret", "canary"] {
        assert!(!rendered.contains(needle), "Debug(ImporterConfig) leaked {needle}");
    }
}

// ----- langfuse_enabled gating: disabled short-circuits before network/Keychain -----------

#[test]
fn enabled_is_true_by_default_and_snapshot_is_unknown_before_import() {
    let c = conn();
    assert!(langfuse_enabled(&c), "default-on preserves the pre-TASK-026 always-active behavior");
    let snap = source_health_snapshot(&c).unwrap();
    assert_eq!(snap.health, "unknown");
}

#[test]
fn disabled_short_circuits_to_a_disabled_snapshot_with_no_secret_store_access() {
    let c = conn();
    let secrets = MemorySecretStore::default();
    set_langfuse_secret_repo(&secrets, PUBLIC.into(), SECRET.into()).unwrap();
    store_settings(&c, &secrets, "http://127.0.0.1:3000", "local", &["vire"], false);
    assert!(!langfuse_enabled(&c));
    // `source_health_snapshot` takes no SecretStore and fires no probe — disabled is resolved
    // structurally before any Keychain or network access.
    let snap = source_health_snapshot(&c).unwrap();
    assert_eq!(snap.health, "disabled");
    assert!(snap.message.contains("not zero AI usage or cost"), "disabled is never reported as zero");
}

#[test]
fn enabled_env_fallback_can_disable_but_stored_setting_wins() {
    let c = conn();
    let env = MapEnv::new().with("VIRE_LANGFUSE_ENABLED", "false");
    assert!(!langfuse_enabled_with(&c, &env), "env dev fallback can disable when unset in settings");
    write_setting(&c, KEY_ENABLED, "true").unwrap();
    assert!(langfuse_enabled_with(&c, &env), "stored setting wins over env");
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
    assert_eq!(view.environments, vec!["prod".to_string(), "staging".to_string()]);
    assert!(view.langfuse_enabled);
    assert_eq!(view, get_langfuse_settings_repo(&c, &secrets).unwrap());
}
