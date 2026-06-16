# Architecture Review — TASK-027 (Langfuse import + env mapping + UX polish)

**Reviewer:** SW Architect (SW-1)
**Change:** `openspec/changes/task-027-langfuse-import-env-mapping-ux-polish/`
**Tier:** L2
**Verdict:** **SPLIT-REQUIRED** — workstreams **A–E are one cohesive, in-boundary change** (fully
designed here and ready to route to developers); workstream **F (in-app app self-update) is carved out
into a new task, TASK-028**, because it requires update-signing + macOS notarization + a release
pipeline + a new network egress host that cross competency and the SEC-002 network boundary. Three
decisions recorded (**DEC-027** auto-import, **DEC-028** env discovery + mapping, **DEC-029** the F
split) and one security control (**SEC-010** secret-free diagnostics/discovery/mapping), all routed to
BA for the canonical logs.

> The split is **scoped to F only**. A–E do not require a split and are ready to implement as a single
> change. If Pi-Assistant prefers, A–E proceed immediately while TASK-028 is opened in parallel for F.

---

## 1. Scope vs. architecture plan

TASK-027 makes the Langfuse import path actually usable and realizes two boundaries the plan specified
but never built. It operationalizes the architecture; it does not change it.

- **`03_architecture_plan.md:89`** — the Langfuse Importer's job is to "query configured Langfuse
  environments; import trace timestamps, usage/cost fields … compute health state." A (diagnostics +
  payload tolerance) and C (discovery) are squarely inside that responsibility. **Honored.**
- **`03_architecture_plan.md:141,175`** — "map traces to Vire projects by **environment first**" /
  `PROJECT_MAPPING`. D realizes exactly this, currently unrealized. **Realized.**
- **`03_architecture_plan.md:144,152`** — "detect traces landing in `default`" / "pi-langfuse needs
  local patching for environment propagation." This is the documented cause of Janne's empty import; C+D
  give the user visibility and control over it. **Addressed.**
- **DEC-003 / DEC-017** — AI evidence stays separate; the importer is the sole AI cost/time authority.
  B (auto-import) is only a **trigger** for the same importer; no second authority is introduced.
  **Preserved.**
- **DEC-001 / DEC-006** — Vire records are source of truth; classification/allocation is suggestion-first
  with a human gate. D maps env→project as a Vire-authoritative mapping and only **suggests** project
  creation (explicit user action). **Preserved.**
- **DEC-020 / SEC-002** — local Docker Langfuse on loopback is the default; off-host is an explicit
  override. Discovery and auto-import reuse `validate_target` (loopback gate) and the `/api/public/` URL
  allowlist unchanged. **Preserved.**

No data-model component boundary (`§3`: capture / observer / importer / store / classifier / review /
exporter) is crossed by A–E. The runtime observer, capture, classifier, review UI, and exporter are
untouched.

## 2. Contract verified against the live code (authoritative source)

| Fact relied on | Verified at |
| --- | --- |
| Import counts exist but are discarded before the UI | `langfuse/importer.rs:26` (`ImportSummary`), `langfuse/mod.rs:58` (`import_result` → `Ok(())`), `lib.rs:303` (returns only snapshot) |
| Traces are hard-filtered by `environment` (default `vire`) | `langfuse/config.rs:226` (`append_pair("environment", …)`), `config.rs:15` (`DEFAULT_ENVIRONMENT`) |
| `default` is only a synthetic probe; other envs invisible | `langfuse/importer.rs:78-85` |
| Token/cost read from legacy fields only | `langfuse/model.rs:152-217` (`usage`, top-level tokens, `calculatedTotalCost`); `lacks_usage_and_cost` `model.rs:222` |
| URL allowlist + loopback gate to reuse for discovery | `langfuse/config.rs:168` (`validate_target`), `config.rs:193` (`build_url`) |
| Network-free importer test seam (mock `LangfuseApi`) | `langfuse/api.rs:14` |
| Bounded import path to reuse for auto-import | `lib.rs:296` (`run_bounded`), `lib.rs:289` (`recv_timeout`) |
| Disabled short-circuit pattern (no probe/Keychain) | `lib.rs:307`, `settings/mod.rs:127` (`langfuse_enabled`) |
| `settings` table + idempotent `init_db` (rollback basis) | `lib.rs:54` (`CREATE TABLE … settings`), `lib.rs:43-59` (`IF NOT EXISTS`) |
| Evidence already keyed by `environment` (join, no rewrite) | `langfuse/store.rs:38,30` (`langfuse_ai_evidence`/`langfuse_raw_traces` PK incl. environment) |
| Existing `create_project` path for suggest-create | `lib.rs:161,82` |
| Fake macOS controls drawn in content | `src/main.ts:35` (`<div class="traffic">`), `src/style.css` `.traffic` |
| Window keeps native decorations (real controls present) | `tauri.conf.json:13` (no `decorations:false`) |
| Placeholder icon fills the full canvas (no safe-area) | `src-tauri/icons/source/generate-vire-mark.mjs` (`roundedRectSDF` spans full `N`) |
| Renderer stays off-network; HTTP in Rust core only | `langfuse/mod.rs` (reqwest in core), CSP `tauri.conf.json:14`, `capabilities/default.json:6` |
| No updater plugin / signing today; unsigned build | `src-tauri/Cargo.toml` (no `tauri-plugin-updater`), `RELEASE.md:32`, `README.md:49` |

