//! Importer configuration, credential handling, and the Rust-side URL allowlist.
//!
//! DEC-020 / SEC-002 / SEC-003: local Docker self-hosted Langfuse on loopback is the
//! default and only direct target; Cloud is an explicit non-default override and the sole
//! off-host egress path. Every request URL is built from the single configured base URL plus
//! a fixed `/api/public/*` path set — nothing else is reachable. Credentials are used only for
//! the `Authorization` header and never printed, logged, persisted, or exported.

use std::fmt;
use url::Url;

/// Default local Docker self-hosted Langfuse base URL (loopback only).
pub const DEFAULT_BASE_URL: &str = "http://127.0.0.1:3000";
/// Default per-project environment mapping (DEC-020 environment-first).
pub const DEFAULT_ENVIRONMENT: &str = "vire";

/// Trace source posture. `Local` is the canonical default; `Cloud` is an explicit,
/// non-default override and the only configuration that produces off-host egress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Local,
    Cloud,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::Local => "local",
            Source::Cloud => "cloud",
        }
    }

    pub fn parse(value: &str) -> Source {
        match value.trim().to_ascii_lowercase().as_str() {
            "cloud" => Source::Cloud,
            _ => Source::Local,
        }
    }
}

/// A secret string whose value is never revealed by `Debug`/`Display`. Exposed only via
/// `expose()` at the single point that builds the `Authorization` header.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Secret(value.into())
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***redacted***")
    }
}

/// API credentials (HTTP Basic). Both fields are kept out of logs/evidence/exports.
#[derive(Clone)]
pub struct Credentials {
    pub public_key: String,
    pub secret_key: Secret,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never print the real key material, not even the public key.
        f.debug_struct("Credentials")
            .field("public_key", &"***redacted***")
            .field("secret_key", &self.secret_key)
            .finish()
    }
}

/// Fixed set of read-only public API paths the importer is allowed to construct. There is no
/// variant that accepts an arbitrary path or absolute URL from response data.
pub enum ApiPath<'a> {
    /// Availability probe (cheap read used to detect a down/unreachable stack).
    Health,
    /// `GET /api/public/traces` with environment + time-window + pagination params.
    Traces {
        environment: &'a str,
        from: &'a str,
        to: &'a str,
        page: u32,
        limit: u32,
    },
    /// `GET /api/public/observations?traceId=` (generation usage/cost lives here).
    Observations { trace_id: &'a str },
}

#[derive(Debug, Clone)]
pub struct ImporterConfig {
    pub base_url: String,
    pub source: Source,
    pub allowed_environments: Vec<String>,
    pub credentials: Option<Credentials>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ConfigError {}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1")
}

impl ImporterConfig {
    /// Construct from explicit values (used by tests and by `from_env`). Credentials are
    /// optional so a not-yet-configured install still resolves a health state rather than
    /// crashing — a missing/invalid key surfaces as `auth_or_network_error`, never zero cost.
    pub fn new(
        base_url: impl Into<String>,
        source: Source,
        allowed_environments: Vec<String>,
        credentials: Option<Credentials>,
    ) -> Self {
        ImporterConfig {
            base_url: base_url.into(),
            source,
            allowed_environments,
            credentials,
        }
    }

    /// Load public (non-secret) configuration: base URL, source, environments. Never reads or
    /// returns credential material — safe to feed into the read-only health snapshot.
    pub fn public_from_env() -> Self {
        let base_url =
            env_nonempty("VIRE_LANGFUSE_BASE_URL").unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let source = env_nonempty("VIRE_LANGFUSE_SOURCE")
            .map(|s| Source::parse(&s))
            .unwrap_or(Source::Local);
        Self::new(base_url, source, env_environments(), None)
    }

    /// Full configuration including credentials, for an actual import run. Credentials are read
    /// from environment variables only (`VIRE_LANGFUSE_PUBLIC_KEY` / `_SECRET_KEY`, falling back
    /// to `LANGFUSE_PUBLIC_KEY` / `LANGFUSE_SECRET_KEY`). No shell sourcing; values never logged.
    pub fn from_env() -> Self {
        let mut config = Self::public_from_env();
        let public_key = env_nonempty("VIRE_LANGFUSE_PUBLIC_KEY")
            .or_else(|| env_nonempty("LANGFUSE_PUBLIC_KEY"));
        let secret_key = env_nonempty("VIRE_LANGFUSE_SECRET_KEY")
            .or_else(|| env_nonempty("LANGFUSE_SECRET_KEY"));
        if let (Some(public_key), Some(secret_key)) = (public_key, secret_key) {
            config.credentials = Some(Credentials {
                public_key,
                secret_key: Secret::new(secret_key),
            });
        }
        config
    }

