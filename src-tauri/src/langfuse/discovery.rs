//! Read-only Langfuse environment discovery (TASK-027 C / DEC-028).
//!
//! The Langfuse public API has no list-environments endpoint, so discovery scans recent traces
//! **without** the `environment` filter and collects the distinct, non-empty `Trace.environment`
//! values the page carries. It is a pure read over the `LangfuseApi` trait — network-free in tests
//! against the in-memory mock — and stays inside the same `/api/public/` allowlist and loopback
//! gate as trace import (the URL is built by `ImporterConfig::build_url`). No credential, raw
//! payload, or trace content leaves this module — only environment **names** (SEC-010).

use std::collections::BTreeSet;

use super::api::LangfuseApi;
use super::model::{ApiError, ImportWindow, Trace};

/// Page size for the discovery scan (mirrors the importer's trace page size).
pub const DISCOVERY_PAGE_LIMIT: u32 = 50;
/// Hard pagination backstop so a wrong `totalPages` can never spin the scan forever. `pub` so the
/// look-back-window tests can assert an `all`-range discovery degrades to "as many pages as the
/// backstop allowed" rather than spinning (TASK-045 B2/C5).
pub const MAX_PAGES: u32 = 1000;

/// Discover the distinct environments present in the source over `window`. Paginates
/// `GET /api/public/traces` without the environment filter and returns the sorted set of
/// non-empty `Trace.environment` values. A transport/API failure short-circuits with `Err` so the
/// caller can treat discovery as best-effort (a failed scan never fabricates an environment list).
///
/// VERIFICATION (C2): this relies on the source returning cross-environment traces when the
/// `environment` filter is omitted, with `Trace.environment` populated on the list payload —
/// confirmed shape-only against the live local stack (the field is present on the list view per the
/// TASK-007 round-trip). If a future source omitted `environment` on the list, a trace yields no
/// name and is simply skipped (the advanced hand-entered environment list remains the fallback);
/// discovery degrades to an empty result, never wrong data.
pub fn discover_environments(
    api: &dyn LangfuseApi,
    window: &ImportWindow,
) -> Result<Vec<String>, ApiError> {
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut page = 1u32;
    loop {
        let tp = api.get_traces_any_env(&window.from, &window.to, page, DISCOVERY_PAGE_LIMIT)?;
        for value in &tp.data {
            // A trace that doesn't match the expected shape is skipped (it contributes no name);
            // an unparseable page never aborts discovery of the rest.
            if let Ok(trace) = serde_json::from_value::<Trace>(value.clone()) {
                if let Some(env) = trace.environment {
                    let env = env.trim().to_string();
                    if !env.is_empty() {
                        found.insert(env);
                    }
                }
            }
        }
        let total_pages = tp.meta.total_pages;
        if tp.data.is_empty() || page >= total_pages || page >= MAX_PAGES {
            break;
        }
        page += 1;
    }
    Ok(found.into_iter().collect())
}
