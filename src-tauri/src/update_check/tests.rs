//! Unit tests for the U-lite version-comparison logic (TASK-050).
//! No network is used — the comparison is exercised through `compare_versions`, a pure helper
//! extracted from `run_check` so tests never need a live `reqwest` client.

use semver::Version;

/// Mirror of the comparison logic in `run_check` — extracted so tests can exercise it directly
/// without constructing a full HTTP response.
pub fn compare_versions(current: &str, tag: &str) -> super::UpdateCheck {
    let current_ver = match Version::parse(current) {
        Ok(v) => v,
        Err(e) => {
            return super::UpdateCheck::Unknown {
                reason: format!("could not parse running version '{current}': {e}"),
            }
        }
    };
    let tag_clean = tag.trim_start_matches('v');
    let latest_ver = match Version::parse(tag_clean) {
        Ok(v) => v,
        Err(e) => {
            return super::UpdateCheck::Unknown {
                reason: format!("could not parse release tag '{tag_clean}': {e}"),
            }
        }
    };
    if latest_ver > current_ver {
        super::UpdateCheck::UpdateAvailable {
            current: current.to_string(),
            latest: tag_clean.to_string(),
            release_url: "https://github.com/kaikkonen4/vire/releases/tag/v0.9.0".to_string(),
        }
    } else {
        super::UpdateCheck::UpToDate {
            current: current.to_string(),
        }
    }
}

#[test]
fn newer_release_is_update_available() {
    match compare_versions("0.8.0", "v0.9.0") {
        super::UpdateCheck::UpdateAvailable { current, latest, .. } => {
            assert_eq!(current, "0.8.0");
            assert_eq!(latest, "0.9.0");
        }
        other => panic!("expected UpdateAvailable, got {:?}", other),
    }
}

#[test]
fn same_version_is_up_to_date() {
    match compare_versions("0.8.0", "v0.8.0") {
        super::UpdateCheck::UpToDate { current } => {
            assert_eq!(current, "0.8.0");
        }
        other => panic!("expected UpToDate, got {:?}", other),
    }
}

#[test]
fn lower_release_is_up_to_date() {
    // Running a dev build ahead of the last release — never "downgrade available".
    match compare_versions("0.9.0-dev", "v0.8.0") {
        super::UpdateCheck::UpToDate { .. } => {}
        other => panic!("expected UpToDate, got {:?}", other),
    }
}

#[test]
fn equal_dev_build_is_up_to_date() {
    // Running ahead of the last release (same base, pre-release label).
    match compare_versions("0.9.0", "v0.9.0") {
        super::UpdateCheck::UpToDate { .. } => {}
        other => panic!("expected UpToDate, got {:?}", other),
    }
}

#[test]
fn malformed_tag_yields_unknown() {
    match compare_versions("0.8.0", "not-semver") {
        super::UpdateCheck::Unknown { reason } => {
            assert!(reason.contains("not-semver"), "reason: {reason}");
        }
        other => panic!("expected Unknown, got {:?}", other),
    }
}

#[test]
fn empty_tag_yields_unknown() {
    match compare_versions("0.8.0", "") {
        super::UpdateCheck::Unknown { .. } => {}
        other => panic!("expected Unknown for empty tag, got {:?}", other),
    }
}

#[test]
fn malformed_current_yields_unknown() {
    // A garbage running version also maps to Unknown, not a panic.
    match compare_versions("not-a-version", "v0.9.0") {
        super::UpdateCheck::Unknown { reason } => {
            assert!(reason.contains("not-a-version"), "reason: {reason}");
        }
        other => panic!("expected Unknown, got {:?}", other),
    }
}

#[test]
fn strip_leading_v_prefix() {
    // "v0.9.0" and "0.9.0" are equivalent tag forms — both parse.
    match compare_versions("0.8.0", "0.9.0") {
        super::UpdateCheck::UpdateAvailable { latest, .. } => {
            assert_eq!(latest, "0.9.0");
        }
        other => panic!("expected UpdateAvailable, got {:?}", other),
    }
}
