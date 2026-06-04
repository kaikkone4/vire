# Design — TASK-002 macOS capture feasibility spike

## Context

EPIC-001's central risk is macOS capture reliability and permission cost, not desktop-shell
creation (`04_technical_plan.md` §3). This spike validates the "macOS capture adapter" component
(`04` §4) before TASK-003 picks an implementation path and TASK-005 builds the adapter. It is the
first task to touch macOS capture surfaces; capture has been deferred until now ("Manual Mode",
TASK-001 §2.8), so the field allowlist (SEC-001) is greenfield here.

## Goals / Non-goals

- **Goals:** establish whether NSWorkspace + AX (+ optional Quartz) + idle signals give Janne
  enough evidence quality at acceptable permission cost; produce the field allowlist, degraded-state
  model, permission-burden analysis, manual validation matrix, and the Tauri+helper-vs-Swift-first
  signal for TASK-003.
- **Non-goals:** shipping the capture adapter, writing product evidence rows, deciding the
  implementation path, deciding the fate of the legacy manual-tracker surface, or building the
  Langfuse importer / runtime observer (TASK-006/007).

## Key decisions

### Decision: probe code lives in an isolated, non-shipping spike path

Probe/exploratory code goes under `spikes/task-002-macos-capture/`, outside `src/`,
`src-tauri/src/`, and `observability/`, and is not a member of any shipped build target. This
directly addresses Janne's concern that legacy/manual-tracker files could entangle with current or
future work: spike outputs are physically separated and disposable, and the legacy surface stays
reference-only (no wipe, no reuse) until TASK-003 decides.

- *Why a separate path, not the change dir:* OpenSpec change directories hold documentation;
  executable probe code is clearer and safer in a clearly-named top-level `spikes/` path that can be
  deleted wholesale and is obviously non-product.
- *Why not weave into the Tauri app:* doing so would prejudge the TASK-003 path decision and risk
  shipping spike code; it would also reopen the manual-tracker scope the BA narrowed away from.

### Decision: probes never persist real private titles

Window titles can contain customer names, secrets, or document titles (SEC-001). Probes emit
redacted or synthetic output, or write to ephemeral local logs with a documented cleanup step. No
real titles are committed or persisted durably. This keeps the spike inside the L2 privacy posture
while still validating capture mechanics.

### Decision: Quartz is opt-in, justified by burden

NSWorkspace (frontmost app) and AX (window title where granted) are the primary path. Quartz is
evaluated only as a fallback and recommended only if its title-matching benefit outweighs a possible
Screen Recording permission requirement — the more intrusive permission the BA flagged as a risk
(`05` §6). The spike records the actual permission requirement on the target macOS version rather
than assuming it.

## Open questions routed downstream

- **Frontmost-app permission reality.** The UX prototype surfaces an "App & activity" permission
  row, but NSWorkspace frontmost-app observation may not require a TCC grant. The spike confirms or
  corrects this; the result feeds the TASK-005 permissions UX and SEC-007 transparency copy.
- **Tauri+helper vs Swift-first.** The spike produces the capture/permission-friction signal; the
  decision itself is TASK-003.
- **Idle thresholds.** Concrete `active`/`idle_candidate`/`away` threshold values are directional
  here and finalised with Janne during TASK-005.
