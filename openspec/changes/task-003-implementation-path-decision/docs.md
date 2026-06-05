# Documentation Gate (SW-6) — TASK-003 implementation-path decision

- **Role:** Documentation Engineer (SW-6)
- **Change:** `task-003-implementation-path-decision`
- **Branch:** `feat/task-003-implementation-path-decision` · **PR:** #9
- **Tier:** L2 · **Gate:** SW-6 release documentation (docs match released state)
- **Date:** 2026-06-05
- **Verdict:** **complete** — no README/docs update required; one non-blocking merge-ordering condition carried forward

## 0. Method

This is a **docs/OpenSpec/ADR decision package** — no product runtime. The documentation gate
therefore asks a single question: does DEC-019 change anything a reader-facing document
(README, installation/user/onboarding guide, API spec, decision-log index) currently asserts? If
yes, those docs would be **drifted** and must be updated to match the released state. If no, the
decision is already self-documented inside the OpenSpec change package and the gate is **complete**.

Assessed mechanically against the repo working tree and the change diff. QA (SW-3), Code Review
(SW-4), and Security (SW-5) have all passed; this gate does not re-adjudicate them.

## 1. Documentation surface inventory

| Surface | Present? | Affected by DEC-019? |
| --- | --- | --- |
| `README.md` (repo root) | Yes | **No** — see §2 |
| `docs/` directory | **No** (does not exist) | N/A |
| OpenAPI / Swagger spec | **No** (no HTTP API surface — local-only Tauri app, `connect-src ipc:`) | N/A |
| `CHANGELOG` / `RELEASE.md` | **No** | N/A |
| ADR index / SW decision-log file | **No** (decisions live in OpenSpec change packages; BA log is `07_decision_log.md`, a BA artifact, updated via `feedback_to_ba[]`) | N/A — see §3 |
| Inline code comments | N/A | **No** — zero source files touched (`git diff main...HEAD -- src/ src-tauri/src/ observability/` empty) |
| OpenSpec change package (proposal/design/spec/tasks/arch-review) | Yes | **Self-documenting** — the ADR *is* the deliverable |

## 2. README is accurate for the released state — no update required

`README.md` describes **Vire v0.1**: a local-only manual time tracker (projects, manual entries,
summaries, CSV export) and states plainly that *"Automatic activity capture is intentionally
deferred"* and the capture status reads *"Manual Mode / Capture deferred"*.

DEC-019 is a **forward-looking decision/spike** — it selects the *future* implementation path
(reuse Tauri shell + native Swift/AppKit capture helper + Rust-core Langfuse importer) but builds
**none** of it. The schema (TASK-004), capture adapter (TASK-005), durable importer (TASK-007 MVP),
and runtime observer (TASK-006) all remain unbuilt; the legacy manual surface is left
reference-only. Therefore:

- Every README claim (manual-only, capture deferred, local SQLite, no network/cloud/accounts, no
  window/idle/screenshot/keystroke capture) **remains true** of the current code. There is **no
  drift**.
- Per the role's versioning rule — *docs describe the released state, not the bleeding edge* —
  the README **must not** be rewritten to advertise the planned capture/importer architecture. Doing
  so would describe unbuilt behavior and misstate the actual shipped product. Updating it is not
  merely unnecessary; it would be incorrect.

The README's "Run locally / Build / Tests / Manual verification / Privacy status" sections all
describe the existing v0.1 surface, which this change does not alter.

## 3. The decision is already documented in the right place

DEC-019 is recorded as an **ADR in `design.md`** (full statement, rationale traced to TASK-001/002/007
and DEC-008/009/017/018, binding downstream constraints, consequences, technology-verification
register) and routed to the **BA decision log** via `feedback_to_ba[]` (FB-003) for ratification —
SW does not edit BA artifacts (`07_decision_log.md`) directly. The repo has **no separate SW-side
ADR index or decision-log file** that would need a pointer; OpenSpec change packages are the system
of record for SW decisions here. Nothing to update.

## 4. Carried-forward condition (non-blocking, from SW-4 §5)

**Merge ordering:** TASK-003's cross-references to TASK-001/002/007 resolve **only on the feeder
branches**; those three feeder PRs are not yet on `main`. PR #9 must merge **after (or in the same
train as)** the TASK-001, TASK-002, and TASK-007 feeder PRs so the spike citations resolve in `main`.
If PR #9 lands first, the links dangle on `main` until the feeders catch up.

- This is a **release-sequencing** note for the Release Manager, **not** a documentation defect and
  **not** a rework item. The references are correct (accurate paths + section anchors); they simply
  depend on the feeders being present.
- **Documentation impact:** none on the released-state docs (README et al.). This condition governs
  *merge order*, not doc content.

## 5. Verdict

**complete.** No README, `docs/`, OpenAPI, CHANGELOG/RELEASE, or ADR-index update is required: this
decision-only spike changes no product runtime, and the README accurately describes the current v0.1
released state (updating it to describe the unbuilt capture path would be wrong, not helpful). The
decision is fully documented as ADR DEC-019 inside the OpenSpec change package and routed to the BA
log via FB-003.

**Carry to Release Manager:** the non-blocking SW-4 merge-ordering condition — PR #9 merges after/with
the TASK-001/002/007 feeder PRs so cross-references resolve on `main`.
