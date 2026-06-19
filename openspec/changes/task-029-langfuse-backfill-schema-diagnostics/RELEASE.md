# Release — TASK-029 Langfuse schema diagnostics, tolerant v3 import, configurable range + resumable backfill

- **Task:** TASK-029
- **Tier:** L2
- **Branch:** `feat/task-029-langfuse-backfill-schema-diagnostics`
- **PR:** #23
- **Tag:** `task-029/v0.3.0`
- **Head commit:** `0d6037e4a85faf9e9d5b6b647258caaeaeef44c1`
- **Date:** 2026-06-19
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## Required declarations

### 1. Deployment size: minor

New features shipped; no breaking changes; no destructive schema migrations; no new runtime dependency
(Cargo.toml, Cargo.lock, and package-lock.json unchanged across `6f90661..HEAD` — verified by
`git diff 6f90661..HEAD -- src-tauri/Cargo.toml src-tauri/Cargo.lock package-lock.json` returning no output;
confirmed by `sec.md §2.8`).

Changes included (Workstreams A–D):

- **A — Forensic, secret-free schema diagnostics:** replaces the repeated free-string warning
  (`"a trace did not match the expected shape"` × N) with a fixed, secret-free `SkipReason` enum
  (`missing_trace_id`, `observations_not_embedded`, `field_type_mismatch`,
  `generation_lacks_usage_and_cost`, `observations_fetch_failed`) with aggregated counts per reason
  per environment and bounded structural samples (top-level JSON key names + offending field JSON type
  name only — no values, no credentials, no raw `serde` error strings; SEC-011 by construction).
- **B — Tolerant v3 trace identification:** decouples trace identification from usage/cost extraction.
  A trace with a usable `id` is always imported; peripheral field shape surprises no longer drop the
  whole trace. `Trace.observations` is widened to tolerate the Langfuse v3 list shape (ID-string array
  → fetch fallback); embedded observation objects remain the fast path. `schema_changed` is reserved for
  genuinely unreadable usage (`generation_lacks_usage_and_cost`, `observations_fetch_failed`) or
  unidentifiable (`missing_trace_id`) traces. Resolves the previously observed 611/640 skip rate.
  Absence-≠-zero invariant preserved (`None`, never `0`).
- **C — Configurable import range + incremental cursor + resumable chunked backfill:**
  - New `langfuse_import_range` settings row: `last_7d | last_30d | last_90d | all | since:<RFC3339>`,
    default `last_30d` (was fixed `last_7d`). IPC `get_langfuse_import_range` /
    `set_langfuse_import_range`. Malformed `since:` rejected with a fixed, non-echoing error string.
  - **Incremental import** resolves a per-environment window from the persisted `cursor_ts`
    (resume from `max(range_floor, cursor_ts − OVERLAP_SECS=3600)`; cursor never regresses).
  - **Backfill now** (`backfill_langfuse_now` IPC, 300 s timeout): re-scans floor→now in ordered monthly
    chunks, each committed atomically (S-3 invariant preserved). Interruption loses at most the
    in-flight chunk; re-running resumes via the inclusive DEC-032 cursor; durable
    `(environment, trace_id)` dedupe ensures each trace is imported exactly once.
  - **DEC-032 inclusive-from cursor:** all trace-list requests use `orderBy=timestamp.asc`
    (fixed literal). Page-limited runs persist `fromTimestamp = max_reached` to
    `langfuse_backfill_progress`; next run re-reads the full boundary instant from page 1 and dedupe
    suppresses overlap. Single-instant saturation (≥ 50 000 traces at one millisecond) surfaces a
    distinct terminal/capped diagnostic (boolean flag, no timestamp value) — never a silent skip or
    infinite loop.
  - All backfill/incremental paths honour the SEC-002 loopback boundary, `langfuse_enabled`
    short-circuit, `import_lock` serialisation, and off-UI dedicated thread — identical posture to
    the prior `import_langfuse_now` path.
- **D — Grouped, actionable import summary:** `renderImportReport()` (extracted to testable
  `src/import-report.ts`) surfaces per-environment seen/new/duplicate/skipped counts, grouped
  skip-reason breakdown, bounded structural-sample `<details>` (key/type names only), incremental-vs-
  backfill headline, page-limit continuation note, and saturation terminal note (distinct from
  ordinary re-run note). All imported text HTML-escaped (SEC-011). `main.ts` imports from
  `import-report.ts`. Range-control helpers in `src/langfuse-settings.ts`; styling in `src/style.css`.

### 2. Rollback strategy: partial-automated

