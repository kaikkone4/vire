# Release — TASK-033 Reports quick-range presets

- **Task:** TASK-033
- **Tier:** L2 (task L1-equivalent)
- **Branch:** `feat/task-033-reports-quick-ranges`
- **PR:** #28
- **Tag:** `task-033/v0.5.0` (SSH-signed; dry-run — see §Tag signing below)
- **Head commit:** `b77d767`
- **Base version:** v0.4.0 (TASK-032)
- **Date:** 2026-06-20
- **Gates passed:** SW-3 QA (PASS), SW-4 Code Review (PASS), SW-5 Security (PASS), SW-6 Release (this doc)

---

## What changed

Added four **quick-range preset buttons** (Last 7 days / Last 14 days / Last 30 days / Last 90 days)
to the Reports view. Clicking a preset populates the `#rStart` / `#rEnd` date inputs and re-renders
the report — identical path to a manual date-range selection.

**Changed paths:**

- `src/report-ranges.ts` (new): pure `{start, end}` date helper `reportRange(days)` using local
  calendar fields via `localDateInputValue` — no UTC conversion; inclusive both ends; correct across
  DST and date-line timezones (`America/Los_Angeles`, `Pacific/Kiritimati` tested).
- `src/main.ts` (`renderReports`): preset button row injected above the existing date inputs;
  click handler writes `start`/`end` via `.value` (not `innerHTML`), calls `run(renderReports)`, and
  the next render captures the updated range in the CSV export closure.
- `tests/reportRanges.test.mjs` (new): five tests covering all four presets, inclusive length,
  month/year rollover, the one-day edge case, and DST / date-line correctness.

No backend, IPC, schema, Tauri capability, or dependency change in this task. Reports IPC
(`loadRange`, `export_report_csv`) is pre-existing and untouched.

---

## Required declarations

### 1. Deployment size: minor

New user-facing feature (preset shortcuts on an existing view). No breaking change to existing
IPC commands, DB columns, configuration surface, or prior Reports behaviour. The change is purely
additive DOM markup plus a new pure helper module.

Confirmed zero dep delta: `git diff main...HEAD -- src-tauri/Cargo.toml package.json` produces
no output.

### 2. Rollback strategy: partial-automated

| Layer | What happens on rollback | Automated? |
|---|---|---|
| App binary | Relaunch the TASK-032 `.app` (`v0.4.0`) — no cleanup step needed | ✅ Automated |
| DB / schema | No schema change; rollback leaves nothing behind | ✅ Automated (N/A) |
| IPC surface | No new commands; prior binary behaviour fully restored | ✅ Automated (N/A) |
| Frontend assets | `dist/` is regenerated per build; rollback `.app` carries prior `dist/` | ✅ Automated |

**Rollback note:** macOS packaged `.app` reinstall is the rollback mechanism; no automated CI/CD
rollback pipeline exists (TASK-028 updater pending). Rated **partial-automated** consistent with
prior releases.

**L2 tested-rollback requirement:** No schema or IPC change means rollback restores all prior
behaviour exactly. Frontend-only rollback is validated by the clean `npm run build` pass in SW-3
(qa.md).

### 3. Component compatibility matrix

No new runtime dependency introduced. No schema change. No IPC surface delta.

| Component | Min version | Max tested | Notes |
|---|---|---|---|
| **macOS** | 11.0 (Big Sur) | 15.x (Sequoia) | Tauri 2.x hard requirement; unchanged |
| **Rust toolchain** | 1.80.0 | 1.87.0 | edition 2021; unchanged |
| **tauri** (Rust crate) | 2.2 | 2.2.x | unchanged |
| **tauri-plugin-dialog** | 2.2 | 2.2.x | unchanged |
| **keyring** | 3.0 | 3.x | `apple-native`; unchanged |
| **rusqlite** | 0.32 | 0.32.x | `bundled` (SQLite 3.46+); no schema change |
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
| **Upstream: Langfuse API** | v2 | v3 | Reports IPC reads SQLite only; no new Langfuse egress |
| **Upstream: macOS Keychain** | Security.framework (macOS 11+) | — | unchanged |
| **Downstream: SQLite DB** | vire.sqlite schema as of TASK-032 | — | no change |

---

## L2 artifact expectations

### Tag signing (dry-run — signing key absent)

Git is configured for SSH signing (`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`).
The private key is absent from this environment — same constraint as TASK-026 through TASK-032.
Role definition prohibits falling back to an unsigned tag at L2+.

**Dry-run record:** the intended tag is `task-033/v0.5.0` (annotated, SSH-signed) on commit
`b77d767`. Action required before this release is finalized: restore the SSH signing key and run:

```
git tag -s task-033/v0.5.0 -m "release(task-033): v0.5.0 minor — Reports quick-range presets (Last 7/14/30/90 days)" b77d767
git push origin task-033/v0.5.0
```

### SBOM (L2 requirement)

Zero dependency delta vs `main` (Cargo.toml and package.json unchanged). The SBOM baseline is
unchanged from v0.4.0. Regenerate with:

```
cargo cyclonedx --manifest-path src-tauri/Cargo.toml --format json -o openspec/changes/task-033-reports-quick-ranges/sbom.json
```

---

## Pre-existing advisories (non-blocking; owned by TASK-043)

- **`vite` ≥ 6.4.3** — GHSA-fx2h-pf6j-xcff (CVSS 8.2) + GHSA-v6wh-96g9-6wx3 (5.5), dev-only.
  Not introduced by TASK-033 (zero dep delta). TASK-043 Stream A.
- **`esbuild` ≥ 0.28.1** — GHSA-g7r4-m6w7-qqqr (CVSS 2.5), dev-only, transitive. TASK-043 Stream A.
- **glib / Tauri-GTK stack RUSTSEC** — RUSTSEC-2024-0429 (CVSS 6.9) + ~15 unscored unmaintained
  crates. Transitive Tauri deps; Linux-backend only; not compiled into macOS target. TASK-043 Stream B.
- **Non-blocking (SW-4):** emit `aria-pressed="false"` on inactive preset buttons if toggle
  semantics are retained (`review.md`). Cosmetic/a11y; not a release blocker.

---

## Gate checklist

- [x] SW-3 QA PASS — 5/5 focused tests across `America/Los_Angeles` + `Pacific/Kiritimati`; build PASS; `git diff --check` PASS; full suite 88/90 (2 pre-existing unrelated failures in `pi-observe.security.test.mjs`)
- [x] SW-4 Code Review PASS — no blocking findings; date arithmetic, inclusive windows, project-filter preservation, export closure rebinding, dead-code, and changed-path scope all reviewed
- [x] SW-5 Security PASS — gitleaks 0 leaks (167 commits); semgrep 0 ERROR; Trivy 0 HIGH/CRITICAL secret/misconfig; OSV pre-existing dev-only advisories only; XSS/DOM-injection/date-input/export-closure/secret-exposure all clean
- [x] RELEASE.md written with all three required declarations
- [ ] Tag `task-033/v0.5.0` — **pending SSH private key** (dry-run record above)
- [x] PR #28 promoted from draft to ready-for-review
- [ ] Manual macOS UAT — outstanding, human-only (`tasks.md` §4.1–§4.4; requires packaged `.app`)

---

## Merge ordering

PR #27 (TASK-032) is merged to `main`. PR #28 (`feat/task-033-reports-quick-ranges`) bases on
`main`; ready to merge after SSH-signed tag is created by Janne.
