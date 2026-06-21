# Handoff — TASK-044 keychain-public-key-to-settings

- **Change dir**: `openspec/changes/task-044-keychain-public-key-to-settings/`
- **Branch / PR**: `feat/task-044-keychain-public-key-to-settings` / PR #32
- **Phase / gate**: SW-4 Code Review **PASS** + SW-5 Security **PASS** (2026-06-21, recheck after `db2eeef`)
- **Reviewed commits**: `1160f04` + `db2eeef`

## Gate result

SW-4 PASS. The architect fix implements pair-level env fallback, renders both half-state directions
inert, prevents mixed stored/env pairs, surfaces failed set compensation with distinct secret-free
errors, and changes clear to SQLite-first with public-key restoration on Keychain delete failure.
The renderer IPC payload and command names are unchanged.

Checks: 33 settings tests PASS; `cargo fmt` PASS; `npm run build` PASS; source diff check PASS.
Clippy reports only the documented pre-existing findings in untouched files.

SW-5 PASS (security gate; see `sec.md`). Tier-1 stack clean: semgrep 0 ERROR, gitleaks 0 in
commits/source (3 FPs in gitignored `target/` artifacts), OSV no CVSS ≥ 7.0, Trivy 0 HIGH/CRITICAL.
Manual: public = non-secret in SQLite; secret Keychain-only / presence-only; pair-level env fallback
makes a mixed-source pair structurally impossible; error paths secret-free; one-store windows
rendered inert by the resolver; no new deps/egress/capabilities (`lib.rs` adds only the Tauri-injected
`State` handle, renderer arg shape unchanged). The prior PASS-but-gated release block (Architect
escalation) is **resolved** by `db2eeef`; no FAIL-DESIGN raised.

## Blocking issues

None.

## Non-blocking suggestion

Add direct coverage for the `INCONSISTENT_CLEAR_ERR` restore-failure branch; see `review.md`.

## Next action

SW-6 DOCS COMPLETE (2026-06-21). Documentation Engineer gate PASS. Changes applied:
- `README.md`: version bump v0.6.0→v0.6.2; Secrets bullet corrected (public key → SQLite,
  secret → Keychain only); existing-install re-save note added; rollback Data bullet adds
  TASK-044 row; packaged-app step 7 corrected to one Keychain entry.
- `docs/langfuse-local-setup.md`: endpoint config §1 corrected (public key in SQLite, not
  Keychain); env var table notes corrected for public key and pair-only requirement (DEC-026).
PR #32 ready-for-review. Tag `task-044/v0.6.2` pending SSH key (dry-run in RELEASE.md).
Manual macOS Keychain dialog verification still pending (non-blocking).