**Implementer levers (biggest correctness wins):** (1) **stop discarding `ImportSummary`** — the data
already exists; A is mostly plumbing. (2) **Capture the live payload first (A1) before widening the
parser** — do not guess the field names. (3) **Reuse `run_bounded` + the disabled short-circuit + the
mock `LangfuseApi`** rather than introducing parallel unbounded/un-mocked paths.

## 3. Component-boundary check (per workstream)

| WS | Components touched | Boundary verdict |
| --- | --- | --- |
| A | Importer, Store | In-boundary — importer's own evidence/health responsibility. |
| C | Importer (read), Store, Settings | In-boundary — read-only discovery scan + additive persistence + surfacing. |
| D | Store, Settings, Projects (read/create via existing path) | In-boundary — realizes `PROJECT_MAPPING`; Vire-authoritative; suggestion-first. |
| B | App runtime (lib.rs), Importer | In-boundary — a trigger, not a new authority; importer stays sole AI source. |
| E | Frontend shell, window config, icon asset | In-boundary — presentation only; no data model. |
| **F** | **Updater plugin, signing/notarization, release pipeline, NEW network egress** | **Out-of-boundary → split (TASK-028).** |

**Constraints implementers must hold (else stop and flag — these would be architecture-level):**

- **No new capability or CSP change.** Auto-import, discovery, and mapping are pure Rust core; the
  renderer makes no network call. If `capabilities/default.json` or the CSP needs editing, stop.
- **No second importer / no new AI-cost authority.** Auto-import calls the same `run_blocking_import`.
- **No silent project creation / auto-map.** Creation is explicit (DEC-006).
- **No new network egress host in this change.** Discovery stays on the configured Langfuse host under
  `/api/public/`. (F would add `api.github.com` / a CDN — that is precisely why F is split.)
- **Absence-≠-zero stays absolute.** New parser locations preserve `None`; the report never coerces zero.

## 4. The one external assumption (flag for SW-2)

Environment discovery (C) assumes Langfuse's public API has **no list-environments endpoint** (true for
the paths in use) and that calling `GET /api/public/traces` **without** the `environment` filter returns
cross-environment traces with `Trace.environment` populated. This is the only unverified external
premise in the change. **The implementer MUST confirm it against Janne's live stack (task C2) before
relying on it**, with a documented fallback (read `environment` from the trace/observation detail). If
neither works, **stop and flag** — discovery would need a different mechanism and the C/D design would
revisit.

## 5. DEC / SEC numbering

- **Highest committed decision** in `artifacts/ba/07_decision_log.md` is **DEC-022**; DEC-023–026 are
  already allocated (proposed) by TASK-023/024/025/026. **Next free is DEC-027.** Allocated:
  **DEC-027** (auto-import), **DEC-028** (env discovery + mapping), **DEC-029** (F split). These align
  contiguously after DEC-026.
- **Security register** `artifacts/ba/11_security_review.md` ends at SEC-008; SEC-009 is allocated by
  TASK-026. **Next free is SEC-010**, used for the secret-free diagnostics/discovery/mapping control.
- **Task numbering:** changes exist through TASK-026; this is **TASK-027**, and the F split is allocated
  as **TASK-028** (next free).

All are **proposed** here and routed to BA for the canonical logs (SW `code/` write-scope cannot edit
`artifacts/`).

## 6. Key decisions / constraints for implementers

- **DEC-027 (auto-import):** startup + periodic background import, serialized with manual, same
  `run_blocking_import`, respects `langfuse_enabled` + loopback, off-UI-thread, bounded; importer stays
  sole AI authority. See `proposal.md` §ADR / `design.md` §6.
