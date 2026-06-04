# QA Report — TASK-002 macOS capture feasibility spike

- **Change:** `task-002-macos-capture-feasibility`
- **Branch:** `feat/task-002-macos-capture-feasibility`
- **PR:** https://github.com/kaikkone4/vire/pull/7
- **Tier:** L2 · **Gate:** SW-3 (QA)
- **Verdict:** **PASS**
- **Date:** 2026-06-04
- **Kind note:** This is a feasibility *spike*, not a product MVP. The deliverable is the
  capture-feasibility report + isolated, compile-validated, redaction-by-default probe. QA
  validates evidence completeness, isolation, privacy, and gate satisfaction — not a shipping
  feature.

---

## 1. Automated checks

| Check | Result |
| --- | --- |
| `openspec validate task-002-macos-capture-feasibility --strict` | **PASS** (`Change … is valid`, exit 0) |
| `swiftc -typecheck spikes/task-002-macos-capture/probe/main.swift` | **PASS** (exit 0, `/usr/bin/swiftc`, macOS 15 CLT SDK) |
| Branch diff scope | **PASS** — only TASK-002 OpenSpec docs + `spikes/task-002-macos-capture/` |
| Probe non-shipping | **PASS** — no reference from `Cargo.toml`/`tauri.conf.json`/`package.json`/`Package.swift`/`.xcodeproj`; no reference under `src/`, `src-tauri/src/`, `observability/` |
| Committed probe output | **PASS** — no `*.log` / `out/` / binary committed; `.gitignore` covers them |
| Secret/credential scan (committed files) | **PASS** — no `ghp_/sk-ant-/AKIA/xox*` etc. patterns |

**Diff (9 files, +1132):** `arch-review.md`, `capture-feasibility-report.md`, `design.md`,
`proposal.md`, `specs/macos-capture-feasibility/spec.md`, `tasks.md`,
`spikes/task-002-macos-capture/{.gitignore,README.md,probe/main.swift}`. No `src/`,
`src-tauri/src/`, `observability/`, schema/config, or legacy manual-tracker changes.

---

## 2. Scenario coverage matrix (spec.md)

| Spec requirement / scenario | Evidence in deliverable | Status |
| --- | --- | --- |
| Capture feasibility validated across signals → Active-app capture validated | Report §3.1 (switch/launch/terminate/sleep-wake/Spaces/full-screen; TCC=none recorded); probe `sampleFrontmostApp` | **Covered** |
| → Window-title capture + degraded states validated | §3.2 + capture-health table; probe `focusedWindowTitle` with `permission_denied`/`no_focused_window`/`unavailable`/`title_empty` | **Covered** |
| → Quartz fallback evaluated vs permission burden | §3.3 (Screen Recording gate on macOS 15; recommend-against); probe `sampleQuartz` (counts names, never reads them) | **Covered** |
| → Idle/away signal validated | §3.4 + threshold table; probe `secondsSinceLastInput`/`activityState` | **Covered** |
| Permission burden documented → required vs optional + degraded behaviour | §5 table (Accessibility required; SR optional/not-recommended; zero-grant baseline) | **Covered** |
| Field allowlist & APP-005 → positive allowlist + non-collection list | §7.1 / §7.2 | **Covered** |
| → APP-005/SEC-001 implications + six-field UX mapping | §7.3 / §7.4 (+ SEC-007 transparency, SEC-002/003 N/A) | **Covered** |
| Manual validation matrix → required cases + expected behaviour | §9 (14 cases incl. grant/revoke, degraded, sleep/wake, Spaces/full-screen, core-app titles, idle thresholds, Quartz burden, privacy) | **Covered** |
| Spike outputs isolated → probe confined to spike path | §2 + verified: under `spikes/`, no build-target membership | **Covered** |
| → Legacy/manual-tracker reference-only | §2 + README isolation guarantees | **Covered** |
| Probe data handling preserves privacy → no real titles persisted/committed | §2, probe `redact()`, `.gitignore`, README cleanup step | **Covered** |
| No capture MVP / no path decision → defers MVP + path to TASK-003 | §1, §10 (signal only), §11 exit gate | **Covered** |

All 12 spec scenarios across 7 requirements have observable coverage in the report and/or probe.

## 3. tasks.md ↔ deliverable correspondence

All 23 checkboxes (1.1–9.3) map to real report/probe content: isolation/safety (§2),
NSWorkspace (§3.1, §8/FB-002), AX (§3.2), Quartz (§3.3), idle (§3.4), degraded states +
ActivityWatch (§4, §6), allowlist (§7), matrix (§9), deliverable + exit gate + verification
(§1, §10, §11). No checkbox is marked done without backing evidence.

---

## 4. Targeted gate findings

- **Manual matrix correctly classified as manual.** §2 and §9 state plainly the probe was
  compile-validated, **not executed against real activity**. The 14 matrix cases are documented
  manual checks to run on a real interactive session; the deliverable does **not** claim it ran
  them. QA did **not** execute the probe binary — running it samples real frontmost apps/titles
  (private data) and requires an interactive GUI/AX grant unavailable and unsafe in this runner.
  Marking as documented-manual, not failing, per gate rules. ✓
- **ActivityWatch absent — reference-only / non-blocking.** §6 states `aw-server` is not
  installed and was not installed (no-dependency rule); used only as a documentation benchmark.
  Correct classification. ✓
- **Full Xcode absent — non-blocking.** Feasibility rests on `swiftc -typecheck` against the
  macOS 15 CLT SDK, which QA re-ran successfully (exit 0). No dependencies installed. ✓
- **Exit gate satisfied.** §11: capture feasibility + permission burden documented;
  APP-005/SEC-001 field-allowlist implications identified (§7); Tauri+helper-vs-Swift-first is a
  *signal only* (§10) with the path decision explicitly deferred to TASK-003. No MVP shipped, no
  product evidence rows written, legacy surface untouched. ✓
- **Privacy posture clean.** Probe reduces every title to `<redacted len=N bucket=…>`
  (`redact()`); Quartz path counts name-exposing windows without reading names; idle uses event
  *age* only. No screenshots, keystrokes, command bodies, prompt/response text, env dumps, or
  secrets in any committed file. Only generic app names (Xcode/Slack/Finder/Mail/Linear) appear
  as examples — not private titles. No committed logs/binaries; `.gitignore` enforces this. ✓

---

## 5. Verdict

**QA STATUS: pass.** All spec scenarios have observable coverage, OpenSpec strict validation
passes, the Swift probe typechecks, the diff is correctly scoped to spike + OpenSpec docs, the
probe is non-shipping and isolated, the exit gate is met, and privacy guardrails hold. No
blocking issues. Route to SW-4 (Code Reviewer) ∥ SW-5 (Security Agent).
