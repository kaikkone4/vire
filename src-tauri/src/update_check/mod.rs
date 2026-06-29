//! U-lite update-available check (TASK-050 / DEC-029 Phase-1).
//!
//! Two IPC commands are exposed:
//! - `check_for_update` — single read-only HTTPS GET to `api.github.com`; returns an `UpdateCheck`
//!   enum (UpToDate / UpdateAvailable / Unknown). Never uses `tauri-plugin-updater`, never downloads
//!   or installs anything, never stores or forwards any app/user data (SEC-002 amendment).
//! - `open_releases_page` — opens `RELEASES_URL` (a compile-time constant) in the OS default
//!   browser via `tauri-plugin-opener`. The renderer supplies no URL.
//!
//! HTTP client mirrors `langfuse/api.rs`: blocking + rustls, connect_timeout 5s, total timeout 10s,
//! `redirect::Policy::none()`, fixed user-agent. The command moves that bounded blocking work to
//! Tauri's blocking pool. No Authorization header, no body, no egress of any app/user/activity data.
//! Every failure path maps to `Unknown { reason }` — fail-soft, no panic.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle};
use tauri_plugin_opener::OpenerExt;

/// The Releases index — a compile-time constant the renderer can never influence.
/// Targets the index (not `/latest`), which needs no redirect.
const RELEASES_URL: &str = "https://github.com/kaikkone4/vire/releases";

/// GitHub latest-release endpoint for a public repo. No auth required.
const GITHUB_API_LATEST: &str = "https://api.github.com/repos/kaikkone4/vire/releases/latest";

/// The three states the update check can return. `Unknown` is fail-soft: the app stays fully
/// usable and no error dialog is shown.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UpdateCheck {
    UpToDate {
        current: String,
    },
    UpdateAvailable {
        current: String,
        latest: String,
        release_url: String,
    },
    Unknown {
        reason: String,
    },
}

/// Raw subset of the GitHub `/releases/latest` response we care about.
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

/// Build the one-shot reqwest client, mirroring `langfuse/api.rs` posture:
/// blocking + rustls, no redirects (a redirect could leave the allowlisted origin),
/// bounded timeouts, fixed user-agent (GitHub requires a UA).
fn build_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("vire-update-check")
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))
}

/// Compare the running version with a GitHub release. Kept pure so all production parsing and
/// comparison branches can be covered without live network access.
fn compare_release(current: &str, release: GithubRelease) -> UpdateCheck {
    // Strip a leading 'v' from the tag (e.g. "v0.9.0" → "0.9.0").
    let tag = release.tag_name.trim_start_matches('v');

    let current_ver = match semver::Version::parse(current) {
        Ok(v) => v,
        Err(e) => {
            return UpdateCheck::Unknown {
                reason: format!("could not parse running version '{current}': {e}"),
            }
        }
    };

    let latest_ver = match semver::Version::parse(tag) {
        Ok(v) => v,
        Err(e) => {
            return UpdateCheck::Unknown {
                reason: format!("could not parse release tag '{tag}': {e}"),
            }
        }
    };

    // "Update available" iff latest > running. Equal or lower (dev build ahead of last release) ⇒
    // up to date, never "downgrade available".
    if latest_ver > current_ver {
        UpdateCheck::UpdateAvailable {
            current: current.to_string(),
            latest: tag.to_string(),
            release_url: release.html_url,
        }
    } else {
        UpdateCheck::UpToDate {
            current: current.to_string(),
        }
    }
}

/// Parse the bounded GitHub response payload and run the production comparison logic.
fn parse_release_payload(current: &str, payload: &[u8]) -> UpdateCheck {
    match serde_json::from_slice::<GithubRelease>(payload) {
        Ok(release) => compare_release(current, release),
        Err(e) => UpdateCheck::Unknown {
            reason: format!("JSON parse error: {e}"),
        },
    }
}

/// Perform the version check. Every error path returns `Unknown { reason }` — the caller never
/// panics, never retries, and never surfaces an error dialog.
pub fn run_check() -> UpdateCheck {
    let current = env!("CARGO_PKG_VERSION").to_string();

    let client = match build_client() {
        Ok(c) => c,
        Err(e) => return UpdateCheck::Unknown { reason: e },
    };

    let resp = match client
        .get(GITHUB_API_LATEST)
        .header("Accept", "application/vnd.github+json")
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            return UpdateCheck::Unknown {
                reason: format!("network error: {e}"),
            }
        }
    };

    if !resp.status().is_success() {
        return UpdateCheck::Unknown {
            reason: format!("GitHub API returned {}", resp.status()),
        };
    }

    let payload = match resp.bytes() {
        Ok(body) => body,
        Err(e) => {
            return UpdateCheck::Unknown {
                reason: format!("response read error: {e}"),
            }
        }
    };

    parse_release_payload(&current, &payload)
}

/// IPC command: perform a single fail-soft update check. The bounded blocking request runs on
/// Tauri's blocking task pool; the renderer asynchronously awaits the one-shot result. Never panics.
#[command]
pub async fn check_for_update() -> UpdateCheck {
    match tauri::async_runtime::spawn_blocking(run_check).await {
        Ok(result) => result,
        Err(e) => UpdateCheck::Unknown {
            reason: format!("update check task failed: {e}"),
        },
    }
}

/// IPC command: open the GitHub Releases index in the OS default browser via `tauri-plugin-opener`.
/// The URL is a compile-time constant — the renderer supplies no URL, mitigating open-redirect risk.
#[command]
pub fn open_releases_page(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(RELEASES_URL, None::<&str>)
        .map_err(|e| format!("could not open releases page: {e}"))
}

#[cfg(test)]
pub(crate) mod tests;