- **DEC-028 (discovery + mapping):** discover environments by trace scan (no list endpoint); map
  env→project (Vire-authoritative, DEC-001); suggest-create only on explicit action (DEC-006). See
  `design.md` §4–5.
- **DEC-029 (F split):** app self-update → TASK-028 (recommended Phase 1 lightweight check, Phase 2
  signed/notarized auto-install). See §F below.
- **SEC-010 (secret-free surfaces):** the import report, counts, discovered-environment lists, and
  mapping data contain only counts/health/env-names/project-refs and the existing secret-free warning
  strings — never credentials, raw bodies, or prompt/session content. Extends SEC-003. Auto-import
  preserves SEC-002 + the disabled short-circuit.
- **Icon (E):** the safe-area fix applies to the **placeholder** generator in `code/`; the **branded**
  asset stays brand-owned (`artifacts/brand/` is read-only to SW) — document the requirement, don't
  write there.
- **Compat/rollback:** all new tables/columns additive via idempotent `init_db`; reverting to TASK-026
  leaves them unused; no destructive migration (mirror `RELEASE.md:36`).

## 7. One change vs. split — the decision

**A–E: one cohesive change, no split.** They share a single purpose ("make the import experience
actually work and finish the desktop chrome"), a single release, and the L2 thresholds (no-secret
diagnostics, compat/rollback, docs) span them. None crosses a data-model boundary. They are sequenced
(A → C → D → B → E) so Pi-Assistant can route each to the right developer, exactly as TASK-026 sequenced
its three workstreams under one change.

**F: split required → TASK-028.** Detailed below.

## §F — App self-update: split rationale (the explicit evaluation Janne asked for)

Janne's ask is "Update available — click to install." Evaluated honestly:

**A true click-to-install updater requires all of:**

1. **Tauri updater plugin** — a new dependency, a **new capability** in `capabilities/default.json`, and
   an `updater` block in `tauri.conf.json` carrying a **bundled minisign public key**. (None exist
   today — `Cargo.toml`, `capabilities/default.json:6`.)
2. **Update-artifact signing** — a minisign **private** key to sign every release artifact: a new
   secret-management concern (where the key lives, who holds it, CI access).
3. **macOS code signing + notarization** — for a downloaded update to launch without Gatekeeper
   blocking it, the app must be Developer-ID signed and notarized. This is **explicitly out of scope as
   of v0.1** (`RELEASE.md:32`, `README.md:49`) and needs an **Apple Developer account**.
4. **A release pipeline** — building, signing, notarizing, and publishing artifacts + a `latest.json`
   manifest to **GitHub Releases** (`origin` = `github.com/kaikkone4/vire`).
5. **A new network egress host** — the app would fetch the update manifest/artifact from
   `api.github.com` / a CDN. Vire today egresses **only** to loopback Langfuse (or an explicit Cloud
   override); adding a self-update host is a **change to the SEC-002 network boundary** and must be
   designed deliberately, not bolted on.

**Why not even the lightweight check in TASK-027?** A read-only "check the latest GitHub release, show
'update available', open the download page" is feasible and cheap, but it still (a) opens the **same new
egress host** (the boundary change), and (b) depends on the **release-hosting + version-source-of-truth
decision** that belongs with the updater. Designing that boundary once — with the updater — is cleaner
than introducing a one-off egress in a UX-polish task and redoing it later. So the recommendation is to
make the lightweight check **Phase 1 of TASK-028**, giving Janne the affordance soon, with Phase 2 the
full signed/notarized auto-install.

**Suggested TASK-028 scope (for Pi-Assistant / BA to open):**

> **TASK-028 — In-app application updates.**
> *Phase 1 (lightweight, low-infra):* read-only update **check** — compare the running version to the
> latest GitHub release; if newer, show "Update available" and open the release/download page in the
> browser. Introduces and documents the **new network egress host** (decision + SEC entry).
> *Phase 2 (full auto-install):* Tauri updater plugin + capability + bundled minisign public key,
> minisign artifact signing, **macOS code signing + notarization (Apple Developer ID)**, and a
> GitHub-Releases publish pipeline with `latest.json`. Competency: devops/release. Depends on an Apple
> Developer account and a key-management decision.

This split keeps TASK-027 cohesive and shippable now, and gives F the infrastructure design it actually
needs.

## 8. feedback_to_ba[]

