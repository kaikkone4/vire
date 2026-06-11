//! Domain types: the 10-state health taxonomy, Langfuse API DTOs (read by observed shape),
//! and the normalized AI-evidence row the importer persists.

use serde::{Deserialize, Serialize};

/// BA `04_technical_plan.md` §7 health taxonomy (authoritative, 10 states). `unavailable` and
/// `unknown` are the DEC-020 additions the feeder's 9-state model lacked. Every "no data" path
/// resolves to one of these states — never a numeric zero folded into an AI total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Missing,
    Stale,
    WrongEnv,
    Delayed,
    Duplicate,
    SchemaChanged,
    AuthOrNetworkError,
    Unavailable,
    Unknown,
}

impl HealthState {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthState::Healthy => "healthy",
            HealthState::Missing => "missing",
            HealthState::Stale => "stale",
            HealthState::WrongEnv => "wrong_env",
            HealthState::Delayed => "delayed",
            HealthState::Duplicate => "duplicate",
            HealthState::SchemaChanged => "schema_changed",
            HealthState::AuthOrNetworkError => "auth_or_network_error",
            HealthState::Unavailable => "unavailable",
            HealthState::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> HealthState {
        match value {
            "healthy" => HealthState::Healthy,
            "missing" => HealthState::Missing,
            "stale" => HealthState::Stale,
            "wrong_env" => HealthState::WrongEnv,
            "delayed" => HealthState::Delayed,
            "duplicate" => HealthState::Duplicate,
            "schema_changed" => HealthState::SchemaChanged,
            "auth_or_network_error" => HealthState::AuthOrNetworkError,
            "unavailable" => HealthState::Unavailable,
            _ => HealthState::Unknown,
        }
    }
}

/// Classification of a transport/API failure. Carries no secret material in `message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorKind {
    /// 401/403 — credentials missing or rejected.
    Auth,
    /// 429 — rate limited (folded into `auth_or_network_error` per the taxonomy).
    RateLimited,
    /// Timeout / TLS / transport error reaching a configured, resolvable host.
    Network,
    /// Connection refused / host unreachable — the Docker/Langfuse stack is down.
    Unavailable,
    /// Anything that leaves currency/completeness indeterminate (e.g. malformed envelope).
    Indeterminate,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub kind: ApiErrorKind,
    /// Human-readable, secret-free description.
    pub message: String,
}

impl ApiError {
    pub fn new(kind: ApiErrorKind, message: impl Into<String>) -> Self {
        ApiError {
            kind,
            message: message.into(),
        }
    }

    /// The health state a probe/list failure maps to.
    pub fn health(&self) -> HealthState {
        match self.kind {
            ApiErrorKind::Unavailable => HealthState::Unavailable,
            ApiErrorKind::Auth | ApiErrorKind::RateLimited | ApiErrorKind::Network => {
                HealthState::AuthOrNetworkError
            }
            ApiErrorKind::Indeterminate => HealthState::Unknown,
        }
    }
}

/// A page of traces from `GET /api/public/traces`.
#[derive(Debug, Clone, Deserialize)]
pub struct TracePage {
    #[serde(default)]
    pub data: Vec<serde_json::Value>,
    #[serde(default)]
    pub meta: PageMeta,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PageMeta {
    // `page`/`limit`/`total_items` are part of the API envelope; only `total_pages` drives loop
    // termination, the rest are retained for shape fidelity.
    #[allow(dead_code)]
    #[serde(default)]
    pub page: u32,
    #[allow(dead_code)]
    #[serde(default)]
    pub limit: u32,
    #[allow(dead_code)]
    #[serde(default, rename = "totalItems")]
    pub total_items: u64,
    #[serde(default, rename = "totalPages")]
    pub total_pages: u32,
}

/// Typed view over a trace. Tolerates sparse traces: nullable `sessionId`, possibly-empty
/// `name`, and variable `metadata`. Usage/cost are intentionally NOT read from the trace body.
/// `environment`/`name`/`session_id`/`metadata` are retained for shape fidelity and future
/// mapping/review use; the full payload is also persisted raw.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Trace {
    pub id: String,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "sessionId")]
    pub session_id: Option<String>,
    /// Aggregate convenience only; authoritative per-call cost lives on observations.
    #[serde(default, rename = "totalCost")]
    pub total_cost: Option<f64>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub observations: Vec<Observation>,
}

/// A generation observation — where token usage and model cost actually live. Read by observed
/// shape: `usage` = `{input, output, total, unit}` plus top-level token counts; several
/// cost/model fields are nullable.
#[derive(Debug, Clone, Deserialize)]
pub struct Observation {
    #[serde(default, rename = "type")]
    pub obs_type: Option<String>,
    // Retained for shape fidelity / future per-model cost mapping.
    #[allow(dead_code)]
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default, rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(default, rename = "endTime")]
    pub end_time: Option<String>,
    #[serde(default, rename = "promptTokens")]
    pub prompt_tokens: Option<i64>,
    #[serde(default, rename = "completionTokens")]
    pub completion_tokens: Option<i64>,
    #[serde(default, rename = "totalTokens")]
    pub total_tokens: Option<i64>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default, rename = "calculatedTotalCost")]
    pub calculated_total_cost: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input: Option<i64>,
    #[serde(default)]
    pub output: Option<i64>,
    #[serde(default)]
    pub total: Option<i64>,
    // Token unit label (e.g. "TOKENS"); retained for shape fidelity.
    #[allow(dead_code)]
    #[serde(default)]
    pub unit: Option<String>,
}

impl Observation {
    pub fn is_generation(&self) -> bool {
        self.obs_type
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("generation"))
            .unwrap_or(false)
    }

    /// Prompt tokens by observed shape: top-level field, else nested `usage.input`. `None` (not
    /// `0`) when neither is present.
    pub fn prompt(&self) -> Option<i64> {
        self.prompt_tokens
            .or_else(|| self.usage.as_ref().and_then(|u| u.input))
    }

    pub fn completion(&self) -> Option<i64> {
        self.completion_tokens
            .or_else(|| self.usage.as_ref().and_then(|u| u.output))
    }

    pub fn total(&self) -> Option<i64> {
        self.total_tokens
            .or_else(|| self.usage.as_ref().and_then(|u| u.total))
    }

    pub fn cost(&self) -> Option<f64> {
        self.calculated_total_cost
    }

    /// True when a generation observation carries neither any token count nor any cost — the
    /// Claude-hook-silent-fail / unmapped-price condition that must degrade to `schema_changed`,
    /// never zero.
    pub fn lacks_usage_and_cost(&self) -> bool {
        self.is_generation()
            && self.prompt().is_none()
            && self.completion().is_none()
            && self.total().is_none()
            && self.cost().is_none()
    }
}

/// The normalized AI-evidence row persisted to `langfuse_ai_evidence`. Token/cost fields are
/// `Option` so absence is preserved as `NULL` — never coerced to `0`.
#[derive(Debug, Clone, PartialEq)]
pub struct AiEvidence {
    pub trace_id: String,
    pub environment: String,
    pub ai_start_ts: Option<String>,
    pub ai_end_ts: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost_total: Option<f64>,
    pub health: HealthState,
}

/// The time window an import run covers (ISO-8601 timestamps).
#[derive(Debug, Clone)]
pub struct ImportWindow {
    pub from: String,
    pub to: String,
}
