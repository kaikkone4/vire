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

/// Fixed, secret-free taxonomy of **why** a trace or observation was skipped or degraded during an
/// import (TASK-029 A / SEC-011 / DEC-031). Each variant maps to a stable `snake_case` label string
/// in the import report — never a passed-through `serde` error, field value, or payload content.
/// Classification is **structural**: it inspects the raw `serde_json::Value` shape (which keys are
/// present, what JSON type the offending field has), never the field's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// No usable string `id` on the entry — cannot identify it; the only identification failure that
    /// still drops a trace. Degrades the environment to `schema_changed` for review (counted).
    MissingTraceId,
    /// `observations` is present but its elements are identifier strings, not embedded objects — the
    /// current Langfuse v3 list shape. **Informational**: usage/cost are read from the observations
    /// fetch, so the trace is imported, not dropped.
    ObservationsNotEmbedded,
    /// An identification field carried an unexpected JSON type (e.g. a numeric `timestamp`). The
    /// trace is still imported via the tolerant reader; this records the offending field **name**
    /// only. **Informational** — a peripheral mismatch never drops an identifiable trace.
    FieldTypeMismatch,
    /// A generation observation had no token usage or cost in any supported location → the trace is
    /// imported as `schema_changed` (counted, surfaced for review), never a numeric zero.
    GenerationLacksUsageAndCost,
    /// The per-trace observations fetch failed → the trace is imported as `schema_changed`.
    ObservationsFetchFailed,
}

impl SkipReason {
    /// Every variant, in a fixed order, so the report's per-reason breakdown is deterministic.
    pub const ALL: [SkipReason; 5] = [
        SkipReason::MissingTraceId,
        SkipReason::ObservationsNotEmbedded,
        SkipReason::FieldTypeMismatch,
        SkipReason::GenerationLacksUsageAndCost,
        SkipReason::ObservationsFetchFailed,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            SkipReason::MissingTraceId => "missing_trace_id",
            SkipReason::ObservationsNotEmbedded => "observations_not_embedded",
            SkipReason::FieldTypeMismatch => "field_type_mismatch",
            SkipReason::GenerationLacksUsageAndCost => "generation_lacks_usage_and_cost",
            SkipReason::ObservationsFetchFailed => "observations_fetch_failed",
        }
    }

    /// Whether this reason is a genuine **drop or degrade** (counted as a schema issue and surfaced
    /// as `schema_changed`), as opposed to an **informational** anomaly the importer tolerates. Only
    /// genuine reasons drive the `skipped_schema` count and the environment health state, so the
    /// normal v3 list shape (`observations_not_embedded`) and tolerated field mismatches never force
    /// `schema_changed`.
    pub fn is_schema_issue(&self) -> bool {
        matches!(
            self,
            SkipReason::MissingTraceId
                | SkipReason::GenerationLacksUsageAndCost
                | SkipReason::ObservationsFetchFailed
        )
    }
}

/// One reason's aggregated count in the import report (secret-free: a fixed label + an integer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkipReasonCount {
    pub reason: String,
    pub count: usize,
}

/// A bounded, **secret-free** structural sample for a skip/degrade reason (SEC-011). It carries only
/// *structure*, never *content*:
/// - `keys`: the top-level JSON key **names** present on the entry (e.g. `["id","observations"]`);
/// - `field`: the offending field's **name** (for the field-scoped reasons);
/// - `field_type`: that field's JSON **type name** (`"string"`/`"array"`/…), never its value;
/// - `element_type`: for an array field, the JSON type name of its first element.
///
/// A sample MUST NOT contain any field value, nested object/array contents, raw payload bytes, a
/// `serde` error string, or any prompt/session/metadata/credential material. Type and key **names**
/// are structure, not content — this is the SEC-011 invariant and is negative-tested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkipSample {
    pub reason: String,
    pub keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_type: Option<String>,
}

/// True when a raw entry's `observations` field is the current Langfuse v3 **identifier-list**
/// shape: a non-empty array carrying at least one non-object element (an observation ID string)
/// rather than only embedded observation objects. Drives the informational `observations_not_embedded`
/// classification — the trace is still imported (usage/cost come from the observations fetch).
pub fn observations_is_id_list(value: &serde_json::Value) -> bool {
    value
        .get("observations")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty() && arr.iter().any(|el| !el.is_object()))
        .unwrap_or(false)
}

