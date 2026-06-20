# Security review — TASK-034 Suggestions UAT polish (SW-5)

- **Change / branch / PR**: `task-034-suggestions-uat-polish` / `feat/task-034-suggestions-uat-polish` / #29
- **Reviewed through**: fix-loop commit `28f4e00` (QA PASS), implementation `51d52fb`
- **Project tier**: L2. **Task tier**: L1 (additive — DB columns + rendering, no new surface).
- **Verdict**: **PASS** — no auto-fail condition hit.

## Summary

TASK-034 adds same-minute span normalization (incl. day-end `23:59`), AI-cost persistence/reporting/CSV,
trackability/disabled-source UI notices, and a 30-minute clustering spec correction. The diff touches
**4 source files** (`src-tauri/src/lib.rs`, `src/main.ts`, `src/suggestions-ui.ts`, `src/summary-cards.ts`)
plus tests and OpenSpec docs. It introduces **no new Tauri command, IPC surface, network egress,
capability, or dependency**, and **no manifest/lockfile/capability change** — confirmed by
`git diff --name-only origin/main..HEAD` (no `tauri.conf`, `capabilities/`, `Cargo.*`, `package*.json`).
All dependency advisories below are pre-existing baseline carry, zero TASK-034 delta.

## Changed paths in scope

| Path | Security-relevant aspect | Result |
|------|--------------------------|--------|
| `src-tauri/src/lib.rs` | new DB columns, `normalize_same_minute_span`, summary/CSV SQL, accept INSERT | parameterized + single-tx — clean |
| `src/main.ts` | summary-cards wiring, disabled-source flags | escaped sinks — clean |
| `src/suggestions-ui.ts` | trackability badge, edit-default helpers, notices | static/escaped — clean |
| `src/summary-cards.ts` | new pure HTML builder, AI-cost sub-line | all output via `esc()` — clean |

## Scanner stack (Tier 1 — all four run)

| Scanner | Scope | Result | Auto-fail? |
|---------|-------|--------|------------|
| **gitleaks** 8.30.1 | full repo + history (174 commits, 3.08 MB) | no leaks found | No |
| **semgrep** 1.166.0 | 4 changed sources, `--config=auto` (427 rules: ts/rust/multilang) | 0 findings (0 blocking) | No |
| **OSV-scanner** 2.3.8 | `Cargo.lock` (492 pkgs) + `package-lock.json` (106 pkgs) | pre-existing advisories only — see below | Triaged → advisory |
| **Trivy** 0.71.1 | `fs --scanners vuln --severity HIGH,CRITICAL` | 0 HIGH / 0 CRITICAL | No |

## Dependency advisories — pre-existing, zero TASK-034 delta (not auto-fail)

No `package.json` / `package-lock.json` / `Cargo.toml` / `Cargo.lock` change in this task, so every OSV
finding is baseline carry, identical to the triage in `task-032/sec.md` and `task-033/sec.md`, now owned
by **TASK-043 (dependency advisory follow-up)**:

- **vite 6.4.2 → 6.4.3 — GHSA-fx2h-pf6j-xcff — CVSS 8.2 (npm, dev).** `vite` is a **devDependency**. The
  shipped artifact is the Tauri Rust binary plus static `dist/` output; the vite dev server is
  loopback-only and never in the `.app`. Not reachable in production, zero task delta. → **Advisory.**
- **vite — GHSA-v6wh-96g9-6wx3 — CVSS 5.5 (npm, dev).** Same package/fix (6.4.3). Advisory.
- **esbuild 0.28.0 → 0.28.1 — GHSA-g7r4-m6w7-qqqr — CVSS 2.5 (npm, dev).** Transitive via vite, dev-only.
  Advisory.
- **glib 0.18.5 → 0.20.0 — RUSTSEC-2024-0429 — CVSS 6.9 (crates.io).** `< 7.0`; transitive via the
  Tauri/GTK Linux GUI backend, not compiled into the macOS target. Advisory. (TASK-043 Stream B.)
