# Tasks — TASK-002 macOS capture feasibility spike

> Spike/assessment. No product-runtime changes. Deliverables are the capture-feasibility report,
> permission-burden analysis, field-allowlist (APP-005/SEC-001) implication map, and manual
> validation matrix. Any probe code is isolated under `spikes/task-002-macos-capture/` (non-shipping,
> redacted/synthetic output). Implementer: **macos-capture (native macOS / Swift+AppKit) developer**
> as primary, with **backend-developer** consulted on the Tauri/Rust helper-boundary signal and
> **security/QA** on the collection allowlist and probe data-handling.

## 1. Spike isolation and safety scaffold
- [x] 1.1 Establish the isolated spike path `spikes/task-002-macos-capture/` and confirm it is **not**
      a member of any shipped build target (not added to the Tauri app, not referenced by
      `src-tauri/src/`, not under `observability/`).
- [x] 1.2 Define the probe data-handling rule: probes emit **redacted or synthetic** titles, or write
      to an **ephemeral local log** with a documented cleanup step; no real private window titles are
      persisted or committed.
- [x] 1.3 Confirm the legacy/manual-tracker surface (`time_entries`, manual-entry view, stopwatch CRUD)
      stays **reference-only** and is not imported, migrated, or reused by spike work (TASK-001 §2.2/§2.4).

## 2. NSWorkspace / NSRunningApplication active-app capture
- [x] 2.1 Capture frontmost app bundle id/name + timestamp at a small sampling interval.
- [x] 2.2 Validate app-switch detection, app launch/terminate, system sleep/wake, multiple Spaces,
      and full-screen apps.
- [x] 2.3 Record whether the frontmost-app signal requires any TCC permission; reconcile against the
      UX prototype's "App & activity" permission row (confirm or correct it).

## 3. AXUIElement focused window / title
- [x] 3.1 Request/observe Accessibility (AX) permission state.
- [x] 3.2 Capture the focused window title where AX is granted, for Janne's core apps.
- [x] 3.3 Record unavailable / permission-denied / redacted states explicitly as capture-health-shaped data.

## 4. Quartz fallback and permission burden
- [x] 4.1 Test whether Quartz Window Services improves title/window matching when AX data is missing.
- [x] 4.2 Determine whether the selected Quartz calls require **Screen Recording** permission on
      Janne's macOS version.
- [x] 4.3 Recommend Quartz only if the title-matching benefit exceeds the added permission burden;
      record the recommendation and its rationale.

## 5. Idle / away signal
- [x] 5.1 Validate a CGEventSource last-event-age (or equivalent local) idle signal.
- [x] 5.2 Define configurable thresholds and conversion to `active` / `idle_candidate` / `away` states.

## 6. Degraded states, health, and ActivityWatch reference
- [x] 6.1 Enumerate degraded/health states: permission state, sampling gaps, app/window unavailable,
      sleep/wake, helper crash/restart.
- [x] 6.2 Benchmark ActivityWatch as a **reference only** (evidence quality, permission model);
      do not adopt.

## 7. Field allowlist and APP-005 implications
- [x] 7.1 Produce the **positive** field allowlist: app bundle/name, focused window title (where
      permitted), coarse timestamps, permission/degraded state, idle/away state.
- [x] 7.2 Produce the **explicit non-collection** list: no screenshots, keystrokes, screen pixels,
      full browser contents, full URLs, terminal command bodies, shell history, prompt/response text,
      environment dumps, or secrets.
- [x] 7.3 Map the allowlist to the six-field UX evidence record and record APP-005/SEC-001 (and
      downstream SEC-007 transparency) implications for the TASK-005 capture adapter.

## 8. Manual validation matrix
- [x] 8.1 Produce a manual validation matrix covering: permission grant/revoke, degraded states,
      sleep/wake, Spaces/full-screen, core-app title availability, idle thresholds, and Quartz
      permission burden — each with expected observable behaviour.

## 9. Deliverable and exit gate
- [x] 9.1 Produce the capture-feasibility report: NSWorkspace/AX/Quartz/idle findings, the
      Tauri+helper-vs-Swift-first signal for TASK-003, and the permission-burden conclusion.
- [x] 9.2 Confirm the exit gate: **capture feasibility and permission burden documented; APP-005
      field-allowlist implications identified.** No capture MVP shipped; no path decided (TASK-003 owns it).
- [x] 9.3 Verify all produced artifacts by re-reading them; confirm no real window/app titles,
      prompt/response text, command bodies, secrets, or environment dumps appear.