/// Name the first identification field on a raw entry whose JSON **type** is unexpected (a present,
/// non-null value that is not the type the typed [`Trace`] expects: string for `timestamp`/
/// `environment`/`name`/`sessionId`, number for `totalCost`). Returns `None` when every present
/// identification field has an acceptable type. Used only to label a `field_type_mismatch` sample
/// with the offending field **name** — never its value (SEC-011).
pub fn offending_identification_field(value: &serde_json::Value) -> Option<&'static str> {
    const STRING_FIELDS: [&str; 4] = ["timestamp", "environment", "name", "sessionId"];
    for &field in &STRING_FIELDS {
        if let Some(v) = value.get(field) {
            if !v.is_null() && !v.is_string() {
                return Some(field);
            }
        }
    }
    if let Some(v) = value.get("totalCost") {
        if !v.is_null() && !v.is_number() {
            return Some("totalCost");
        }
    }
    None
}

/// The JSON **type name** of a value — pure structure, never the value itself (SEC-011). Used to
/// describe an offending field in a [`SkipSample`] without copying any content.
pub fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
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
    /// Embedded observation objects when the source carries them (legacy/fast path), OR an empty
    /// vec when the source used the current Langfuse v3 list shape (an array of observation **ID
    /// strings**) — in which case usage/cost come from the `get_observations(trace_id)` fetch. The
    /// tolerant deserializer keeps object elements and drops identifier strings, so the v3 shape
    /// never fails the whole-trace parse (TASK-029 B / DEC-031).
    #[serde(default, deserialize_with = "deserialize_tolerant_observations")]
    pub observations: Vec<Observation>,
}

/// Tolerantly deserialize the `observations` field across Langfuse shapes (TASK-029 B / DEC-031).
/// The legacy/embedded shape carries observation **objects**; the current v3 list shape carries
/// observation **ID strings**. This keeps every element that is a JSON object (the fast path) and
/// silently drops non-object elements (ID strings, nulls). A non-array value (or `null`) yields an
/// empty vec. The dropped/ID-only case falls through to the authoritative `get_observations`
/// fetch, so the whole-trace parse never fails merely because the source used the identifier-list
/// shape.
fn deserialize_tolerant_observations<'de, D>(deserializer: D) -> Result<Vec<Observation>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(deserializer)?;
    Ok(observation_objects(&raw))
}