    pub fn parsed_base(&self) -> Result<Url, ConfigError> {
        let url = Url::parse(&self.base_url)
            .map_err(|_| ConfigError("Langfuse base URL is not a valid URL".into()))?;
        match url.scheme() {
            "http" | "https" => {}
            other => {
                return Err(ConfigError(format!(
                    "Langfuse base URL scheme '{other}' is not allowed (http/https only)"
                )))
            }
        }
        if url.host_str().is_none() {
            return Err(ConfigError("Langfuse base URL must include a host".into()));
        }
        Ok(url)
    }

    /// Enforce the network boundary (SEC-002): when the source is `Local`, the base host must be
    /// loopback. Cloud is permitted only because the operator explicitly set it. Any other
    /// host/scheme is refused — this is the DEC-020 "Tauri HTTP URL allowlist" as a Rust invariant.
    pub fn validate_target(&self) -> Result<(), ConfigError> {
        let url = self.parsed_base()?;
        let host = url.host_str().unwrap_or("");
        match self.source {
            Source::Local => {
                if !is_loopback_host(host) {
                    return Err(ConfigError(format!(
                        "local source must target a loopback host, got '{host}'; set source=cloud to reach an off-host endpoint"
                    )));
                }
            }
            Source::Cloud => {
                if is_loopback_host(host) {
                    return Err(ConfigError(
                        "cloud source must target an off-host endpoint, not loopback".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Build a concrete request URL from the configured base plus one of the fixed `/api/public/*`
    /// paths. The result always shares the base host/scheme and its path is always rooted under
    /// `/api/public/` — no path or host from response data can ever be substituted in.
    pub fn build_url(&self, path: &ApiPath<'_>) -> Result<Url, ConfigError> {
        self.validate_target()?;
        let base = self.parsed_base()?;
        let mut url = base.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| ConfigError("Langfuse base URL cannot be a base".into()))?;
            // Drop any trailing empty segment from a base like `http://host:3000/`.
            segments.pop_if_empty();
            segments.push("api");
            segments.push("public");
            match path {
                ApiPath::Health => {
                    segments.push("health");
                }
                ApiPath::Traces { .. } => {
                    segments.push("traces");
                }
                ApiPath::Observations { .. } => {
                    segments.push("observations");
                }
            }
        }
        match path {
            ApiPath::Traces {
                environment,
                from,
                to,
                page,
                limit,
            } => {
                let mut q = url.query_pairs_mut();
                q.append_pair("environment", environment);
                q.append_pair("fromTimestamp", from);
                q.append_pair("toTimestamp", to);
                q.append_pair("page", &page.to_string());
                q.append_pair("limit", &limit.to_string());
            }
            ApiPath::Observations { trace_id } => {
                url.query_pairs_mut().append_pair("traceId", trace_id);
            }
            _ => {}
        }
        // Defence in depth: the constructed path must stay under the public API root and on the
        // same origin as the configured base.
        if !url.path().starts_with("/api/public/") {
            return Err(ConfigError(
                "refusing to build a URL outside /api/public/".into(),
            ));
        }
        if url.host_str() != base.host_str() || url.scheme() != base.scheme() {
            return Err(ConfigError(
                "refusing to build a URL that leaves the configured host/scheme".into(),
            ));
        }
        Ok(url)
    }
}

fn env_nonempty(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_environments() -> Vec<String> {
    match env_nonempty("VIRE_LANGFUSE_ENVIRONMENTS") {
        Some(csv) => {
            let envs: Vec<String> = csv
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if envs.is_empty() {
                vec![DEFAULT_ENVIRONMENT.to_string()]
            } else {
                envs
            }
        }
        None => vec![DEFAULT_ENVIRONMENT.to_string()],
    }
}
