# Architecture Review — TASK-002 macOS capture feasibility spike

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-002-macos-capture-feasibility`
- **Branch (implementation):** `feat/task-002-macos-capture-feasibility`
- **Tier:** L2 · **Gate context:** APP-005 (SEC-001 primary, SEC-007; SEC-002/003 only if a probe
  touches tooling/logging/network)
- **Date:** 2026-06-04
- **Verdict:** **PASS** — scope is a single, component-aligned capture-feasibility spike; no split
  required; no BA escalation. Janne's legacy-isolation concern is encoded as enforceable spec
  guardrails (no `escalate-to-ba` needed). One non-blocking `feedback_to_ba[]` note is recorded.

## 1. Scope validation against BA architecture

TASK-002 is a **spike** (`05_project_plan_epics.md` §3 row TASK-002; `04_technical_plan.md` §5
spike plan; `04` §13 WP-002). Its mandate is to validate macOS capture feasibility and permission
burden, with the exit gate *"capture feasibility and permission burden documented; APP-005 field
allowlist implications identified."*

- **Component-aligned, single boundary.** All spike work targets exactly one BA component — the
  **macOS capture adapter** (`04` §4: inputs NSWorkspace/NSRunningApplication, AXUIElement, optional
  Quartz, idle signal; outputs the `raw_evidence` / `normalized_evidence` / `capture_health` shape).
  It crosses no other component boundary (no importer, store, classification, review, or export
  work), so it is **not splittable on boundary grounds**. The single OpenSpec change is the right
  unit.
- **Sequencing is correct.** TASK-001 (repo assessment) → TASK-002 capture spike → TASK-003 path
  decision (`05` §4 dependency chain). TASK-002 must *feed* the Tauri+helper-vs-Swift-first signal
  into TASK-003 (`04` §5 acceptance criteria) but must **not make** that decision — the spec encodes
  that deferral.
- **No scope creep / no premature MVP.** This is a feasibility assessment, not the capture adapter
  (TASK-005). The spec forbids shipping the adapter, writing product evidence rows, and deciding the
  path. This matches the BA "spike-first, evidence before commitment" posture (`05` §1).
- **Risk coverage.** The spike directly retires the two EPIC-001 capture risks in `05` §6:
  "macOS title capture is inconsistent" (AX validation + visible degraded states) and "Quartz
  requires intrusive Screen Recording permission" (opt-in, burden-justified Quartz).

Conclusion: the task design is consistent with `04/05` and APP-005. **PASS.**

## 2. Janne's concern — legacy isolation encoded as architecture guardrails

Janne's stated worry: old manual-tracker files may confuse the current implementation or get mixed
into future work. The instruction is **not** to decide a wipe in TASK-002, but to encode a guardrail
that legacy/manual-tracker code is reference-only unless explicitly selected later by TASK-003, and
to keep spike outputs from entangling with legacy/product runtime.

This is handled at the spec level, not as an escalation, because it is a containment guardrail
TASK-002 can fully own:

1. **Legacy stays reference-only (no wipe, no reuse).** TASK-001 §2.2/§2.4 already classified the
   manual-tracker surface (`time_entries` table, `renderManual`/`bindEntries` view, stopwatch CRUD)
   as **reference-only**, with the migrate-vs-retire decision routed to TASK-003/TASK-004. TASK-002's
   spec (*Requirement: Spike outputs are isolated from product and legacy code* → *Legacy/
   manual-tracker code stays reference-only*) preserves exactly that: the spike must not import,
   migrate, reuse, or wipe it. **The wipe decision is explicitly NOT taken here.**
2. **Spike code is physically isolated and disposable.** Probe code is confined to
   `spikes/task-002-macos-capture/`, outside `src/`, `src-tauri/src/`, and `observability/`, and not
   a member of any shipped build target. It cannot be woven into product runtime, so it cannot
   entangle with — or be mistaken for — either legacy or product code, and it can be deleted wholesale
   without touching the app.
3. **Anti-momentum note for TASK-003.** The single largest design risk is reuse momentum:
   re-importing the manual-tracker "generic time tracker" surface the BA deliberately narrowed away
   from (`01`/`04` §1). The capture adapter is an **evidence source** (raw → normalized →
   `capture_health`), not a manual stopwatch. TASK-003 must keep any reuse evidence-driven and must
   not let the legacy CRUD surface re-enter scope as a capture path.

## 3. APP-005 / security posture (spike scope)

| Control | Spike-scope handling | Downstream (TASK-005/012/014) |
| --- | --- | --- |
| **SEC-001** capture allowlist | Greenfield here. Spec mandates the **positive allowlist** (app bundle/name, focused window title where permitted, coarse timestamps, permission/degraded state, idle/away) and the **explicit non-collection** list; mapped to the six-field UX record. | Enforced field-level in the adapter; adversarial-title tests prove prohibited fields are never persisted. |
| **SEC-007** permission transparency | Spike documents required vs optional permissions and degraded behaviour; confirms/corrects the UX "App & activity" permission row. | In-app permissions page explains each permission, what is/ isn't collected, and current degraded state. |
| **SEC-002** network boundary | **N/A by default** — no network client; capture is local-only. Only fires if a probe benchmarks ActivityWatch or touches tooling that egresses. Spec forbids raw activity egress. | Importer-side concern (TASK-007/012), not capture. |
| **SEC-003** credentials | **N/A by default** — capture collects no credentials. Only relevant if a probe reads tooling config/logs. | Importer-side concern. |
| Probe data safety | Spec mandates **no real private titles persisted** — redacted/synthetic output or ephemeral local logs with documented cleanup. This keeps the spike inside L2 privacy while validating mechanics. | Carries into adapter test fixtures (synthetic titles only). |

Gate D (APP-005 release gate) does **not** fire: the spike ships nothing and writes no product
evidence. Coverage is recorded so the spike is control-aware and the TASK-005 adapter inherits a
ready allowlist.

## 4. Field-allowlist implications identified (exit-gate item)

The spike must hand TASK-005 a concrete allowlist. Architect's expected shape, aligned to the UX
six-field evidence record (`ux/prototype/setup-permissions.html`) and SEC-001:

- **Collected (positive allowlist):** `app_name` (bundle display name), `window_title` (title-bar
  string, AX-permitted only), `time_range` (coarse start/end), `day`, `source`
  (`nsworkspace`/`accessibility`/`quartz`/`runtime`), permission/degraded state, idle/away state.
- **Never collected:** screenshots, keystrokes, screen pixels, full browser contents, full URLs,
  terminal command bodies, shell history, prompt/response text, environment dumps, secrets.
- **Implication for TASK-005:** the allowlist is a *field-level* enforcement contract, not a
  guideline — the window title is the single highest-risk field (free-form, can contain customer
  names/secrets), so degraded/unavailable/redacted handling for it must be first-class, and raw
  titles must never reach logs or export by default.

## 5. Design-level note → `feedback_to_ba[]` (non-blocking)

**FB-002 — UX "App & activity" permission row vs NSWorkspace TCC reality.**
- **Observation:** `setup-permissions.html` presents an "App & activity" macOS permission row
  (alongside Accessibility) governing the NSWorkspace frontmost-app signal. On macOS, frontmost-app
  observation via NSWorkspace generally does **not** require a TCC permission grant, whereas the
  window-title (AX) and optional Quartz (Screen Recording) paths do. The spike will confirm the real
  requirement.
- **Impact:** purely a transparency/UX accuracy question — if the row implies a permission that
  isn't actually requested, the SEC-007 permissions page and onboarding copy could misstate the
  permission burden (over- or under-stating it).
- **Recommendation (non-blocking):** treat the spike's permission-burden finding as the source of
  truth and have UX/BA reconcile the permissions screen to it before TASK-005 builds the permissions
  flow. No change to TASK-002 scope.
- **Disposition:** recorded as a downstream decision input; does **not** block TASK-002. This does
  not warrant `escalate-to-ba` — surfacing exactly this kind of capture/permission detail is the
  spike's purpose.

## 6. Other architectural notes for downstream

- **TASK-003:** consume the Tauri+helper-vs-Swift-first signal; weigh permission friction and helper
  packaging cost (`04` §3 decision table). Decide the `time_entries`/manual-tracker fate together
  with TASK-004 — keep it evidence-driven, do not reuse by momentum.
- **TASK-005:** the capture adapter is the *only* component that should grow from this spike; keep it
  behind the helper/adapter boundary so the Tauri webview CSP stays locked (TASK-001 §4.3) and raw
  titles never cross into the webview or network path.
- **Helper boundary, not webview.** Any native capture (Swift/AppKit helper or native module) stays
  out of the renderer; the webview never receives raw titles, consistent with the locked
  `connect-src ipc:` CSP.
- **Spike disposal.** `spikes/task-002-macos-capture/` should be deleted or archived once TASK-003
  consumes its findings, so no probe code lingers near product runtime.

## 7. Handoff

- **SW-2 implementer (primary):** **macOS capture / native (Swift + AppKit) developer** — owns the
  NSWorkspace/NSRunningApplication, AXUIElement, Quartz, and CGEventSource idle probes and the
  feasibility report.
- **Consulted:** **backend-developer** (Rust/Tauri) for the Tauri+helper-boundary friction signal
  that feeds TASK-003; **security/QA** for the SEC-001 collection allowlist and probe data-handling
  review.
- **Mandatory QA / security checks for this spike's deliverables:**
  1. **Probe data-safety review:** confirm no real private window/app titles, prompt/response text,
     command bodies, secrets, or environment dumps are persisted or committed; probe output is
     redacted/synthetic or ephemeral-with-cleanup (SEC-001).
  2. **Isolation check:** confirm `spikes/task-002-macos-capture/` is **not** referenced by any
     shipped build target and that no file under `src/`, `src-tauri/src/`, or `observability/` was
     modified; confirm the legacy manual-tracker surface was not imported, migrated, reused, or wiped.
  3. **Allowlist completeness:** confirm the positive allowlist and explicit non-collection list are
     produced and the APP-005/SEC-001 implications for TASK-005 are identified (exit-gate item).
  4. **Manual validation matrix present:** confirm the matrix covers permission grant/revoke,
     degraded states, sleep/wake, Spaces/full-screen, core-app title availability, idle thresholds,
     and Quartz permission burden.
  5. **Exit-gate text:** feasibility + permission burden documented; Tauri+helper-vs-Swift-first
     signal produced; no capture MVP shipped and no path decided (TASK-003 owns it).
  6. **Secret-scan** any committed spike artifacts (filenames/counts only; no values printed).
- **OpenSpec status:** `openspec validate task-002-macos-capture-feasibility --strict` → valid.
