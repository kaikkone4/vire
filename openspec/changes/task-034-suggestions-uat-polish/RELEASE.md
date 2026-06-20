# Release — TASK-034 Suggestions UAT polish

- **Task:** TASK-034
- **Tier:** L2 project (task L1 additive — DB columns + rendering, no new IPC surface or dep)
- **Branch:** `feat/task-034-suggestions-uat-polish`
- **PR:** #29
- **Tag:** `task-034/v0.6.0` (SSH-signed; dry-run — see §Tag signing below)
- **Head commit:** `28f4e00`
- **Base version:** v0.5.0 (TASK-033)
- **Date:** 2026-06-21
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## What changed

**Workstream A — Same-minute / 23:59 span normalization (bug fix)**

- New `normalize_same_minute_span` helper in `lib.rs` (replaces `bump_end_if_not_after`). A
  same-minute suggestion block (e.g. `09:00:10 → 09:00:50`) is normalized to a non-zero span
  before the accept transaction: forward bump if room exists, or anchor on `23:59` for the
  day-end edge case (`23:59:10 → 23:59:50` → stored as `23:58 → 23:59`, duration 1 min, no
  midnight cross). Manual entries still reject `start == end` at the existing validation path.
- Frontend mirrors both branches in `suggestions-ui.ts`: same-minute block → End defaults to
  `addMinutesHHMM(start, n)`; if `forward === start` (23:59 clamped) → End = start, Start =
  `subMinutesHHMM(start, n)` (new helper, floors at 00:00).

**Workstream B — AI cost persistence, reporting, and CSV (new capability)**

- Two additive nullable columns (`cost_total REAL`, `cost_currency TEXT`) added to `time_entries`
  via idempotent `add_column_if_absent`. `accept_suggestion_repo` copies `suggestion.cost_total` /
  `cost_currency` verbatim onto the inserted entry. Manual entry paths write `NULL` — absence ≠ zero.
- `SummaryRow` gains `ai_cost_total` and `ai_cost_currency` computed via
  `SUM(CASE WHEN origin='ai_suggested' THEN cost_total END)` — NULL on manual-only ranges, never 0.
  AI cost is never folded into `duration_minutes` (DEC-003).
- CSV export gains `cost_total,cost_currency` columns; AI-accepted rows carry the value, manual
  rows emit empty strings.
- `summary-cards.ts` (new): `summaryCards` builder — per-project card shows
  `AI-suggested {h} · {cost}` sub-line (omitted when `ai_minutes == 0`; "—" when `cost_total`
  is NULL). Lead "Total tracked" card aggregates via `aggregateAiCost`; mixed-currency result → "—"
  reported separately.

**Workstream C — Trackability and disabled-source notices (new UI)**

- Unmapped environment suggestion → `unmappedNotice` copy "not trackable until mapped" + "Map in
  Settings" action button.
- Untimed suggestion row → `.hint` badge "not auto-trackable — add time manually" in the summary
  `<tr>`, before the edit panel. Timed rows carry no badge.
- `emptyState` rewritten to list all causes with actions: nothing-imported, unmapped (when present),
  untimed, source-down (only when `sourceDegraded`). No bare empty table, no "0" shown.

**Workstream D — 30-minute gap spec correction (no code change)**

- `GAP_MINUTES` is `const i64 = 30` at `engine.rs:21` — always was. The spec was wrong (said
  configurable). Spec corrected to match implementation. No logic or test change required.

**Changed paths:**

- `src-tauri/src/lib.rs`: `normalize_same_minute_span`, `accept_suggestion_repo` (normalize + cost
  fields), `init_db` (additive columns), `TimeEntry`/`SummaryRow` structs and SQL, `export_csv_repo`
  (cost columns), 6 new unit tests.
- `src/suggestions-ui.ts`: `subMinutesHHMM` helper, `suggestionRow` day-end branches, `unmappedNotice`,
  `trackBadge`, `emptyState` rewrite.
- `src/summary-cards.ts` (new): `summaryCards`, `aggregateAiCost`.
- `src/main.ts`: summary-cards wiring, DTO type additions.
- `tests/suggestionsUi.test.mjs`: extended (subMinutesHHMM, DEC-035 23:59, C1–C4 coverage, SEC-012).
- `tests/summaryCards.test.mjs` (new): 7 B5 tests.

No backend IPC command, Tauri capability, network egress path, Rust crate, or npm package added.
No `tauri.conf`, `capabilities/`, `Cargo.toml`, `Cargo.lock`, `package.json`, or
`package-lock.json` change.

---

## Required declarations

### 1. Deployment size: minor

TASK-034 introduces new user-facing capability (AI cost in entries, reporting, and CSV export;
trackability notices in the Suggestions view) alongside a bug fix (same-minute normalization).
No breaking change to prior IPC commands, DB structure, or user-visible behaviour. The schema
delta is two additive nullable columns on `time_entries`; older binaries ignore them silently.

### 2. Rollback strategy: partial-automated

| Layer | What happens on rollback | Automated? |
|---|---|---|
| App binary | Relaunch the TASK-033 `.app` (`v0.5.0`) — no cleanup step needed | ✅ Automated |
| DB / schema | `cost_total` / `cost_currency` columns remain in place but are never read or written by older builds — inert; no data loss | ✅ Automated (N/A) |
| IPC surface | No new IPC commands; prior binary behaviour fully restored | ✅ Automated (N/A) |
| AI-cost entries | AI-accepted entries created under v0.6.0 persist as plain entries in older builds; cost is simply not displayed | ✅ Automated (non-destructive) |
| Frontend assets | `dist/` is regenerated per build; rollback `.app` carries prior `dist/` | ✅ Automated |

