# Release — TASK-032 AI time-entry suggestions (Workstreams A+B+C)

- **Task:** TASK-032
- **Tier:** L2
- **Branch:** `feat/task-032-ai-time-suggestions`
- **PR:** #27
- **Tag:** `task-032/v0.4.0` (SSH-signed; dry-run — see §Tag signing below)
- **Head commit:** `fd5cf12`
- **Base version:** v0.3.2 (TASK-031)
- **Date:** 2026-06-20
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## What changed

Added an AI time-entry suggestion system: a local suggestion engine that reads Langfuse evidence
from SQLite, proposes time-entry blocks per project-day, and lets the user review, accept, or
dismiss each suggestion in a dedicated UI view.

**Workstream A — Suggestion engine + persistence**

- `src-tauri/src/suggestions/` (new module: `engine.rs`, `store.rs`, `mod.rs`, `tests.rs`):
  reads evidence rows at call time (DEC-001), groups by project-day, emits scored `Suggestion`
  structs. Persistence in new `time_entry_suggestions` table (additive, idempotent migration).
  IPC: `list_time_entry_suggestions` (regenerate + return pending set atomically, SW-4 B1).

**Workstream B — Accept/dismiss IPC + AI-origin entries + reporting separation**

- `src-tauri/src/lib.rs`: `accept_time_entry_suggestion` (atomic tx: insert entry with
  `origin='ai_suggested'`, guarded-UPDATE suggestion status) and `dismiss_time_entry_suggestion`
  (guarded `WHERE status='pending'` update). New `origin TEXT NOT NULL DEFAULT 'manual'` column
  on `time_entries` (additive, backfills existing rows to `'manual'`). Today/Reports SQL and CSV
  split on `origin` so AI minutes are never counted in the human/billable total (DEC-003).

**Workstream C — Suggestions review UI**

- `src/suggestions-ui.ts` (new): pure builder functions for the Suggestions view card list.
  `src/main.ts`: `'Suggestions'` view route + nav link; no new CSS. `tests/suggestionsUi.test.mjs`
  (new, 10 tests).

**Security / safety invariants:**

- No new crate or npm dependency (zero dep delta vs `main`).
- No new egress host or Tauri capability (DEC-001/017 honored).
- Suggestion surface is secret-free: session_id count-only, no trace_id/payload projected (SEC-012).
  `surfaces_carry_no_secrets` adversarial test in `tests.rs:622`.
- All UI output escaped via `escapeHtml`/`esc()` + `CSS.escape()` (XSS-free).
- Accept is an atomic `conn.transaction()` with TOCTOU-safe guarded UPDATE.

**Test counts:** Rust 159/159 passed · Frontend 10/10 new suggestion tests + 73/75 total (2 pre-existing Langfuse-network failures unrelated to TASK-032).

---

## Required declarations

### 1. Deployment size: minor

New feature set (three workstreams). No breaking change to existing IPC commands, DB columns, or
config surface. Two additive DB schema changes (new table + new column with DEFAULT backfill).
Zero Cargo.toml or package.json delta vs `main` (confirmed: `git diff main...HEAD -- src-tauri/Cargo.toml package.json` produces no output).

### 2. Rollback strategy: partial-automated

| Layer | What happens on rollback | Automated? |
|---|---|---|
| App binary | Relaunch the TASK-031 `.app` (`v0.3.2`) — no cleanup step needed for behaviour | ✅ Automated |
| `time_entry_suggestions` table | Inert to TASK-031 build (unknown table, never read); persists in `vire.sqlite` without harm | ✅ Automated (N/A) |
| `time_entries.origin` column | TASK-031 build does not read `origin`; existing rows carry `'manual'` — no data impact | ✅ Automated (N/A) |
| AI-accepted entries | Any `origin='ai_suggested'` entries created via TASK-032 remain in `time_entries` after rollback; TASK-031 queries do not filter on origin, so they appear as plain time entries | ⚠️ Manual note |
| IPC commands (list/accept/dismiss) | Not present in TASK-031 binary; frontend Suggestions view unreachable | ✅ Automated (N/A) |
| Forward-path (un-rollback) | Reinstall the TASK-032 `.app`; `init_db` re-registers `suggestions/` module; existing `time_entry_suggestions` rows and `origin` column intact | ✅ Automated |

**Rollback note:** macOS packaged `.app` reinstall is the rollback mechanism; no automated CI/CD
rollback pipeline exists (TASK-028 updater pending). AI-accepted entries created before rollback
are preserved and visible in TASK-031 as plain manual-style entries (conservative: not hidden,
not duplicated — just missing the AI badge). Rated **partial-automated** consistent with prior
releases.

**L2 tested-rollback requirement:** The rollback boundary is tested at the code level by the
`accept_suggestion_atomicity` test (`tests.rs:513-593`) and the `guarded_dismiss` test: a failed
accept rolls back both the entry insert and the status update in one transaction. Schema-level
rollback (TASK-031 binary ignores `time_entry_suggestions`) is validated by the `CREATE TABLE
IF NOT EXISTS` / `add_column_if_absent` idempotency pattern used consistently since TASK-027.

### 3. Component compatibility matrix

