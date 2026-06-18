//! The read-only Langfuse REST client interface plus its `reqwest` (blocking, rustls)
//! implementation in the Rust core. The importer engine depends only on the `LangfuseApi`
//! trait, so its logic is exercised against an in-memory mock with no network or credentials.

use std::time::Duration;

use serde_json::Value;

use super::config::{ApiPath, ImporterConfig};
use super::model::{ApiError, ApiErrorKind, Observation, TracePage};

/// Read-only contract the importer uses. GET-only; the importer never writes to Langfuse and
/// never pushes local activity (SEC-002).
pub trait LangfuseApi {
    /// Cheap availability check. `Err(Unavailable)` means the Docker/Langfuse stack is down or
    /// unreachable — which must surface as `unavailable`, never as zero usage/cost.
    fn probe(&self) -> Result<(), ApiError>;

    /// One page of `GET /api/public/traces` for an environment + time window, ordered explicitly by
    /// `order_by` (a fixed `[field].[asc|desc]` literal — the importer always passes `timestamp.asc`
    /// so the inclusive-`fromTimestamp` resume-cursor walks oldest → newest, DEC-032).
    fn get_traces(
        &self,
        environment: &str,
        from: &str,
        to: &str,
        page: u32,
        limit: u32,
        order_by: &str,
    ) -> Result<TracePage, ApiError>;

    /// One page of `GET /api/public/traces` over a time window **without** the `environment`
    /// filter, used only by environment discovery (TASK-027 C). The response spans every
    /// environment; the caller reads the distinct `Trace.environment` values. Read-only and bound
    /// by the same allowlist/loopback gate as [`get_traces`] (the URL is built under `/api/public/`).
    fn get_traces_any_env(
        &self,
        from: &str,
        to: &str,
        page: u32,
        limit: u32,
    ) -> Result<TracePage, ApiError>;

    /// Generation observations for a trace (used only when the list payload did not already
    /// embed `observations[]`). Usage/cost are read from here, not the trace body.
    fn get_observations(&self, trace_id: &str) -> Result<Vec<Observation>, ApiError>;
}

/// `reqwest`-backed client. Holds the validated config; every request URL is built through
/// `ImporterConfig::build_url`, so the allowlist is enforced on every call.
pub struct ReqwestLangfuseApi {
    client: reqwest::blocking::Client,
    config: ImporterConfig,
}

impl ReqwestLangfuseApi {
    pub fn new(config: ImporterConfig) -> Result<Self, ApiError> {
        config
            .validate_target()
            .map_err(|e| ApiError::new(ApiErrorKind::Indeterminate, e.0))?;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            // Never auto-follow redirects: a redirect could point off the allowlisted origin.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("vire-langfuse-importer")
            .build()
            .map_err(|_| {
                ApiError::new(ApiErrorKind::Indeterminate, "failed to build HTTP client")
            })?;
        Ok(ReqwestLangfuseApi { client, config })
    }

    fn get_json(&self, path: ApiPath<'_>) -> Result<Value, ApiError> {
        let url = self
            .config
            .build_url(&path)
            .map_err(|e| ApiError::new(ApiErrorKind::Indeterminate, e.0))?;
        let mut req = self.client.get(url);
        if let Some(creds) = &self.config.credentials {
            req = req.basic_auth(&creds.public_key, Some(creds.secret_key.expose()));
        }
        let resp = req.send().map_err(map_transport_error)?;
        let status = resp.status();
        if status.is_success() {
            return resp.json::<Value>().map_err(|_| {
                ApiError::new(
                    ApiErrorKind::Indeterminate,
                    "Langfuse response was not valid JSON",
                )
            });
        }
        Err(map_status_error(status.as_u16()))
    }
}

impl LangfuseApi for ReqwestLangfuseApi {
    fn probe(&self) -> Result<(), ApiError> {
        self.get_json(ApiPath::Health).map(|_| ())
    }

    fn get_traces(
        &self,
        environment: &str,
        from: &str,
        to: &str,
        page: u32,
        limit: u32,
        order_by: &str,
    ) -> Result<TracePage, ApiError> {
        let value = self.get_json(ApiPath::Traces {
            environment,
            from,
            to,
            page,
            limit,
            order_by,
        })?;
        serde_json::from_value(value).map_err(|_| {
            ApiError::new(
                ApiErrorKind::Indeterminate,
                "Langfuse traces page had an unexpected shape",
            )
        })
    }

    fn get_traces_any_env(
        &self,
        from: &str,
        to: &str,
        page: u32,
        limit: u32,
    ) -> Result<TracePage, ApiError> {
        let value = self.get_json(ApiPath::TracesAllEnvironments {
            from,
            to,
            page,
            limit,
        })?;
        serde_json::from_value(value).map_err(|_| {
            ApiError::new(
                ApiErrorKind::Indeterminate,
                "Langfuse traces page had an unexpected shape",
            )
        })
    }

    fn get_observations(&self, trace_id: &str) -> Result<Vec<Observation>, ApiError> {
        let value = self.get_json(ApiPath::Observations { trace_id })?;
        // Observations are returned under `data` like traces.
        let data = value.get("data").cloned().unwrap_or(value);
        serde_json::from_value(data).map_err(|_| {
            ApiError::new(
                ApiErrorKind::Indeterminate,
                "Langfuse observations had an unexpected shape",
            )
        })
    }
}

/// Map a transport error to a health-bearing `ApiError`. Crucially this never includes the URL,
/// headers, or any credential material — only the failure class.
fn map_transport_error(err: reqwest::Error) -> ApiError {
    if err.is_connect() {
        ApiError::new(
            ApiErrorKind::Unavailable,
            "could not connect to the local Langfuse stack",
        )
    } else if err.is_timeout() {
        ApiError::new(ApiErrorKind::Network, "Langfuse request timed out")
    } else {
        ApiError::new(ApiErrorKind::Network, "Langfuse request failed")
    }
}

fn map_status_error(status: u16) -> ApiError {
    match status {
        401 | 403 => ApiError::new(
            ApiErrorKind::Auth,
            "Langfuse rejected the credentials (auth error)",
        ),
        429 => ApiError::new(
            ApiErrorKind::RateLimited,
            "Langfuse rate limited the import",
        ),
        500..=599 => ApiError::new(
            ApiErrorKind::Unavailable,
            "Langfuse stack returned a server error",
        ),
        _ => ApiError::new(
            ApiErrorKind::Indeterminate,
            format!("Langfuse returned HTTP {status}"),
        ),
    }
}
