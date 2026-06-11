# Design — TASK-003 implementation-path decision

> **⚠ SUPERSESSION NOTE (DEC-020 / DEC-022 · TASK-018, 2026-06-11).** This package was authored
> 2026-06-05 under **DEC-018 (cloud-first Langfuse source)**. DEC-018 is now **superseded by
> DEC-020**: Vire's canonical/default Langfuse source is **local Docker self-hosted Langfuse** at
> `http://127.0.0.1:3000`; Langfuse Cloud is an **explicit non-default override only**. The DEC-019
> decision below — reuse Tauri + native Swift/AppKit capture helper + **read-only Rust-core Langfuse
> REST importer behind the locked CSP + URL allowlist** — is **unaffected and remains valid in
> full**; only the importer's *default base URL / source posture* changes (and the privacy boundary
> is *strengthened*, since the default target is now on-host loopback). Wherever this file says
> "cloud-first" / "DEC-018" (notably **constraint 3** below and the trace lists), read
> "local-Docker-default per DEC-020; Cloud explicit override only". Authoritative realignment and the
> binding TASK-007 direction: `openspec/changes/task-018-local-langfuse-source-addendum/arch-review.md`.

## Context

TASK-003 is the Phase A **decision gate** (`05_project_plan_epics.md` §1 phase table; §4 chain:
TASK-001 → TASK-002 + TASK-007 spike + TASK-006 spike → **TASK-003** → TASK-004). Its single
mandate is to **choose and record** the implementation path before any schema or feature build.
DEC-008/DEC-009 already framed the choice — *reuse the current Tauri shell unless the capture spike
proves replacement is simpler* — and deferred the call to an implementation-path spike. That
evidence now exists (TASK-001/002/007). This design synthesises it into one decision.

## Goals / Non-goals

- **Goals:** weigh the three BA-mandated options against spike evidence; record the decision (ADR
  DEC-019) with a verified technology register; fix the architecture constraints TASK-004/005/006/
  007-MVP inherit; keep the decision inside DEC-017/DEC-018 and the L2 privacy posture.
- **Non-goals:** building the SQLite schema (TASK-004), the capture adapter (TASK-005), the durable
  Langfuse importer (TASK-007 MVP), or the runtime observer (TASK-006); migrating or wiping the
  legacy manual-tracker surface; finalising idle thresholds or the importer table shapes (those are
  TASK-005 / TASK-004 inputs, recorded here, not decided).

## Options considered

Criteria are drawn directly from the spikes (TASK-002 §10 trade dimensions; TASK-001 §2 salvage
classifications; TASK-007 arch-review §2/§4 host-runtime + SEC-002). "Capture API access" and
"permission burden" are deliberately **non-differentiating** because TASK-002 §10 proved the capture
APIs are equally reachable from a native helper or a Swift-first app.

| Criterion (weight) | A. Tauri reuse + native helper + Rust importer | B. Swift/AppKit-first single binary | C. ActivityWatch import/reference |
| --- | --- | --- | --- |
| Capture API access (gate) | Full — helper is native, same APIs validated in TASK-002 | Full — native | Indirect via its watchers |
| Permission burden (gate) | One AX grant; helper must be the TCC-trusted binary | One AX grant; simplest trust model | One AX grant (its process) |
| **Reuse of validated assets (high)** | **High** — keeps locked-CSP review/store/export shell + tested CSV escaping/formula-neutralization (TASK-001 §2.1/§2.5) | **Low** — rebuild review/store/export/CSV-safety from scratch; discards adversarial-tested primitives | **Low** — adopt its model; rebuild around it |
| **SEC-002 importer fit (high)** | **Strong** — read-only Rust-backend REST pull behind locked webview CSP + URL allowlist (TASK-001 §4.3, TASK-007 §4) | Adequate — native HTTP client, but boundary rebuilt by hand | Out of scope — AW does not import Langfuse |
| **Added complexity (high)** | IPC + helper packaging + nested-binary codesign/notarization + TCC-trust placement | Rebuild cost of the entire review/store/export UI + persistence + export-safety | New runtime + schema + weak pi/Langfuse fit |
| Privacy-boundary strength (high) | **Strong** — capture stays in helper; raw titles never cross into webview or network (`connect-src ipc:`) | Strong — single native process | External process owns the data model |
| EPIC-006 extension seams (med) | Clear adapter seams: capture helper · Rust importer · classifier · exporter (`03` §9 maintainability) | Achievable but rebuilt | Bends Vire to AW's seams |
| pi/Claude + Langfuse reconciliation fit (med) | Native — DEC-017 health taxonomy + TASK-006 reconciliation live in the Rust core | Native, rebuilt | Out of AW scope |

