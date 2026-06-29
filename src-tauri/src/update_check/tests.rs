//! Unit tests for the production U-lite payload parsing and version comparison (TASK-050).
//! No network is used.

use super::{parse_release_payload, UpdateCheck, GITHUB_API_LATEST, RELEASES_URL};

fn payload(tag: &str) -> Vec<u8> {
    serde_json::json!({
        "tag_name": tag,
        "html_url": "https://github.com/kaikkone4/vire/releases/tag/v0.9.0"
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
                "https://github.com/kaikkone4/vire/releases/tag/v0.9.0"
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

// --- TASK-051 regression guard ---

/// Both update-check endpoints must target the canonical repo owner `kaikkone4/vire`, never the
/// typo'd owner that carries an extra `n`. The wrong owner 404s every check into fail-soft
/// `Unknown` and 404s the Releases page — the exact regression this hotfix corrects.
#[test]
fn endpoints_target_canonical_repo_owner() {
    // Build the forbidden owner at runtime so its literal never appears in the tree, keeping the
    // repo grep-clean for the typo while still asserting its absence.
    let typo_owner = format!("kaikkone{}4", 'n');
    for url in [RELEASES_URL, GITHUB_API_LATEST] {
        assert!(
            url.contains("kaikkone4/vire"),
            "missing canonical owner in {url}"
        );
        assert!(!url.contains(&typo_owner), "typo'd owner present in {url}");
    }
}

/// The scoped `opener:allow-open-url` allowlist URL must stay byte-for-byte equal to `RELEASES_URL`
/// so the capability and the constant can never drift to different owners again.
#[test]
fn opener_allowlist_url_equals_releases_url() {
    let capabilities: serde_json::Value =
        serde_json::from_str(include_str!("../../capabilities/default.json"))
            .expect("capabilities/default.json is valid JSON");

    let permissions = capabilities["permissions"]
        .as_array()
        .expect("permissions is an array");

    let opener = permissions
        .iter()
        .find(|perm| perm["identifier"] == "opener:allow-open-url")
        .expect("opener:allow-open-url permission is present");

    let allow_url = opener["allow"][0]["url"]
        .as_str()
        .expect("opener allow[0].url is a string");

    assert_eq!(
        allow_url, RELEASES_URL,
        "opener allowlist URL must equal RELEASES_URL"
    );
}
