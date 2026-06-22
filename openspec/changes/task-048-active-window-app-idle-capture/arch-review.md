# Architecture review — TASK-048 zero-permission app + idle capture

- **Reviewer:** SW Architect (SW-1 design review)
- **Change:** `task-048-active-window-app-idle-capture`
- **Verdict:** **pass** — proceed now as a single-boundary implementation task. **Not blocked by
  signing/TCC; no Apple Developer ID signing required** for this slice.
- **Date:** 2026-06-22

## 1. Finding

This is the smallest unblocked slice after storage: the *"zero-permission, locally-stored active-app +
idle timeline (no titles)"* that `task-035-active-window-tracking/design.md` §4 named as shippable on
its own. Its sink — the **Local SQLite Store** with the typed, allowlist-enforcing write/read/prune
API — already exists and is merged (**TASK-046**, `src-tauri/src/active_window/store.rs`). What is
missing is a writer. This slice adds one: a Rust-core capture loop filling the store with the two
signals that need **no macOS permission**.

It sits in **one** component boundary (macOS Evidence Capture → the existing store). It does **not**
cross into permissions UI, suggestion integration, or security-acceptance — those stay downstream. So
the TASK-035 "stop and split" rule is satisfied; this is a single, proportional slice. **Pass.**

## 2. Can it proceed / is it blocked by signing or TCC?

**It proceeds, and it is not blocked.** Three verified facts (primary sources in `design.md` §6):

1. `NSWorkspace.frontmostApplication` (app identity) and `CGEventSource.secondsSinceLastEventType`
   (idle age) require **no TCC grant**. The TCC gates in this space are on `kCGWindowName`
   (Screen Recording) and `CGEventTap` creation (Accessibility / Input Monitoring) — none of which
   this slice uses. Idle is read as event *age*, creating no tap and reading no content.
2. Because **no grant is requested, no TCC entry is created**, so the app's code signature is
   irrelevant to this slice. The app currently ships **unsigned / ad-hoc** (no signing or sandbox
   config in `tauri.conf.json`), and that is sufficient for local use. **No Developer ID needed.**
3. The in-process path (`design.md` §3) adds **no `externalBin`**, so it avoids the documented Tauri
   macOS codesigning/notarization friction entirely.

**One residual assumption, de-risked, not blocking:** TASK-002 compile-validated the idle API but
never *executed* it against live activity, so "idle read shows no prompt on the target Mac" is
mechanism-backed (no tap ⇒ no gate) but not empirically confirmed. `tasks.md` §0 makes a one-time live
confirmation the **first** acceptance step; if a prompt unexpectedly appears, the implementer stops and
reports (the zero-permission premise would be wrong) rather than requesting a grant.

## 3. Build-shape decision (the one design call this review makes)

**Implement this slice in the Rust core (`objc2-app-kit` NSWorkspace + `objc2-core-graphics`
CGEventSource), not as a Swift `externalBin` sidecar.** Full rationale and sources in `design.md` §3.
In short: C1/DEC-019's sidecar requirement exists to place the **AX grant** off the webview and keep
**raw titles** out of the renderer — *neither exists in a zero-permission, no-title slice*, so the
sidecar's only effect here would be to import `externalBin` codesign/notarization risk (tauri#11992 /
#9422 / #12690) into the smallest, lowest-risk increment, against the TASK-048 mandate to avoid
Developer ID signing where possible. The sidecar + `externalBin` + IPC + TCC-trust work belongs to the
**AX-title task**, exactly where TASK-035 §3 wanted the native-packaging risk isolated. Migration cost
is low and non-throwaway (the store-write/coalesce logic stays in the core regardless).

## 4. Routed item — `feedback_to_ba[]` → ba-architect

**FB-048 — refine DEC-019 / constraint C1 scope (ratification, non-blocking).**
DEC-019 mandates capture in a native Swift/AppKit sidecar with the AX grant on the sidecar binary.
This review reads that requirement as **triggered by the Accessibility (title) path**, and proposes to
state explicitly that the **zero-permission app + idle signal may run in the Rust core**, with the
Swift sidecar / `externalBin` / codesign / TCC-trust work introduced by the AX-title task when an AX
grant is first requested.
- **Why routed, not decided unilaterally:** DEC-019 is a BA-ratified decision; an architect refining
  its scope files feedback rather than overwriting it.
- **Why non-blocking:** TASK-048 proceeds under either reading. If BA declines, the fallback is the
  strict-C1 Swift sidecar — more packaging risk, no change to this spec's observable behavior.
- **Carried, still open:** **FB-002** (UX `setup-permissions.html` overstates the permission burden —
  frontmost-app/idle need **no** grant; Accessibility is the one real grant). Unchanged by TASK-048
  (no UI here); still must be reconciled before the downstream permissions-UI task.

## 5. Inherited constraints — how this slice honors them

C1 **scoped** (in-process for zero-permission; sidecar deferred to AX path — §3, FB-048) ·
C2 **NSWorkspace + idle = zero grant**, Quartz/Screen-Recording off · C3 allowlist preserved (writes
only via the typed TASK-046 API; no new column/writer) · C4 `no_gui_session` + `sampling_gap` health
rows · C5 honored by **reading no title at all** (`window_title` always NULL,
`title_state=absent_no_permission`) · C6 retention driver reuses tested `prune_expired`, touches only
`active_window_*` · C7 no egress, locked `connect-src ipc:` CSP unchanged, no network client.

## 6. New dependencies (flagged for SW-2 / SW-5)

`objc2-app-kit` + `objc2-core-graphics` (maintained `madsmtm/objc2` Apple bindings) are the only new
crates — required for in-process native calls. TASK-046 was deliberately zero-dep; this capture task
is where native bindings are intrinsic. SW-5 scans the additions; exact versions recorded in
`tasks.md` §1.

## 7. Exit

- [x] Smallest unblocked single-boundary slice confirmed against live code + the merged TASK-046 sink.
- [x] Verdict **pass** — proceeds now; **not blocked by signing/TCC**; no Developer ID required.
- [x] macOS APIs verified against primary sources; verified-vs-assumption marked with URLs
  (`design.md` §6); the one residual assumption is the first acceptance step (`tasks.md` §0).
- [x] Build-shape decided (in-process Rust, no sidecar/`externalBin`) with rationale + sources.
- [x] FB-048 routed to ba-architect (DEC-019 scope refinement, non-blocking); FB-002 carried.
- [x] OpenSpec proposal / design / tasks / spec written; `handoff.md` updated.
