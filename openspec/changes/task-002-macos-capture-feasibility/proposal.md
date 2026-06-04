# TASK-002 — macOS capture feasibility spike

## Why

Phase A (`05_project_plan_epics.md` §3, row TASK-002; `04_technical_plan.md` §5) requires a
**spike** to validate whether local macOS app/window evidence can be captured at the quality
Janne's daily reconstruction needs, and at an acceptable permission burden, **before** any
capture MVP (TASK-005) or implementation-path decision (TASK-003) is committed. The highest
EPIC-001 risk is capture reliability and permission cost, not desktop-shell creation
(`04_technical_plan.md` §3). This change produces the evidence base that resolves the
"macOS title capture is inconsistent" and "Quartz requires intrusive permission" risks
(`05` §6 risk register) and feeds the Tauri+helper-vs-Swift-first signal into TASK-003.

This is a **spike/assessment only**. It produces a capture-feasibility report, a permission-burden
analysis, a field allowlist (APP-005/SEC-001) implication map, and a manual validation matrix.
It does **not** ship the capture adapter, write to the product datastore, persist real activity
evidence, or alter product runtime. TASK-001's salvage inventory
(`salvage-reuse-inventory.md`) classified the existing manual-tracker surface (`time_entries`
table, manual-entry view, stopwatch CRUD) as **reference-only**; this change preserves that
classification and must not entangle spike work with it.

## What Changes

- Validate **NSWorkspace / NSRunningApplication** frontmost-app capture: bundle id/name +
  timestamp at a small sampling interval; app switch, launch/terminate, sleep/wake, multiple
  Spaces, and full-screen behaviour. Record whether any TCC permission is actually required for
  the frontmost-app signal (the UX prototype surfaces an "App & activity" permission row — the
  spike must confirm or correct that).
- Validate **AXUIElement** focused-window/title capture where Accessibility is granted; record
  permission-denied / unavailable / redacted states explicitly as `capture_health`-shaped data.
- Evaluate the optional **Quartz Window Services** fallback and whether the selected calls
  require **Screen Recording** permission on Janne's macOS version; recommend Quartz only if the
  title-matching benefit exceeds the added permission burden.
- Validate an **idle/away** signal (CGEventSource last-event-age or equivalent) and its conversion
  to `active` / `idle_candidate` / `away` states after configurable thresholds.
- Document **degraded states and capture health**: permission state, sampling gaps, app/window
  unavailable, sleep/wake, helper crash/restart.
- Benchmark **ActivityWatch** as a reference only (evidence quality / permission model), without
  adopting it.
- Produce the **field allowlist (positive)** and **explicit non-collection list** with their
  APP-005 / SEC-001 implications, aligned to the six-field UX evidence record
  (`ux/prototype/setup-permissions.html`: day, time_range, app_name, window_title, source,
  review_state).
- Produce a **manual validation matrix** covering permission grant/revoke, degraded states,
  sleep/wake, Spaces/full-screen, core-app title availability, idle thresholds, and Quartz
  permission burden.
- If exploratory probe code is needed, isolate it under a clearly-named spike path
  (`spikes/task-002-macos-capture/`) that is **not** part of any shipped build target, never
  persists real private titles (redaction / synthetic output / ephemeral local logs with
  documented cleanup), and is never woven into `src/`, `src-tauri/src/` product runtime, or
  `observability/`.

## Impact

- **Affected specs:** adds `macos-capture-feasibility` capability (spike deliverables + isolation
  and collection guardrails).
- **Affected code:** none under product runtime. No source, schema, or configuration under `src/`,
  `src-tauri/src/`, or `observability/` is modified. Any probe code is confined to the isolated,
  non-shipping spike path and is reference-only output pending TASK-003/TASK-005.
- **Downstream:** feeds TASK-003 (implementation-path decision) with the
  Tauri+helper-vs-Swift-first capture/permission signal, and TASK-005 (capture adapter MVP) with
  the validated field allowlist, degraded-state model, and permission flows. Identifies APP-005
  field-allowlist implications for the capture adapter and (indirectly) the privacy/export
  boundary.
- **Guardrails preserved:** local-only raw evidence, no SaaS/cloud sync, no raw activity egress,
  positive collection allowlist, explicit non-collection, no credentials in logs/exports, and the
  legacy/manual-tracker reference-only boundary (no wipe and no reuse decided here — that is
  TASK-003).
- **Branch convention for implementation:** `feat/task-002-macos-capture-feasibility`.