**Rollback note:** macOS packaged `.app` reinstall is the rollback mechanism; no automated
CI/CD rollback pipeline exists (TASK-028 updater pending). Rated **partial-automated** consistent
with prior releases.

**L2 tested-rollback requirement:** No IPC or schema breaking change; additive columns are inert
to older reads. The idempotent `add_column_if_absent` posture means a v0.5.0 binary opening the
same SQLite file operates normally. Validated by the clean `cargo test` + `npm run build` in SW-3.

### 3. Component compatibility matrix

No new runtime dependency introduced. Schema delta: two additive nullable `time_entries` columns
(`cost_total REAL`, `cost_currency TEXT`), both via `add_column_if_absent`.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; unchanged |
| **tauri** (Rust crate) | 2.2 | 2.2.x | unchanged |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | unchanged |
| **keyring** | 3.0 | 3.x | `apple-native`; unchanged |
| **rusqlite** | 0.32 | 0.32.x | `bundled` (SQLite 3.46+); additive-only schema change |
| **reqwest** | 0.12 | 0.12.x | `rustls-tls`; unchanged |
| **serde / serde_json** | 1.0 | 1.x | unchanged |
| **chrono** | 0.4 | 0.4.x | unchanged |
| **uuid** | 1.0 | 1.x | unchanged |
| **thiserror** | 2.0 | 2.x | unchanged |
| **url** | 2.0 | 2.x | unchanged |
| **Node.js** | 18.0 LTS | 22.x | Build tooling only; not shipped |
| **@tauri-apps/cli** | 2.2.0 | 2.2.x | `devDependency`; unchanged |
| **@tauri-apps/api** | 2.2.0 | 2.2.x | IPC bindings; no new commands |
| **@tauri-apps/plugin-dialog** | 2.2.0 | 2.2.x | unchanged |
| **tsx** | 4.22.0 | 4.x | Test runner `devDependency`; unchanged |
| **typescript** | 5.7.2 | 5.x | Build-only; unchanged |
| **vite** | 6.0.7 | 6.4.2 | Build-only `devDependency`; **not in shipped `.app`** — see §Pre-existing advisories |
| **Upstream: Langfuse API** | v2 | v3 | Evidence reads SQLite only; no new Langfuse egress |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-032 + two additive columns | — | no destructive change; backward-compatible |

---

## L2 artifact expectations

### Tag signing (dry-run — signing key absent)

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).
The private key is absent from this environment — same constraint as TASK-026 through TASK-033.
Role definition prohibits falling back to an unsigned tag at L2+.

**Dry-run record:** the intended tag is `task-034/v0.6.0` (annotated, SSH-signed) on commit
`28f4e00`. Action required before this release is finalized: restore the SSH signing key and run:

```
git tag -s task-034/v0.6.0 -m "release(task-034): v0.6.0 minor — Suggestions UAT polish: same-minute normalization, AI cost persistence/reporting/CSV, trackability notices" 28f4e00
git push origin task-034/v0.6.0
```

### SBOM (L2 requirement)

Zero dependency delta vs `main` (Cargo.toml and package.json unchanged). The SBOM baseline is
unchanged from v0.5.0. Regenerate with:

```
cargo cyclonedx --manifest-path src-tauri/Cargo.toml --format json -o openspec/changes/task-034-suggestions-uat-polish/sbom.json
```

---

## Pre-existing advisories (non-blocking; owned by TASK-043)

- **`vite` ≥ 6.4.3** — GHSA-fx2h-pf6j-xcff (CVSS 8.2) + GHSA-v6wh-96g9-6wx3 (5.5), dev-only.
  Not introduced by TASK-034 (zero dep delta). TASK-043 Stream A.
- **`esbuild` ≥ 0.28.1** — GHSA-g7r4-m6w7-qqqr (CVSS 2.5), dev-only, transitive. TASK-043 Stream A.
- **glib / Tauri-GTK stack RUSTSEC** — RUSTSEC-2024-0429 (CVSS 6.9) + ~15 unscored unmaintained
  crates. Transitive Tauri deps; Linux-backend only; not compiled into macOS target. TASK-043 Stream B.

---

## Gate checklist

- [x] SW-3 QA PASS — 165 Rust tests (`cargo test --lib`); 105 frontend tests (`npm run test:frontend` with `LANGFUSE_*` unset); build PASS; `git diff --check origin/main..HEAD` PASS; OpenSpec strict validation PASS
- [x] SW-4 Code Review PASS — no blocking findings; normalize/day-end logic, cost NULL semantics, AI/human separation, CSV injection neutralization, XSS escaping, and changed-path scope all reviewed
- [x] SW-5 Security PASS — gitleaks 0 leaks (174 commits); semgrep 0 ERROR; Trivy 0 HIGH/CRITICAL; OSV pre-existing dev-only advisories only; XSS/SQL/CSV/accept-tx/cost-exposure all clean
- [x] RELEASE.md written with all three required declarations
- [ ] Tag `task-034/v0.6.0` — **pending SSH private key** (dry-run record above)
- [x] PR #29 promoted from draft to ready-for-review
- [ ] Manual macOS UAT — outstanding, human-only (requires packaged `.app` on physical Mac)

---

## Merge ordering

PR #28 (TASK-033) is merged to `main`. PR #29 (`feat/task-034-suggestions-uat-polish`) bases on
TASK-033's merge commit; ready to merge after SSH-signed tag is created by Janne.
