# Documentation Review — TASK-002 macOS capture feasibility spike

- **Change:** `task-002-macos-capture-feasibility`
- **Branch:** `feat/task-002-macos-capture-feasibility`
- **PR:** https://github.com/kaikkone4/vire/pull/7
- **Tier:** L2 · **Gate:** SW-6 (Documentation)
- **Reviewer role:** Documentation Engineer (SW-6)
- **Date:** 2026-06-04
- **Verdict:** **No drift — no README/docs/runbook changes required**

> Scope note: this is a feasibility *spike*. It ships nothing, changes no product runtime,
> and decides no implementation path (TASK-003 owns that). The deliverables are the
> capture-feasibility report, permission-burden analysis, field-allowlist (APP-005/SEC-001)
> implication map, and manual validation matrix — all under the OpenSpec change dir — plus one
> isolated, non-shipping, compile-validated Swift probe. SW-6 here verifies documentation
> drift against shipped state and downstream handoff sufficiency; it does not document a
> released feature (none was released).

---

## 1. What was reviewed

| Surface | State |
| --- | --- |
| `README.md` | Describes Vire **v0.1** (local-only manual time tracker; automatic capture intentionally deferred; "Privacy status" affirms no window/screenshot/keystroke capture). |
| `docs/` | Does not exist. Not required at L2 for a spike that ships nothing. |
| `RELEASE.md` / `CHANGELOG.md` | Do not exist. No release is cut by this spike. |
| Spike probe docs | `spikes/task-002-macos-capture/README.md` + `probe/main.swift:1-21` header. |
| Spike deliverables | `openspec/changes/task-002-macos-capture-feasibility/` (report, proposal, design, spec, tasks). |

Upstream gates all **PASS**: QA (`qa.md`), Code Review (`review.md`), Security (`sec.md`).

---

## 2. Drift assessment against the three SW-6 questions

### Q1 — Does the probe require documentation? **No — already adequately documented.**

The probe is self-documented to a standard SW-4 explicitly verified:

- `spikes/task-002-macos-capture/README.md` covers: purpose and non-shipping status, the four
  signals exercised (NSWorkspace / AXUIElement / Quartz / CGEventSource), isolation guarantees,
  the SEC-001 privacy warning (redaction-by-default), build/run/cleanup steps, the compile-only
  (`swiftc -typecheck`) check, and the **delete-after-TASK-003** disposal instruction.
- `probe/main.swift:1-21` repeats the non-shipping/disposable header at the code boundary.
- The feasibility report links the probe as supporting evidence (§2, §13).

No additional probe documentation is warranted; adding more would duplicate existing content.

### Q2 — UX permission-burden finding: README now, or downstream docs only? **Downstream only — do NOT add to README now.**

The finding (NSWorkspace frontmost-app needs **no TCC grant**; **Accessibility required** for
window titles; **Quartz/Screen Recording not recommended** on the default path) is correctly
**captured downstream and must not enter the v0.1 README**, for three reasons:

1. **Doc-version alignment.** The README documents the **shipped product (v0.1)**, in which
   automatic capture is *intentionally deferred*. Documentation must describe the released
   state, not bleeding-edge spike findings about a future adapter (TASK-005). Adding capture
   permission detail now would describe behaviour that does not exist in the shipped app.
2. **Privacy-promise integrity.** README "Privacy status" affirms v0.1 does **not** capture
   active windows, screenshots, keystrokes, etc. Injecting capture-permission guidance now
   would contradict that promise and risk implying capture is present or imminent.
3. **It is already captured where it belongs.** The finding is fully documented in the
   capture-feasibility report — §5 (permission-burden table), §3.1–§3.4 (signal-by-signal),
   §8/FB-002 (UX "App & activity" row overstates burden) — and routed downstream to TASK-005
   (adapter), TASK-003 (path), and BA/UX (FB-002 permissions-screen reconciliation). SEC-007
   transparency is preserved via that routing (`sec.md` §3).

**Action for a future release:** when TASK-005 ships the capture adapter, the README "Privacy
status" and a future installation/permissions guide must be updated to state the real burden
(Accessibility required; Screen Recording not requested) and the field allowlist. That is a
**TASK-005 release-docs obligation**, not a TASK-002 one. Recorded here so it is not lost.

### Q3 — Are TASK-003 / TASK-005 handoff docs sufficient? **Yes — concrete and traceable.**

- **TASK-003 (implementation-path decision):** report §10 gives the Tauri+helper-vs-Swift-first
  comparison table and observed-friction signal, with the decision **explicitly deferred**; §12
  flags helper TCC-trust placement as the open question TASK-003 must resolve. Code Review §4
  independently confirmed no premature path decision.
- **TASK-005 (capture adapter MVP):** report §7.4 gives the field-allowlist enforcement
  contract, the highest-risk-field (`window_title`) handling rule, the degraded-state/capture-
  health model (§4), and the SEC-007 transparency requirement; §12 routes idle-threshold and
  Electron/browser-title finalisation to TASK-005. Code Review §6 adds three probe-vs-adapter
  refinement notes (capture-health taxonomy breadth, Quartz owner-pid filtering, `redact()`
  lower bound) for the TASK-005 implementer to carry forward.
- **Cross-references:** report §13 and the OpenSpec proposal/design cite BA `04_technical_plan.md`
  and `05_project_plan_epics.md` sections and APP-005/SEC-001/SEC-007 concretely; QA and Code
  Review both verified traceability with no checkbox unsupported.

Handoff documentation is sufficient; no gap to fill at SW-6.

---

## 3. Conclusion

No documentation drift exists against the shipped state, and no README, `docs/`, or runbook
changes are appropriate for this spike:

- The probe is already documented (Q1).
- The permission-burden finding belongs downstream and is already there; adding it to the v0.1
  README would break doc-version alignment and contradict the shipped privacy promise (Q2). It
  is recorded above as a **TASK-005 release-docs obligation** for when capture actually ships.
- TASK-003 / TASK-005 handoff docs are concrete and traceable (Q3).

The only documentation this spike should produce is the OpenSpec change deliverables (already
present) and this SW-6 record. No PR/doc content changes were made beyond adding this report.

**DOCS STATUS: complete.**