- **~15 unscored gtk-rs / unic / proc-macro-error RUSTSEC advisories (no CVSS).** Transitive
  Linux-backend / unmaintained-class, pre-existing. Advisory. (TASK-043 Stream B.)

The single CVSS ≥ 7.0 finding (vite 8.2) is a dev-only build tool absent from the shipped artifact, not
reachable in production, and not introduced by this change. No CVSS ≥ 7.0 is reachable or introduced.

## Manual review (task scope)

- **XSS / DOM injection — none.** Every dynamic value reaching the DOM passes through `escapeHtml`
  (`& < > ' "`). New `summary-cards.ts` wraps each interpolation in `esc(...)` (project name, durations,
  `esc(costLabel(...))`); `costLabel` output is also escaped at the `<td>` sink in `suggestionRow`. New
  edit-default helpers `addMinutesHHMM` / `subMinutesHHMM` are pure numeric `HH:MM` math (no HTML), and
  their results land via `value="${esc(startVal)}"` / `${esc(endVal)}`. New `trackBadge`,
  `sourceDisabledNotice`, and `emptyState` markup is static — no untrusted interpolation.
- **Secret / trace / prompt exposure — none.** AI cost provenance is a numeric `cost_total` (f64) plus a
  currency code (`cost_currency`, usually NULL); no Langfuse key, trace body, or prompt text is copied or
  rendered. Accept copies only `suggestion.cost_total` / `cost_currency`. Render path remains secret-free.
- **SQL injection — none.** All new statements are parameterized (`?N` + `params![]`). The summary query
  only conditionally appends the static `" AND e.project_id=?3"` clause; the new
  `SUM(... cost_total)` / `MAX(... cost_currency)` are static SQL. No value is string-interpolated into SQL.
- **Transaction safety — preserved.** Accept remains a single SQLite transaction.
  `normalize_same_minute_span` runs before any mutation and returns `Result`; a malformed start/end errors
  out (tx dropped → rollback) before the guarded INSERT/UPDATE. Cost fields are bound params on the same
  INSERT. The `status='pending'` guard on the UPDATE is retained (no double-accept window).
- **CSV injection — neutralized.** `cost_currency` flows through `csv_escape`, which first applies
  `csv_formula_neutralized` (prefixes `'` for cells starting `= + - @` or leading control chars) then
  quote/delimiter-escapes. `cost_total` is a numeric `f64.to_string()` (no delimiter or formula risk;
  Rust Display never emits locale separators).
- **Auth / permission — no regression.** `project_exists_active` guard retained on accept; no auth model
  exists in this local single-user app and none is weakened.
- **Egress / capabilities / deps / local-only boundary — unchanged.** No new network call, command,
  capability, CSP, or dependency. Schema change is two additive nullable columns (`cost_total REAL`,
  `cost_currency TEXT`) via idempotent `add_column_if_absent`; existing rows backfill to NULL. All data
  stays in local SQLite; CSV writes to a user-chosen local path. Local-only boundary intact.

## Advisory (non-blocking)

- `normalize_same_minute_span` adds `chrono::Duration::minutes(minutes)` to a `NaiveDateTime`. `minutes`
  is bounded by the local engine (`max(1, round)`); a pathologically large value in a *corrupted local
  DB* could theoretically panic on overflow. Not a security finding for a local single-user app
  (no external input path); noted only for robustness. The two `.unwrap()`s are on constant valid times
  (`23:59:00`, `00:00:00`) and never fail.

## Auto-fail conditions

| Condition | Hit? |
|-----------|------|
| Any secret detected (gitleaks) | No |
| Any semgrep ERROR finding | No |
| Any dependency CVE ≥ 7.0 reachable/introduced | No (vite 8.2 = dev-only, not shipped, pre-existing) |
| Any Trivy HIGH/CRITICAL | No |

**SEC verdict: PASS.** No code-level or design-level security issue. No architect escalation. Proceed:
wait for SW-4 (already PASS) → route to SW-6 Release Manager.
