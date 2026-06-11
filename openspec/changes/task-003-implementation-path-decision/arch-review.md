# Architecture Review — TASK-003 implementation-path decision

> **⚠ SUPERSESSION NOTE (DEC-020 / DEC-022 · TASK-018, 2026-06-11).** Reviewed 2026-06-05 under
> **DEC-018 (cloud-first Langfuse source)**, now **superseded by DEC-020**: the canonical/default
> Langfuse source is **local Docker self-hosted Langfuse** (`http://127.0.0.1:3000`); Cloud is an
> **explicit non-default override only**. The DEC-019 decision and every boundary affirmed here
> (read-only Rust-core REST importer, locked CSP, URL allowlist, no raw-activity egress, AX grant on
> the helper, no new pi/Claude adapter) **remain valid**; only the importer's *default source posture*
> moves cloud → loopback. Read every "cloud-first" / "DEC-018" mention below as
> "local-Docker-default per DEC-020; Cloud explicit override only". Authoritative realignment:
> `openspec/changes/task-018-local-langfuse-source-addendum/arch-review.md`.

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-003-implementation-path-decision`
- **Branch (implementation):** `feat/task-003-implementation-path-decision`
- **Tier:** L2 · **Gate context:** APP-005 Gate A (validation/decision gate). SEC-002/SEC-003 only
  as **forward constraints** the decision fixes for the importer; no network/credential surface is
  built here.
- **Date:** 2026-06-05
- **Verdict:** **PASS** — TASK-003 is a single, component-aligned **decision gate** that the BA flow
  explicitly assigns to this task (`05` §4; DEC-008/DEC-009). It crosses no implementation boundary
  (it *sets* the boundaries downstream tasks build on), needs no split, and stays inside the BA's own
  framing, so no `escalate-to-ba`. One non-blocking `feedback_to_ba[]` entry (FB-003) ratifies the
  decision into the BA decision log as **DEC-019**.

## 1. Scope validation against BA architecture

TASK-003's mandate (`05_project_plan_epics.md` §3 row TASK-003; §4 chain; §5 Gate A;
`04_technical_plan.md` §3 path-decision table) is to **record an implementation path before the
TASK-004 schema/feature build starts**. The three feeder spikes are complete and were read in full
for this review:

- **TASK-001** classified the Tauri shell, CSP, capabilities, SQLite layer, and CSV
  escaping/formula-neutralization as salvage assets (several `reuse-as-is`), left both reuse and
  replace open, and recommended (§4.3) the Langfuse importer run server-side in the Rust backend.
- **TASK-002 §10** produced the deciding signal: capture APIs are **equally reachable** from a
  native helper or a Swift-first app — capture feasibility favours neither — so the trade is helper
  packaging/IPC/TCC-trust vs. rebuild cost of review/store/export.
- **TASK-007 arch-review §2/§6** assigned the importer **host-runtime** choice to TASK-003 and
  fixed the importer as a read-only, cloud-first REST pull (DEC-018) behind SEC-002.

- **Component-aligned, single decision unit.** TASK-003 produces a decision record, not an
  implementation. It does not build any BA component — not the schema (TASK-004), capture adapter
  (TASK-005), durable importer (TASK-007 MVP), or runtime observer (TASK-006). It is the one task
  whose explicit job is to choose across components, so "deciding the path for all of them" is its
  mandate, **not** scope creep. The single OpenSpec change is the correct unit; **no split**.
- **Sequencing is correct.** We are at the TASK-001/002/007 → **TASK-003** → TASK-004 gate. The
  decision is recorded here so TASK-004's migrations build on a fixed host and evidence boundary.
- **DEC-017/DEC-018 preserved.** The chosen importer is a read-only Langfuse pull with no new
  pi/Claude adapter and no raw-activity egress; cloud-first per DEC-018. The decision does not
  reopen the adapter question or widen the network boundary.

Conclusion: the task design is consistent with `03/04/05`, DEC-008/009/017/018, and APP-005 Gate A.
**PASS.**

## 2. The decision (ADR DEC-019) and why it is architecturally sound

**DEC-019 — Reuse the Tauri v2 shell + native Swift/AppKit capture helper (sidecar) + Rust-backend
Langfuse REST importer. ActivityWatch reference-only; Swift/AppKit-first rejected.**

- **DEC-009's switch condition is not met.** DEC-009 says switch to Swift-first *only if
  capture/permission/helper integration proves simpler than reuse*. TASK-002 §10 shows integration
  is **not** simpler (capture is equal; the only Tauri-specific cost is helper packaging/TCC-trust),
  while reuse value is high (CSP-locked privacy boundary + adversarially-tested CSV safety + a
  server-side SEC-002 boundary that fits the importer). Replacing all of that buys **no capture
  gain**. Reuse therefore stands — exactly the DEC-009 default.
- **The decision driver is correctly identified.** Because the two native options tie on capture,
  the differentiator is reuse value vs. rebuild cost, not capture mechanics. The decision rests on
  that axis, which is the architecturally honest one.
- **Privacy boundary is strengthened, not weakened.** Capture stays in the helper; raw titles flow
  helper → Rust core → normalized/redacted and never reach the renderer (the `connect-src ipc:` CSP
  stays locked) or the network. The importer is the single allowed outbound path, scoped by the
  Tauri HTTP URL allowlist (verified, §3).
- **Accepted cost is named, not hidden.** Nested-binary codesign + notarization of the sidecar is
  the one real friction, and it is recorded as an **assumption with a TASK-005 packaging-spike
  follow-up** rather than overclaimed — the correct posture under the research mandate.

## 3. Research mandate — technology verification (sources read 2026-06-05)

Every decision-load-bearing technology was verified against official docs, or marked as an
assumption with a follow-up. Full register is in `design.md`; the load-bearing confirmations:

- **Tauri v2 sidecar (`externalBin`) — verified.** https://v2.tauri.app/develop/sidecar/ confirms
  bundling + spawning a native helper (per-target-triple naming `*-aarch64-apple-darwin`, spawn via
  the shell plugin, stdin/stdout `CommandEvent`s). This is the mechanism the helper path depends on.
- **Tauri v2 IPC — verified.** https://v2.tauri.app/concept/inter-process-communication/ confirms the
  webview reaches the core **only** via async message-passing IPC — it has no direct network/process
  access. This is what keeps raw titles out of the renderer.
- **Tauri v2 HTTP client — verified.** https://v2.tauri.app/plugin/http-client/ confirms a Rust
  `reqwest` re-export with URL allow/deny scoping — a precise fit for the SEC-002 Langfuse-only
  outbound allowlist.
- **macOS TCC/Accessibility trust — verified** (carried from TASK-002/DEC-002): the AX grant attaches
  to the signed binary that calls AX, so the helper must be the TCC-trusted binary.
- **Nested-binary codesign + notarization — assumption + TASK-005 follow-up.**
  https://v2.tauri.app/distribute/sign/macos/ confirms app-bundle signing/notarization but does
  **not** explicitly state sidecars are auto-signed; TASK-005 must prove the signed helper holds AX
  and passes Gatekeeper. This is the **only** signal that would reopen DEC-019 toward Option B.
- **Swift/AppKit, ActivityWatch, SQLite — verified / reference-only** with source URLs in the
  register.

No unverified claim is presented as fact; the single gap is explicitly flagged. Mandate satisfied.

## 4. APP-005 / security posture (decision scope)

| Control | Decision-scope handling | Downstream owner |
| --- | --- | --- |
| **SEC-001** capture allowlist | Not built here; the decision **binds** capture to the helper so the TASK-002 field allowlist is enforced off-renderer. | TASK-005 |
| **SEC-002** network boundary | Not built here; the decision **fixes** the importer as a read-only Rust-core REST pull behind the locked CSP + URL allowlist (Langfuse-only, no raw-activity egress). | TASK-007 MVP / TASK-012 |
| **SEC-003** credentials | Not built here; the decision keeps credential handling server-side in the core (never the renderer, never logs/exports). | TASK-007 MVP / TASK-012 |
| Decision data safety | The change is a decision record; it persists no evidence, credentials, or titles. | n/a |

Gate D does **not** fire: the decision ships nothing durable and writes no product evidence. Gate A
(validation/decision gate) is the relevant gate, and this change satisfies its "implementation path
selected and documented" exit item (`05` §5 Gate A).

## 5. Design-level note → `feedback_to_ba[]` (non-blocking)

**FB-003 — ratify DEC-019 into the BA decision log.**
- **Observation:** DEC-008/DEC-009 framed the path choice and deferred it to this implementation
  spike. The spike evidence now resolves it in favour of **reuse + native helper + Rust importer**.
  This is a confirmation/operationalization of DEC-009, not a divergence from it.
- **Recommendation (non-blocking):** record the decision as **DEC-019** in
  `artifacts/ba/07_decision_log.md` (SW edits to BA artifacts go through BA, hence this entry rather
  than a direct edit), tracing DEC-001/004/007/008/009/017/018 and EPIC-001/EPIC-006.
- **Disposition:** does **not** block TASK-003 or downstream work; the decision is binding for SW
  regardless of ratification timing. No `escalate-to-ba` — choosing the path is exactly this task's
  purpose.

## 6. Other architectural notes for downstream

- **TASK-004:** build migrations on the reused Rust-core host; take TASK-007's proposed
  `langfuse_import_runs` / normalized-AI-evidence shapes as input; **own** the legacy `time_entries`
  migrate-vs-retire call on the boundary DEC-019 sets (legacy stays reference-only until then).
- **TASK-005:** capture adapter = the native Swift/AppKit helper; prove the signed helper holds the
  AX grant and passes notarization/Gatekeeper (the DEC-019 accepted-cost follow-up); keep raw titles
  out of the webview.
- **TASK-006:** runtime observer lives in the Rust core; reconcile against TASK-007's health
  taxonomy; do **not** become a duplicate cost/time ledger when valid Langfuse traces exist (DEC-017).
- **TASK-007 MVP:** durable importer = read-only Rust-core REST client behind the locked CSP + URL
  allowlist; re-confirm cost fields against Janne's real environments via `needs_input` if shapes
  differ.

## 7. Handoff

- **SW-2 implementer (primary):** **backend-developer (Rust/Tauri)** — owns the host-runtime decision
  record and the reuse boundary; confirms the sidecar/IPC/HTTP-allowlist wiring is the path TASK-004/
  005/007-MVP build on.
- **Consulted:**
  - **macos-capture (Swift/AppKit) developer** — helper/TCC-trust placement and the nested-binary
    codesign/notarization follow-up that TASK-005 must validate.
  - **integration-engineer** — Langfuse importer host-runtime fit (Rust-core REST pull, URL
    allowlist, DEC-018 cloud-first).
- **Mandatory QA / security checks for this decision's deliverables:**
  1. **Verification completeness:** confirm every named technology in `design.md` carries a
     **verified (source URL)** status or an **assumption + named follow-up**; confirm the
     nested-binary codesign gap is flagged, not asserted as fact.
  2. **Boundary correctness:** confirm the decision binds the AX grant to the helper, keeps raw
     titles off the renderer/network, and scopes the importer to a Langfuse-only Rust-core outbound
     allowlist (SEC-002).
  3. **DEC-017/DEC-018 fidelity:** confirm no new pi/Claude adapter is introduced and the importer
     stays a read-only cloud-first pull.
  4. **No-build check:** confirm no file under `src/`, `src-tauri/src/`, or `observability/` was
     created, modified, or deleted and no build target was added; confirm the legacy manual-tracker
     surface was not migrated, reused-as-capture, or wiped.
  5. **Artifact data safety:** confirm no credentials, real window/app titles, prompt/response text,
     command bodies, or environment dumps appear in any artifact.
  6. **Exit-gate text:** implementation path selected and documented before TASK-004; schema/capture/
     importer MVPs unbuilt; legacy fate deferred to TASK-004.
- **OpenSpec status:** `openspec validate task-003-implementation-path-decision --strict` → **valid**
  (run from the `code/` project root, 2026-06-05).
