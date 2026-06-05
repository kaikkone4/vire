# Tasks — TASK-003 implementation-path decision

> Decision/spike task. **No product-runtime changes** — no file under `src/`, `src-tauri/src/`, or
> `observability/` is created, modified, or deleted; no build target is added. Deliverables are the
> decision record (ADR DEC-019 in `design.md`), the options-comparison matrix, the verified
> technology register, and the downstream architecture constraints. Implementer:
> **backend-developer (Rust/Tauri)** as primary owner of the host-runtime decision, with
> **macos-capture (Swift/AppKit) developer** consulted on the helper/TCC-trust placement and
> **integration-engineer** consulted on the Langfuse importer host-runtime fit.

## 1. Gather and confirm the spike evidence base
- [x] 1.1 Confirm the three feeder spikes are complete and read their decision-relevant outputs:
      TASK-001 `salvage-reuse-inventory.md` (§2 salvage classifications, §3 data-model coverage,
      §4.3 importer placement), TASK-002 `capture-feasibility-report.md` (§5 permission burden, §10
      Tauri+helper-vs-Swift-first signal), TASK-007 `arch-review.md` (§2 host-runtime split, §4
      SEC-002, §6 TASK-003 note).
- [x] 1.2 Confirm DEC-008/DEC-009 framing and the DEC-017/DEC-018 constraints the decision must
      observe (Langfuse-first, no new pi/Claude adapter, cloud-first import, no raw-activity egress).

## 2. Compare the three BA-mandated options
- [x] 2.1 Define weighted criteria from the spikes (reuse value, SEC-002 importer fit, added
      complexity, privacy boundary, EPIC-006 seams; capture access + permission burden as
      non-differentiating gates per TASK-002 §10).
- [x] 2.2 Score Option A (Tauri reuse + native helper + Rust importer), Option B (Swift/AppKit-first),
      and Option C (ActivityWatch import/reference) against the criteria.
- [x] 2.3 Identify the decision driver (reuse value vs. rebuild cost, since capture does not favour
      either native path) and test it against the DEC-009 switch condition.

## 3. Verify every named technology (research mandate)
- [x] 3.1 Verify Tauri v2 sidecar/`externalBin` (bundle + spawn a native helper) against official docs.
- [x] 3.2 Verify Tauri v2 IPC (webview↔core message passing; renderer has no direct network/process
      access) against official docs.
- [x] 3.3 Verify Tauri v2 HTTP client (Rust `reqwest` re-export, URL-allowlist scoping) against
      official docs for SEC-002 importer fit.
- [x] 3.4 Confirm the macOS TCC/Accessibility trust model (grant attaches to the signed binary that
      calls AX) from Apple docs (carried from TASK-002/DEC-002).
- [x] 3.5 Record the macOS **nested-binary codesign + notarization** point as an **assumption** with a
      TASK-005 packaging-spike follow-up (docs do not explicitly confirm auto-signing of sidecars).
- [x] 3.6 Cite Swift/AppKit, ActivityWatch, and SQLite source URLs; mark each verified or
      reference-only. Record all in the `design.md` technology verification register.

## 4. Record the decision and constraints
- [x] 4.1 Write ADR **DEC-019** (decision, rationale traced to TASK-001/002/007 and DEC-008/009/017/
      018, consequences/accepted costs).
- [x] 4.2 Fix the binding downstream constraints: helper holds the AX grant; helper↔core via IPC;
      raw titles never reach the webview/network; Langfuse importer is a read-only Rust-core REST
      client behind the locked CSP + URL allowlist; reuse is evidence-driven and excludes the legacy
      CRUD capture surface.
- [x] 4.3 Route DEC-019 to the BA decision log via `feedback_to_ba[]` (ratify the SW-side
      confirmation of DEC-009).

## 5. Deliverable and exit gate
- [x] 5.1 Confirm the exit gate: **implementation path selected and documented before the TASK-004
      schema/feature build starts** (`05` §5 Gate A).
- [x] 5.2 Confirm no product-runtime change, no schema build, no capture/importer MVP, and no legacy
      wipe/reuse-as-capture decided here (those are TASK-004/005/007-MVP).
- [x] 5.3 Re-read all produced artifacts; confirm no credentials, no real window/app titles, no
      prompt/response text, no command bodies, and no environment dumps appear.
- [x] 5.4 `openspec validate task-003-implementation-path-decision --strict` → **valid** (run from
      the `code/` project root, 2026-06-05).