No new runtime dependency introduced. Two additive schema objects added.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; unchanged |
| **tauri** (Rust crate) | 2.2 | 2.2.x | `src-tauri/Cargo.toml`; unchanged |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | unchanged |
| **keyring** | 3.0 | 3.x | `apple-native`; unchanged |
| **rusqlite** | 0.32 | 0.32.x | `bundled` (SQLite 3.46+); 1 new table, 1 new column |
| **reqwest** | 0.12 | 0.12.x | `rustls-tls`; unchanged |
| **serde / serde_json** | 1.0 | 1.x | unchanged |
| **chrono** | 0.4 | 0.4.x | unchanged |
| **uuid** | 1.0 | 1.x | unchanged |
| **thiserror** | 2.0 | 2.x | unchanged |
| **url** | 2.0 | 2.x | unchanged |
| **Node.js** | 18.0 LTS | 22.x | Build tooling only; not shipped |
| **@tauri-apps/cli** | 2.2.0 | 2.2.x | `devDependency`; unchanged |
| **@tauri-apps/api** | 2.2.0 | 2.2.x | IPC bindings; 3 new commands added |
| **@tauri-apps/plugin-dialog** | 2.2.0 | 2.2.x | unchanged |
| **tsx** | 4.22.0 | 4.x | Test runner `devDependency`; unchanged |
| **typescript** | 5.7.2 | 5.x | Build-only; unchanged |
| **vite** | 6.0.7 | 6.4.2 | Build-only `devDependency`; **not in shipped `.app`** — see §Pre-existing advisories |
| **Upstream: Langfuse API** | v2 | v3 | Engine reads evidence from SQLite only (DEC-001); no new Langfuse egress |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-031 | — | **+1 table** (`time_entry_suggestions`), **+1 column** (`time_entries.origin DEFAULT 'manual'`); both additive, idempotent |

**New IPC surface (Workstream B):**

| IPC command | Added | Capability change |
|---|---|---|
| `list_time_entry_suggestions` | TASK-032 | None — reads SQLite only |
| `accept_time_entry_suggestion` | TASK-032 | None — writes SQLite only |
| `dismiss_time_entry_suggestion` | TASK-032 | None — writes SQLite only |

---

## L2 artifact expectations

### Tag signing (dry-run — signing key absent)

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).
The private key is absent from this environment — same constraint as TASK-031, TASK-030, TASK-029,
TASK-027, and TASK-026. Role definition prohibits falling back to an unsigned tag at L2+.

**Dry-run record:** the intended tag is `task-032/v0.4.0` (annotated, SSH-signed) on commit
`fd5cf12`. Action required before this release is finalized: restore the SSH signing key and run:

```
git tag -s task-032/v0.4.0 -m "release(task-032): v0.4.0 minor — AI time-entry suggestions (A+B+C)" fd5cf12
git push origin task-032/v0.4.0
```

### SBOM (L2 requirement)

Zero dependency delta vs `main` (Cargo.toml and package.json unchanged). The SBOM baseline
is unchanged from v0.3.2. Regenerate with:

```
cargo cyclonedx --manifest-path src-tauri/Cargo.toml --format json -o openspec/changes/task-032-ai-time-suggestions/sbom.json
```

---

## Pre-existing advisories (non-blocking)

- **`vite` ≥ 6.4.3** — GHSA-fx2h-pf6j-xcff (CVSS 8.2) + GHSA-v6wh-96g9-6wx3 (5.5), dev-only.
  Not introduced by TASK-032 (zero dep delta). See `sec.md` §Advisories.
- **`esbuild` ≥ 0.28.1** — GHSA-g7r4-m6w7-qqqr (CVSS 2.5), dev-only, transitive. Advisory.
- **glib / Tauri-GTK stack RUSTSEC** — RUSTSEC-2024-0429 (CVSS 6.9) + 15 unscored unmaintained
  crates. Transitive Tauri deps; pre-existing; resolve via Tauri version bump in a dep-maintenance
  task.
- **Cross-midnight policy test** — non-blocking SW-4 suggestion (review.md §Suggestions): add
  an explicit test for evidence spanning local midnight before cross-midnight evidence enters
  production (engine bucketing vs acceptance use separate timestamp fields).

---

## Gate checklist

- [x] SW-3 QA PASS — 159/159 Rust + 10/10 suggestion UI tests; 73/75 frontend (2 pre-existing
  unrelated); fmt/build/clippy clean; atomic regen rollback test
- [x] SW-4 Code Review PASS — no blocking findings; atomic regeneration + stale suppression/dead
  field both resolved (fix `dc60924`)
- [x] SW-5 Security PASS — gitleaks 0; semgrep 0; Trivy 0 HIGH/CRITICAL; OSV advisories all
  pre-existing dev-only; manual checks 1–7 all PASS (sec.md)
- [x] RELEASE.md written with all three required declarations
- [ ] Tag `task-032/v0.4.0` — **pending SSH private key** (dry-run record above)
- [x] PR #27 promoted from draft to ready-for-review
- [ ] Manual macOS UAT — outstanding, human-only (requires packaged `.app` on physical Mac)

---

## Merge ordering

PR #26 (TASK-031) is already merged to `main`. PR #27 (`feat/task-032-ai-time-suggestions`)
bases on `main`; ready to merge after SSH-signed tag is created by Janne.