**Decision driver:** because capture feasibility does **not** favour either native option (TASK-002
§10), the deciding axis is **reuse value vs. rebuild cost**. Option A keeps a CSP-locked privacy
boundary plus adversarially-tested CSV-safety and a server-side network boundary that exactly fits
the SEC-002 importer — assets Option B would discard for **no capture gain**. DEC-009's switch
condition ("Swift-first only if capture/permission/helper integration proves *simpler* than reuse")
is **not met**: integration is not simpler, and reuse value is high. Option C remains a benchmark,
not a product runtime (TASK-002 §6; DEC-002).

## ADR DEC-019 — Reuse Tauri v2 shell + native Swift/AppKit capture helper + Rust-backend Langfuse importer

- **Status:** accepted (SW architecture decision; confirms & operationalizes DEC-008/DEC-009 with
  Phase A spike evidence). Route to BA decision log via `feedback_to_ba[]` for ratification.
- **Decision:** Build Vire on the **existing Tauri v2 / Rust / TypeScript shell (reuse)**, add a
  **native Swift/AppKit macOS capture helper as a bundled Tauri sidecar** (`externalBin`), and
  implement the **Langfuse importer as a read-only REST client in the Rust core** behind the locked
  webview CSP and a URL allowlist. **ActivityWatch is reference/benchmark only — not adopted.**
  **Swift/AppKit-first whole-app replacement is rejected.**
- **Constraints fixed for downstream (binding):**
  1. **The capture helper holds the Accessibility (AX) grant**, not the webview. TCC trust attaches
     to the signed binary that calls AX (TASK-002 §10), so the helper — not the renderer — is the
     TCC-trusted binary. Capture stays out of the renderer entirely.
  2. **Helper ↔ core via Tauri IPC** (sidecar stdin/stdout through the shell plugin; core→frontend
     via events/commands). Raw window titles flow helper → Rust core → (redacted/normalized) and
     **never** into the webview or any network path; the `connect-src ipc:` CSP stays locked.
  3. **The Langfuse importer runs in the Rust core** as a read-only pull (DEC-018 cloud-first),
     scoped to the configured Langfuse base URL via the Tauri HTTP allowlist; **no** raw macOS
     activity/prompt/command-body/env egress (SEC-002).
  4. **Reuse is evidence-driven.** Carry forward `reuse-as-is` assets (CSP, capabilities, CSV
     escaping/formula-neutralization, validation/error patterns); **do not** re-admit the legacy
     generic-tracker CRUD surface (`time_entries`, manual-entry view, stopwatch) as a capture path.
     Its migrate-vs-retire fate is a **TASK-004** decision on this boundary.
- **Rationale:** capture is equally feasible either way (TASK-002 §10); reuse preserves a
  CSP-locked privacy boundary, adversarially-tested export safety, and a server-side SEC-002
  boundary that fits the importer — discarding them (Option B) buys no capture benefit. Aligns with
  DEC-001 (local-only), DEC-007 (privacy/export), DEC-008/DEC-009 (salvage-first), DEC-017
  (Langfuse-first, no new adapter), DEC-018 (cloud-first import).