```
feedback_to_ba:
  - id: FB-027-1
    type: decision-log-entry
    target: artifacts/ba/07_decision_log.md
    summary: >
      Record DEC-027 — Vire imports Langfuse AI evidence automatically on app startup and on a periodic
      background interval, in addition to the retained explicit manual import. Auto and manual imports
      share the single run_blocking_import path and are serialized (no concurrent imports on the local
      DB); auto-import honors langfuse_enabled and the SEC-002 loopback boundary, runs off the UI thread,
      is bounded, and resolves failures to the existing ten-state health taxonomy (never zero). The
      importer remains the sole AI cost/time authority (DEC-003/DEC-017 unchanged).
    reason: >
      SW write-scope is limited to code/; ADRs must land in the canonical BA decision log. DEC-027 is
      proposed in this change but needs a BA-flow entry to be authoritative.
    blocking: false

  - id: FB-027-2
    type: decision-log-entry
    target: artifacts/ba/07_decision_log.md
    summary: >
      Record DEC-028 — Vire discovers Langfuse environments by scanning recent traces (the public API
      has no list-environments endpoint) and lets the user map each environment to a Vire project
      (Vire-authoritative, DEC-001). A discovered-but-unmapped environment yields a suggestion to create
      a project, created only by explicit user action (DEC-006), never auto-created. Realizes the
      environment-first PROJECT_MAPPING in 03_architecture_plan.md §3/§4.3.
    reason: >
      Canonical decision log entry; out of SW write-scope.
    blocking: false

  - id: FB-027-3
    type: decision-log-entry + new-task-request
    target: artifacts/ba/07_decision_log.md / pi-assistant
    summary: >
      Record DEC-029 — the in-app app self-update capability (Janne's "Update available — click to
      install") is split out of TASK-027 into a new task, TASK-028, because click-to-install requires
      the Tauri updater plugin + new capability + bundled minisign public key, minisign artifact
      signing, macOS code signing + notarization (out of scope at v0.1 per RELEASE.md; needs an Apple
      Developer ID), a GitHub-Releases publish pipeline, and a new network egress host that changes the
      SEC-002 boundary. Recommended TASK-028 split: Phase 1 lightweight read-only version check (open
      download page); Phase 2 full signed/notarized auto-install. Please open TASK-028 with this scope.
    reason: >
      The split is an architecture/competency boundary decision and needs both a canonical log entry and
      a new task allocation, neither writable from SW code/ scope.
    blocking: false

  - id: FB-027-4
    type: security-register-entry
    target: artifacts/ba/11_security_review.md
    summary: >
      Record SEC-010 — secret-free import diagnostics, environment discovery, and mapping surfaces. The
      new import report (per-env and total counts, skipped, health, secret-free warnings), discovered-
      environment lists, and env→project mapping data contain no credentials, raw API response bodies,
      or trace prompt/session content. Automatic import preserves SEC-002 (loopback) and the disabled
      short-circuit identically to manual import. No new network egress host is introduced by TASK-027
      (the self-update host is deferred to TASK-028 and will carry its own SEC entry). Extends SEC-003.
    reason: >
      L2 no-secret-exposure threshold for the new surfaces; canonical security register is out of SW
      write-scope.
    blocking: false

  - id: FB-027-5
    type: numbering-confirmation
    target: pi-assistant / ba-architect
    summary: >
      Next free numbers used: DEC-027/028/029 (committed log ends at DEC-022; DEC-023–026 already
      allocated by TASK-023–026), SEC-010 (register ends at SEC-008; SEC-009 by TASK-026), and TASK-028
      for the split. Please confirm when recording the canonical entries.
    reason: >
      Avoid a decision/security/task number collision in the canonical logs.
    blocking: false
```

## Verdict

**SPLIT-REQUIRED.** Implement **A–E as the single TASK-027 change** as scoped in `proposal.md` /
`design.md` / `tasks.md` / `specs/*`, honoring DEC-027 (auto-import), DEC-028 (discovery + mapping),
SEC-010 (secret-free surfaces), the unchanged loopback allowlist + disabled short-circuit, the
suggestion-first project creation, and the additive/idempotent compat-rollback posture. **Carve F (app
self-update) into TASK-028** (DEC-029) — recommend Phase 1 lightweight version check, Phase 2
signed/notarized auto-install. Confirm the verified A1 payload shape (do not guess) and the C2 discovery
assumption before relying on them. Route DEC-027/028/029, SEC-010, and the TASK-028 allocation to BA /
Pi-Assistant (non-blocking). Sequence implementation A → C → D → B → E; A is the security-bearing
correctness core for the SW-4/SW-5 gates.