/// Extract embedded [`Observation`] objects from a raw `observations` value, ignoring identifier
/// strings, nulls, and any non-array shape. Shared by the tolerant deserializer and the
/// identification-first [`Trace::from_value_tolerant`] reader.
fn observation_objects(value: &serde_json::Value) -> Vec<Observation> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|el| el.is_object())
                .filter_map(|el| serde_json::from_value::<Observation>((*el).clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Read a usable string trace `id` from a raw list-payload entry, or `None` when it is absent,
/// non-string, or blank. This is the single identification predicate (TASK-029 B1 / DEC-031): a
/// trace with an id is always importable; one without is the only entry the importer still skips
/// (classified `missing_trace_id`).
pub fn trace_id(value: &serde_json::Value) -> Option<String> {
    value
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

impl Trace {
    /// Identification-first tolerant construction from a raw list-payload entry (TASK-029 B1 /
    /// DEC-031). A peripheral field with an unexpected JSON type (e.g. a numeric `timestamp`) is
    /// read as absent rather than dropping the whole trace; `observations` keeps embedded objects
    /// and ignores identifier-string elements. Returns `None` only when the entry has no usable
    /// string `id` — the sole identification failure. Usage/cost are NOT read here (they come from
    /// observations), so a tolerated mismatch never fabricates a token or cost value.
    pub fn from_value_tolerant(value: &serde_json::Value) -> Option<Trace> {
        let id = trace_id(value)?;
        let str_field = |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_string);
        Some(Trace {
            id,
            timestamp: str_field("timestamp"),
            environment: str_field("environment"),
            name: str_field("name"),
            session_id: str_field("sessionId"),
            total_cost: value.get("totalCost").and_then(|v| v.as_f64()),
            metadata: value
                .get("metadata")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            observations: value
                .get("observations")
                .map(observation_objects)
                .unwrap_or_default(),
        })
    }
}

/// A generation observation — where token usage and model cost actually live. Read by observed
/// shape across Langfuse versions: the legacy `usage` = `{input, output, total, unit}` map plus
/// top-level `promptTokens`/`completionTokens`/`totalTokens` AND the current `usageDetails` /
/// `costDetails` maps the live stack (v3, verified at SW-2 against the local 3.178.0 instance and
/// the TASK-007 shape-only spike) returns. Several cost/model fields are nullable; absence in
/// every supported location stays absent, never `0`.
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
    /// Current-shape token usage map (e.g. `{"input": N, "output": M, "total": T, …}`). May be
    /// absent or empty on older shapes; per-model extra keys (cache/reasoning tokens) are ignored —
    /// only the conventional `input`/`output`/`total` keys are read.
    #[serde(default, rename = "usageDetails")]
    pub usage_details: Option<serde_json::Map<String, serde_json::Value>>,
    /// Current-shape cost breakdown map (e.g. `{"input": x, "output": y, "total": z}`). The
    /// authoritative per-call total is `costDetails["total"]`; absent ⇒ `None`.
    #[serde(default, rename = "costDetails")]
    pub cost_details: Option<serde_json::Map<String, serde_json::Value>>,
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

    /// Prompt tokens by observed shape, in precedence order: legacy top-level `promptTokens`,
    /// else nested legacy `usage.input`, else current `usageDetails["input"]`. `None` (not `0`)
    /// when none is present — absence is preserved across all supported shapes.
    pub fn prompt(&self) -> Option<i64> {
        self.prompt_tokens
            .or_else(|| self.usage.as_ref().and_then(|u| u.input))
            .or_else(|| self.usage_detail_tokens("input"))
    }

    pub fn completion(&self) -> Option<i64> {
        self.completion_tokens
            .or_else(|| self.usage.as_ref().and_then(|u| u.output))
            .or_else(|| self.usage_detail_tokens("output"))
    }

    pub fn total(&self) -> Option<i64> {
        self.total_tokens
            .or_else(|| self.usage.as_ref().and_then(|u| u.total))
            .or_else(|| self.usage_detail_tokens("total"))
    }

    /// Per-call cost: legacy `calculatedTotalCost`, else current `costDetails["total"]`. `None`
    /// (not `0`) when neither is present.
    pub fn cost(&self) -> Option<f64> {
        self.calculated_total_cost
            .or_else(|| self.cost_detail("total"))
    }

    /// Read a token count from the current `usageDetails` map by key, tolerating integer or float
    /// JSON encodings. An absent or null key returns `None`, never `0` (absence-≠-zero); a present
    /// `0` reads as `Some(0)`.
    fn usage_detail_tokens(&self, key: &str) -> Option<i64> {
        self.usage_details
            .as_ref()
            .and_then(|m| m.get(key))
            .and_then(json_number_as_i64)
    }

    /// Read a cost component from the current `costDetails` map by key. Absent/null ⇒ `None`.
    fn cost_detail(&self, key: &str) -> Option<f64> {
        self.cost_details
            .as_ref()
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_f64())
    }

    /// True when a generation observation carries neither any token count nor any cost in **any**
    /// supported location (legacy or current) — the Claude-hook-silent-fail / unmapped-price
    /// condition that must degrade to `schema_changed`, never zero.
    pub fn lacks_usage_and_cost(&self) -> bool {
        self.is_generation()
            && self.prompt().is_none()
            && self.completion().is_none()
            && self.total().is_none()
            && self.cost().is_none()
    }
}

/// Read a JSON number as `i64`, tolerating a float encoding (e.g. `10.0`). A non-number or `null`
/// value yields `None`, so an absent/typeless `usageDetails` entry never becomes a fabricated `0`.
fn json_number_as_i64(value: &serde_json::Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_f64().map(|f| f as i64))
}

/// The normalized AI-evidence row persisted to `langfuse_ai_evidence`. Token/cost fields are
/// `Option` so absence is preserved as `NULL` — never coerced to `0`.
#[derive(Debug, Clone, PartialEq)]
pub struct AiEvidence {
    pub trace_id: String,
    pub environment: String,
    /// Hashed `session-<hash>` id from the trace, surfaced (privacy-positive) so the runtime
    /// observer (TASK-022) matches against this normalized row instead of the prompt-bearing raw
    /// payload. Opaque, not content; `None` when the trace carried no session id.
    pub session_id: Option<String>,
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