- **Consequences / accepted costs:** helper packaging + **nested-binary codesign & notarization** is
  the known friction (see register row below — assumption pending a TASK-005 packaging spike);
  sidecar lifecycle (crash/restart) is modelled as `capture_health` (TASK-002 §4). These are
  packaging concerns, not feasibility blockers.
- **Trace:** EPIC-001, EPIC-006, DEC-008, DEC-009, DEC-017, DEC-018; TASK-001 §2/§4.3, TASK-002
  §10, TASK-007 arch-review §2/§4/§6.

## Technology verification register

Per the research mandate, every named technology the decision rests on is verified against official
docs or marked as an assumption with a follow-up. Sources read 2026-06-05.

| Technology | Decision use | Status | Source / follow-up |
| --- | --- | --- | --- |
| Tauri v2 sidecar (`externalBin`) | bundle + run the native Swift capture helper as a sidecar | **verified** | https://v2.tauri.app/develop/sidecar/ — `bundle.externalBin`, per-target-triple naming (`*-aarch64-apple-darwin`), spawn via `tauri_plugin_shell` `app.shell().sidecar(...)`, stdin/stdout `CommandEvent`s |
| Tauri v2 IPC | helper↔core / core↔frontend messaging; renderer isolation | **verified** | https://v2.tauri.app/concept/inter-process-communication/ — async message passing; webview talks to core **only** via IPC (commands/events), no direct network/process access |
| Tauri v2 HTTP client (Rust) | Langfuse REST pull from the core, URL-allowlisted | **verified** | https://v2.tauri.app/plugin/http-client/ — re-exports `reqwest`; scoped/permissioned with URL allow/deny (e.g. `"allow":[{"url":"https://…"}]`) → fits SEC-002 |
| macOS TCC / Accessibility trust model | AX grant attaches to the signed helper binary | **verified** | Apple Accessibility/AXUIElement + Screen Recording privacy docs (read in TASK-002/DEC-002): https://developer.apple.com/documentation/applicationservices/axuielement , https://support.apple.com/guide/mac-help/mchld6aa7d23/mac |
| macOS nested-binary codesign + notarization for a sidecar | sign/notarize the embedded helper so TCC trust + Gatekeeper hold | **assumption — TASK-005 packaging spike** | https://v2.tauri.app/distribute/sign/macos/ confirms app-bundle signing/notarization but does **not** explicitly state nested sidecars are auto-signed; treat each embedded binary as separately signed (codesign all nested executables, then notarize the bundle). Validate concretely in TASK-005. |
| Swift / AppKit | native capture helper language/framework | **verified** | https://developer.apple.com/documentation/swift , https://developer.apple.com/documentation/appkit/nsworkspace (compile-validated probe in TASK-002) |
| ActivityWatch | reference/benchmark only (not adopted) | **verified (as reference)** | https://activitywatch.net/ , https://docs.activitywatch.net/en/latest/ — local server + watcher architecture; weak pi/Claude + Langfuse fit (TASK-002 §6) |
| SQLite (`rusqlite`, bundled) | local store, retained on the reused path | **verified** | https://www.sqlite.org/index.html (engine choice ratified TASK-001 §2.4; boundaries are TASK-004) |

## Open questions routed downstream

- **Nested-binary codesign/notarization** — the one accepted friction. TASK-005 must prove the
  signed helper holds AX and passes Gatekeeper; if packaging proves materially harder than expected,
  that is the only signal that would reopen DEC-019 toward Option B.
- **Legacy `time_entries` migrate-vs-retire** — decided in TASK-004 on the boundary DEC-019 sets
  (legacy stays reference-only until then; not a capture path).
- **Helper sampling cadence / idle thresholds** — directional in TASK-002; finalised in TASK-005.
- **Importer table shapes (`langfuse_import_runs`, normalized AI evidence)** — proposed in TASK-007
  spike; owned by TASK-004.
