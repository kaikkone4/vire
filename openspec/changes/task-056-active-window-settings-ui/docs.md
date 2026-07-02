# Documentation gate — TASK-056 active-window capture Settings + privacy/status UI

- **Branch / PR**: feat/task-056-active-window-settings-ui · draft PR #43
- **Gate**: L2 documentation review, before SW-6 Release Manager

## Verdict: docs updated, no blockers

Read `handoff.md`, `qa.md`, `review.md`, `sec.md`, `docs/active-window-capture.md`, `specs/active-window-settings/spec.md`,
`tasks.md`, `proposal.md`, and the current `README.md`, cross-checked against the shipped source
(`src/active-window-settings-ui.ts`, `src/main.ts`, `src-tauri/src/active_window/settings_api.rs`).
Verified: default OFF, macOS-only gating, the Captured/Never-captured claims (no titles, screenshots,
Accessibility tree, keystrokes, mouse/clipboard, URLs, file paths, terminal command bodies, prompts,
secrets), status fields (`last_sample_ts`, `samples_today`, `evidence_blocks_retained`, `open_health`,
`recent_health`), and validation bounds all match the implementation. Found and fixed three stale/
incomplete spots; no other doc changes were needed — `docs/active-window-capture.md`'s core content (SW-2
workstream D) was already accurate.

## Changes made

- **`README.md`**
  - Top summary line still read *"no in-app UI in this release"* — false now that TASK-056 ships the
    Settings panel. Replaced with a truthful one-line description of the panel (toggle, cadence/retention
    controls, status readout, privacy table).
  - "Privacy status" section's capture paragraph only mentioned the env-var enable path; added the in-app
    Settings path alongside it.
  - Manual-verification step 2 asserted the literal removed string `Manual Mode / Capture deferred` and
    claimed "no automatic capture controls" — both false post-TASK-056 (that sidebar copy was replaced by
    `sidebarCaptureStatus()` in the SW-3 fix commit, and Settings now has real capture controls, off by
    default). Rewrote the step to assert the current truthful off-state copy.
  - Added a new **"Active-window capture Settings UI (TASK-056 — required before release)"** manual-
    verification section (steps 26–30), mirroring the existing per-task pattern (TASK-026/029/032/034) and
    the mandated physical-Mac smoke checklist in `tasks.md` §Smoke — so the still-outstanding physical-Mac
    smoke (flagged not-yet-run in `qa.md`) has a durable, discoverable checklist for whoever runs it.
- **`docs/active-window-capture.md`**: the safe-bounds prose in §"Enabling capture" omitted the upper
  bounds on `idle_candidate_seconds`/`idle_away_seconds` (86400s each) present in
  `settings_api::validate`. Completed the bounds statement to match the backend exactly.
- **`openspec/changes/task-056-active-window-settings-ui/tasks.md`**: workstreams A and B (backend IPC +
  status projection) were fully implemented and tested (confirmed via `lib.rs:909,920,1256-1257`,
  `store.rs:353`, and the `settings_api_*`/`status_snapshot_*` tests in `tests.rs`) but every checkbox was
  still unchecked, contradicting the shipped code — this was flagged as a non-blocking suggestion in
  `review.md`. Checked off A and B; left the §Smoke checkboxes unchecked since physical-Mac smoke has
  genuinely not run yet (per `qa.md`).

## Not changed (verified accurate, left as-is)

- `docs/active-window-capture.md` privacy table, "Not yet available" section, storage-table descriptions.
- `README.md` "Release compatibility and rollback" TASK-048 entry (still correct — no schema/table change
  from TASK-056, so no new entry needed there).
- `src/active-window-settings-ui.ts:35` comment ("rejects" vs "ignores/drops" unknown keys) — flagged in
  `review.md` as a source-code comment nitpick, outside this gate's `README.md`/`docs/`/OpenAPI scope.
- No OpenAPI spec exists in this repo (Tauri IPC app, no HTTP API surface) — nothing to update there.

## Checks

- `npx openspec validate task-056-active-window-settings-ui --strict` — passes (re-run after all edits).
- Manual cross-check of `CAPTURE_BOUNDS` (`src/active-window-settings-ui.ts`) and
  `settings_api::{MIN,MAX}_*` constants (`src-tauri/src/active_window/settings_api.rs:19-25`) against doc
  prose — now byte-consistent.
- Manual cross-check of privacy claims against `capturePanel`/`captureBanner`/`sidebarCaptureStatus`/
  `privacyTable` copy in `src/active-window-settings-ui.ts` — consistent across sidebar, Today banner,
  Settings panel, and both doc files; no view asserts a capture denial while capture is on, or vice versa.

## Blockers

None.

## Suggestions (non-blocking, out of this gate's scope)

- `src/active-window-settings-ui.ts:35` comment says backend "rejects" unknown keys; serde actually
  drops/ignores them (already flagged in `review.md`) — a source-code fix, not a docs fix.