| Layer | What happens on rollback | Automated? |
|---|---|---|
| App binary | Relaunch the TASK-027 `.app` — no cleanup step needed | ✅ Automated |
| `langfuse_import_range` settings row | Created by `init_db` (additive key/value row); TASK-027 build ignores unknown settings keys — no schema conflict, no data loss | ✅ Automated (schema-free at rollback) |
| `langfuse_backfill_progress` table | Created by `init_db` (`CREATE TABLE IF NOT EXISTS`); TASK-027 build ignores unknown tables — no conflict, no data loss; re-created idempotently on re-upgrade | ✅ Automated (schema-free at rollback) |
| Import window behaviour | TASK-027 build uses its fixed 7-day window; no data loss — incremental cursor is in `langfuse_discovered_environments` (unchanged); prior evidence rows unchanged | ✅ Automated |
| Import diagnostics (A–B) | TASK-027 build surfaces the prior coarse `schema_changed` health and repeated-string warnings; no data loss | ✅ Automated |
| Backfill IPC commands | Not registered by TASK-027 build; Backfill now button is absent — import is manual 7-day-window only | ✅ Automated |
| Frontend report renderer | TASK-027 build's renderer: no grouped reasons, no range control, no saturation note | ✅ Automated |
| Forward-path (un-rollback) | Reinstall the TASK-029 `.app` — both new DB artefacts intact, backfill cursor preserved, durable dedupe ensures no double-import | ✅ Automated |

**Rollback tested (L2 precondition):** SW-3 QA (`qa.md`) verifies idempotent `init_db`, additive
`langfuse_backfill_progress`, backfill resumability and durable-dedupe convergence
(`backfill_imports_history_in_chunks_and_is_resumable`), and rollback-safe additive settings
(`import_range_setting_persists_validates_and_defaults`). Full macOS smoke (packaged `.app`, backfill
E2E with interrupt + re-run, rollback to TASK-027 `.app`) requires a physical Mac and is documented
in the manual smoke checklist in the root `RELEASE.md §Manual smoke steps before shipping`.

### 3. Component compatibility matrix

No new runtime dependency was introduced (Cargo.toml, Cargo.lock, and package-lock.json are unchanged
across the TASK-029 diff — verified by empty `git diff 6f90661..HEAD -- src-tauri/Cargo.toml
src-tauri/Cargo.lock package-lock.json`). The matrix extends TASK-027's.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged from TASK-026/027 |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; `cargo build` clean; 142 tests pass |
| **tauri** (Rust crate) | 2.2 | 2.2.x | `src-tauri/Cargo.toml`; unchanged from TASK-026/027 |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | `src-tauri/Cargo.toml`; unchanged from TASK-026/027 |
| **keyring** | 3.0 | 3.x | `apple-native` only; no new usage in TASK-029 |
| **rusqlite** | 0.32 | 0.32.x | `bundled` (SQLite 3.46+); new `langfuse_backfill_progress` table uses same additive posture |
| **reqwest** | 0.12 | 0.12.x | `rustls-tls`; backfill/incremental paths use same `get_traces` + `build_url` client |
| **serde / serde_json** | 1.0 | 1.x | |
| **chrono** | 0.4 | 0.4.x | |
| **uuid** | 1.0 | 1.x | |
| **thiserror** | 2.0 | 2.x | |
| **url** | 2.0 | 2.x | `ApiPath::Traces` extended with `orderBy` parameter |
| **Node.js** | 18.0 LTS | 22.x | Build tooling only; not shipped in `.app` |
| **@tauri-apps/cli** | 2.2.0 | 2.2.x | `devDependency`; used for `tauri build` |
| **@tauri-apps/api** | 2.2.0 | 2.2.x | IPC bindings in renderer; `backfill_langfuse_now` / `get_langfuse_import_range` / `set_langfuse_import_range` added |
| **vite** | 6.0.7 | 6.4.2 | Build-only `devDependency`; **not in shipped `.app`** — see §Pre-existing advisories |
| **Upstream: Langfuse API** | v2 | v3 | `/api/public/traces` with `orderBy=timestamp.asc` + `fromTimestamp` + `environment` filter; backfill adds no new endpoint or egress host |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged from TASK-026/027 |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-027 | — | one new table (`langfuse_backfill_progress`), one new settings row (`langfuse_import_range`); no destructive migration |

---

## L2 artifact expectations

### Tag signing (dry-run — signing key absent)

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).
The private key is absent from this environment (`~/.ssh/id_ed25519: No such file`) — same constraint
as TASK-026 and TASK-027 (see prior release artifacts for context).

**Dry-run record:** the intended tag is `task-029/v0.3.0` (annotated, SSH-signed) on commit
`0d6037e4a85faf9e9d5b6b647258caaeaeef44c1`. The role definition prohibits falling back to an unsigned
tag at L2+. Action required before this release is finalized: restore the SSH signing key and run:

```sh
git tag -s task-029/v0.3.0 -m "task-029: Schema diagnostics, tolerant v3 import, configurable range + resumable backfill (v0.3.0, minor, partial-automated rollback)"
git push origin task-029/v0.3.0
```

Verify with: `git tag -v task-029/v0.3.0` (requires the signing key in the verifier's
`allowedSignersFile`).

**This is a local prototype — no production deployment pending.** The signing gap does not block PR
review or merge; it must be resolved before any distribution artifact is published.

### SBOM

No formal SBOM generated for this release. No new dependency was introduced; the existing
`src-tauri/Cargo.lock` (492 packages) and `package-lock.json` (106 packages) remain the
machine-readable manifests. OSV-scanner 2.3.8 ran against both at SW-5 (`sec.md §1`). A
toolchain-generated SBOM (`cargo-sbom` / `syft`) is recommended before any L3 or public distribution
milestone — same standing recommendation as TASK-026/027.

### Signed binary / notarization

The packaged `.app` is not code-signed or notarized (out of scope for this L2 prototype). Unchanged
from TASK-026/027.

### Tested rollback

Verified at SW-3 QA level: idempotent `init_db` (`CREATE TABLE IF NOT EXISTS`), additive
`langfuse_backfill_progress` table, backfill resumability and convergence, inclusive-cursor
dedup-correctness (`backfill_imports_history_in_chunks_and_is_resumable`,
`backfill_equal_timestamp_block_at_boundary_is_fully_reimported_not_skipped`), saturation terminal
diagnostic (non-looping, no cursor advance past unread data), and additive settings rollback. Full
macOS smoke (packaged `.app`, backfill interrupt + re-run, rollback to TASK-027 `.app`) requires a
physical Mac and is documented in the root `RELEASE.md §Manual smoke steps before shipping`.

---

## Pre-existing issues (classified non-blocking)

| Issue | Classification | Recommended action |
|---|---|---|
| `tests/pi-observe.security.test.mjs` — 2 failures (sandbox EPERM on 127.0.0.1 bind) | Pre-existing; file unchanged from `main` in all TASK-029 commits (`git diff --name-only 6f90661..HEAD -- tests/pi-observe.security.test.mjs` returns empty); tests the `pi-observe` CLI, not the Vire Tauri app | Separate issue against pi-observe observability tooling; not a TASK-029 blocker |
| `vite` 6.4.2 GHSA-fx2h-pf6j-xcff / `esbuild` GHSA-gv7w-rqvm-qjhr (CVSS 8.1/8.2) | Dev-server `devDependencies` only; absent from shipped `.app`; pre-existing from TASK-026 (package-lock.json unchanged in TASK-029 diff) | Follow-up housekeeping: `npm update vite` → latest 6.x |
| `glib` 0.18.5 RUSTSEC-2024-0429 (CVSS 6.9 Medium — below 7.0 L2 auto-fail) | Pre-existing Tauri framework transitive dep; below auto-fail threshold; not on macOS hot path | Track at Tauri framework upgrade level |
| ~16 no-CVSS RustSec advisories (GTK/unic transitive) | Pre-existing; GTK Linux backend unused on macOS | Advisory only |
| `import-report.ts:140` continuation copy reads "oldest history … progressively further back" (ascending sweep resumes toward newer, not older, history) | Non-blocking copy error flagged by SW-4 (`review.md §Suggestions`); static text, no SEC-011 / value-interpolation impact | Fix copy in a follow-up commit before or after merge |
| N+1 `GET /api/public/observations` per trace during backfill | Correct, slower; documented at the fetch site and in `design.md §4.4`; chunked backfill bounds the per-run cost | Windowed-scan optimization recommended as TASK-029 follow-up (C6 acknowledged-deferred) |
| `seen_trace_ids` `HashSet` per-run memory | Acceptable for single-user prototype scale; flagged in root `RELEASE.md §Known limitations` | Bounded-cursor approach if histories grow large |

---

## Gate summary

| Gate | Verdict | Artifact |
|---|---|---|
| SW-3 QA | PASS (142 Rust / 71 frontend tests; TypeScript clean; `npm run build` clean; `cargo fmt --check` clean; `openspec validate --strict` clean — 2 SW-4 recheck cycles resolved) | `qa.md` |
| SW-4 Code Review | PASS (all blockers resolved at commits `4018a00`, `242f670`, `11c8e1c`; DEC-032 redesign fully verified) | `review.md` |
| SW-5 Security | PASS (gitleaks 0 history leaks; OSV max CVSS 6.9 < 7.0; Trivy 0 HIGH/CRIT; semgrep 0 ERROR; SEC-011 verified structurally and by test; no new egress/CSP/capability/dependency) | `sec.md` |
| SW-6 Release | **PASS** | this document |

**Release approval = Janne merging PR #23.**
