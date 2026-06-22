//! Unit tests for the production U-lite payload parsing and version comparison (TASK-050).
//! No network is used.

use super::{parse_release_payload, UpdateCheck};

fn payload(tag: &str) -> Vec<u8> {
    serde_json::json!({
        "tag_name": tag,
        "html_url": "https://github.com/kaikkonen4/vire/releases/tag/v0.9.0"
    })
    .to_string()
    .into_bytes()
}

#[test]
fn newer_release_is_update_available() {
    match parse_release_payload("0.8.0", &payload("v0.9.0")) {
        UpdateCheck::UpdateAvailable {
            current,
            latest,
            release_url,
        } => {
            assert_eq!(current, "0.8.0");
            assert_eq!(latest, "0.9.0");
            assert_eq!(
                release_url,
                "https://github.com/kaikkonen4/vire/releases/tag/v0.9.0"
            );
        }
        other => panic!("expected UpdateAvailable, got {:?}", other),
    }
}

#[test]
fn same_version_is_up_to_date() {
    match parse_release_payload("0.8.0", &payload("v0.8.0")) {
        UpdateCheck::UpToDate { current } => {
            assert_eq!(current, "0.8.0");
        }
        other => panic!("expected UpToDate, got {:?}", other),
    }
}

#[test]
fn lower_release_is_up_to_date() {
    // Running a dev build ahead of the last release — never "downgrade available".
    match parse_release_payload("0.9.0-dev", &payload("v0.8.0")) {
        UpdateCheck::UpToDate { .. } => {}
        other => panic!("expected UpToDate, got {:?}", other),
    }
}

#[test]
fn equal_dev_build_is_up_to_date() {
    // Running ahead of the last release (same base, pre-release label).
    match parse_release_payload("0.9.0", &payload("v0.9.0")) {
        UpdateCheck::UpToDate { .. } => {}
        other => panic!("expected UpToDate, got {:?}", other),
    }
}

#[test]
fn malformed_tag_yields_unknown() {
    match parse_release_payload("0.8.0", &payload("not-semver")) {
        UpdateCheck::Unknown { reason } => {
            assert!(reason.contains("not-semver"), "reason: {reason}");
        }
        other => panic!("expected Unknown, got {:?}", other),
    }
}

#[test]
fn empty_tag_yields_unknown() {
    match parse_release_payload("0.8.0", &payload("")) {
        UpdateCheck::Unknown { .. } => {}
        other => panic!("expected Unknown for empty tag, got {:?}", other),
    }
}

#[test]
fn malformed_current_yields_unknown() {
    // A garbage running version also maps to Unknown, not a panic.
    match parse_release_payload("not-a-version", &payload("v0.9.0")) {
        UpdateCheck::Unknown { reason } => {
            assert!(reason.contains("not-a-version"), "reason: {reason}");
        }
        other => panic!("expected Unknown, got {:?}", other),
    }
}

#[test]
fn strip_leading_v_prefix() {
    // "v0.9.0" and "0.9.0" are equivalent tag forms — both parse.
    match parse_release_payload("0.8.0", &payload("0.9.0")) {
        UpdateCheck::UpdateAvailable { latest, .. } => {
            assert_eq!(latest, "0.9.0");
        }
        other => panic!("expected UpdateAvailable, got {:?}", other),
    }
}

#[test]
fn malformed_payload_yields_unknown() {
    match parse_release_payload("0.8.0", br#"{"tag_name":"v0.9.0""#) {
        UpdateCheck::Unknown { reason } => {
            assert!(reason.contains("JSON parse error"), "reason: {reason}");
        }
        other => panic!("expected Unknown for malformed payload, got {:?}", other),
    }
}
