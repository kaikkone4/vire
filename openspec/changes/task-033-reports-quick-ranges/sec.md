# Security review — TASK-033 reports-quick-ranges (SW-5)

- **Change / branch / PR**: `task-033-reports-quick-ranges` · `feat/task-033-reports-quick-ranges` · PR #28
- **Reviewed commit**: `b77d767` (feat(task-033): Reports quick-range presets)
- **Date**: 2026-06-20
- **Tier**: L2 project; task is L1-equivalent (frontend-only, zero dependency/IPC delta)
- **Verdict**: **PASS** — no auto-fail condition hit

## Summary

TASK-033 adds four static quick-range preset buttons (Last 7/14/30/90 days) to the Reports view plus a
pure local-date helper (`src/report-ranges.ts`). The change is frontend-only: it sets the existing
`#rStart`/`#rEnd` date inputs to a `{start, end}` pair the user could already type by hand, then re-renders
through the existing path. No new Tauri command, capability, network egress, secret access, schema, or
dependency. Clean across every scanned dimension. The only surfaced dependency advisories are the
**pre-existing, zero-delta** set already triaged in `task-032/sec.md` and now owned by `task-043`
(dependency-advisory follow-up) — not introduced by this change, not a TASK-033 auto-fail.

## Changed paths in scope

| Path | Nature | Security relevance |
|------|--------|--------------------|
| `src/report-ranges.ts` (new) | Pure `{start,end}` date math; static preset list | Date-input safety |
| `src/main.ts` (`renderReports`) | Preset button row + click handler | XSS / DOM injection, export closure |
| `tests/reportRanges.test.mjs` (new) | Unit tests | none |

## Scanner results (Tier 1)

| Scanner | Scope | Result | Auto-fail? |
|---------|-------|--------|------------|
| **gitleaks** 8.30.1 | full repo + history (167 commits, 2.92 MB) | no leaks found | No |
| **semgrep** 1.166.0 | changed source (`report-ranges.ts`, `main.ts`, `date.ts`, `html.ts`, test), `--severity ERROR`, 109 rules | 0 findings | No |
| **Trivy** 0.71.1 | fs `secret,misconfig` HIGH/CRITICAL | 0 secrets, 0 misconfig | No |
| **OSV-scanner** 2.3.8 | `Cargo.lock` (492 pkgs) + `package-lock.json` (106 pkgs) | pre-existing advisories only — see below | Triaged → advisory |

## Dependency advisories — pre-existing, zero TASK-033 delta (not auto-fail)

`git show b77d767 --stat` confirms **no `package.json`/`package-lock.json`/`Cargo.toml`/`Cargo.lock`
change** in this task. Every OSV finding is carried baseline, identical to `task-032/sec.md`, and is now
explicitly owned by **TASK-043 (dependency advisory follow-up)**. Repeating the established triage:

- **vite 6.4.2 → 6.4.3 — GHSA-fx2h-pf6j-xcff — CVSS 8.2 (npm, dev).** `vite` is a **devDependency**. The
  shipped artifact is the Tauri Rust binary plus static `dist/` build output (`frontendDist: "../dist"`);
  the vite dev server is loopback-only and never in the `.app`. Not reachable from production, zero
  task-033 delta. → **Advisory, not a TASK-033 auto-fail.** Fix owned by TASK-043 Stream A.
- **vite — GHSA-v6wh-96g9-6wx3 — CVSS 5.5 (npm, dev).** Same package/fix (6.4.3). Advisory.
- **esbuild 0.28.0 → 0.28.1 — GHSA-g7r4-m6w7-qqqr — CVSS 2.5 (npm, dev).** Transitive via vite, dev-only,
  pre-existing. Advisory.
- **glib 0.18.5 → 0.20.0 — RUSTSEC-2024-0429 — CVSS 6.9 (crates.io).** `< 7.0`; transitive via the
  Tauri/GTK Linux GUI backend, not compiled into the macOS-only target. Advisory. (TASK-043 Stream B.)
- **~15 unscored gtk-rs / unic / proc-macro-error RUSTSEC advisories (no CVSS).** Transitive Linux-backend
  / unmaintained-class, pre-existing. Advisory. (TASK-043 Stream B.)

No finding with CVSS ≥ 7.0 is reachable in the shipped app or introduced by this change.

## Manual review

- **XSS / DOM injection — clean.** Preset buttons render via `escapeHtml` (`src/html.ts` escapes
  `& < > ' "`) on `data-range="${esc(p.key)}"` and the label text — and both `key` and `label` come from
  the hardcoded `REPORT_RANGE_PRESETS` constant, not user input. `aria-pressed` is gated on a pure string
  comparison. No untrusted value reaches the markup.
- **Date-input safety — clean.** `reportRange(days)` derives `{start, end}` purely from numeric `Date`
  components via `localDateInputValue` → strictly `YYYY-MM-DD` (zero-padded numbers, no interpolated
  strings). `days` is a fixed constant (7/14/30/90); `start ≤ end` always holds. No user-controlled
  arithmetic or string passthrough.
- **No DOM-sink injection on apply.** The click handler writes the computed range to the inputs via the
  JS `.value` property (`si.value=s`), not `innerHTML`, then calls `run(renderReports)`. No markup path.
- **CSV / export closure — no security impact.** The `exportCsv` handler closes over `start`/`end`/`pid`
  captured at render time; selecting a preset re-renders, producing a fresh closure with the new range, so
  export honours the selected window. Values are passed as typed args to the **pre-existing, unchanged**
  `export_report_csv` Tauri command (`projectId: pid||null`); the CSV-writing backend is out of task-033
  scope and untouched. Closure-rebinding correctness was already verified in SW-4.
- **Secret exposure — none.** The Reports render path is secret-free; no credential, Keychain, or
  Langfuse access is touched.
- **No IPC / backend / capability / egress / dependency change.** Confirmed by `git show b77d767`: only
  `src/report-ranges.ts`, `src/main.ts` (`renderReports`), and the test changed. No new `#[tauri::command]`,
  no capability/allowlist edit, no network call. Reports IPC (`loadRange`, `export_report_csv`) is
  pre-existing and already accepts arbitrary start/end.

## Auto-fail conditions

- **None hit.** Secrets: none. semgrep ERROR: none. Trivy HIGH/CRITICAL secret/misconfig: none.
  Dependency CVSS ≥ 7.0 reachable/introduced: none.

## Escalations

- **Design-level (BA-flow Architect):** none.
- **Advisory (no block; before next release):** clear the carried dependency advisories via **TASK-043**
  (Stream A npm dev-deps `vite`→6.4.3 / `esbuild`→0.28.1; Stream B Tauri/GTK Rust chain). Pre-existing,
  not shipped, out of TASK-033 scope.
- **Non-blocking (carried from SW-4):** emit `aria-pressed="false"` on inactive preset buttons if toggle
  semantics are retained (`review.md`). Cosmetic/a11y, not security.

## Verdict

**PASS.** No auto-fail condition. Proceed — hold for SW-4 (already PASS) before SW-6 Release Manager.
