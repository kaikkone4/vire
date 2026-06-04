# Code Review — TASK-002 macOS capture feasibility spike

- **Change:** `task-002-macos-capture-feasibility`
- **Branch:** `feat/task-002-macos-capture-feasibility`
- **PR:** https://github.com/kaikkone4/vire/pull/7
- **Tier:** L2 · **Gate:** SW-4 (Code Review — craft/conventions/complexity/dead code)
- **Reviewer:** Code Reviewer (SW-4)
- **Date:** 2026-06-04
- **Verdict:** **PASS**

> Scope note: this is a feasibility *spike*. The deliverables are docs (OpenSpec change +
> capture-feasibility report) plus one isolated, non-shipping, compile-only Swift probe. Review
> targets craft, convention adherence, complexity, dead code, traceability, build isolation, and
> OpenSpec quality — not product-runtime correctness (none was changed) and not security (SW-5).

---

## 1. Independently verified

| Check | Result |
| --- | --- |
| `openspec validate task-002-macos-capture-feasibility --strict` | **valid** |
| `swiftc -typecheck spikes/task-002-macos-capture/probe/main.swift` | exit 0 (Swift 6.2.4, target arm64-apple-macosx15.0 — matches report header) |
| Spike referenced by `src/` | none (Grep clean) |
| Spike referenced by `src-tauri/` | none (Grep clean) |
| Spike in build graph (`Cargo.toml`/`tauri.conf.json`/`package.json`; no `Package.swift`/`.xcodeproj` exist) | not a member |
| `observability/` modified | no |
| Trailing newline on all reviewed files | present (single `\n`) |

Diff scope (10 files, +1224) is confined to the change dir and `spikes/task-002-macos-capture/`.
No product source, schema, or config touched.

## 2. Traceability

- Proposal/design/spec/tasks/report cross-reference the BA sources concretely (`04_technical_plan.md`
  §3/§4/§5/§6/§13, `05_project_plan_epics.md` §3/§6) and APP-005 / SEC-001 / SEC-007.
- All 23 `tasks.md` checkboxes (1.1–9.3) map to real report/probe content; spot-checked against
  report §2–§11 and probe functions — no checkbox marked done without backing content.
- `spec.md` requirements each carry properly-formed WHEN/THEN/AND scenarios; `--strict` passes.
- Commit messages follow conventional-commit style and accurately describe each layer
  (`docs:` arch+OpenSpec, `feat:` report+probe, `test:` QA gate). Good.

## 3. Build isolation & legacy boundary — PASS

Probe lives wholly under `spikes/task-002-macos-capture/`, outside product runtime, not in any build
target (verified above). README (`spikes/.../README.md:24-29`) and probe header
(`probe/main.swift:1-21`) both document the non-shipping/disposable status and the
delete-after-TASK-003 instruction. Legacy manual-tracker surface (`time_entries`, manual-entry view,
stopwatch CRUD) is explicitly preserved reference-only and not imported/migrated/reused/wiped
(`proposal.md:17-20`, `design.md:23-36`, `spec.md:102-108`); Grep confirms no entanglement.

## 4. No premature path decision — PASS

The Tauri+helper-vs-Swift-first decision is produced only as a *signal* and explicitly deferred to
TASK-003 (`capture-feasibility-report.md` §10 lines 331-348, exit-gate line 361; `design.md:57-58`).
The Quartz "recommend-against" call (§3.3/§5) is an in-scope permission-burden recommendation that
the spec requirement explicitly asks for (`spec.md:26-32`), not an implementation-path decision —
correctly scoped.

## 5. Craft of the Swift probe — PASS

Clean, idiomatic Swift: clear `MARK:` sectioning, descriptive names, small single-purpose functions,
no dead code (every function reachable from `runOnce()`; every struct field printed), no unused
imports, no commented-out code. Redaction-by-default (`redact()`) is applied at every free-form
string boundary. Thresholds in the probe (`60`/`300`, `main.swift:133-134`) match the report's state
table (§3.4) — consistent.

## 6. Suggestions (non-blocking)

1. **Probe emits a narrower capture-health taxonomy than the report documents.**
   `focusedWindowTitle` (`probe/main.swift:81-95`) folds `kAXErrorAttributeUnsupported` into
   `.noFocusedWindow` and turns an empty title into `.granted(redact(nil))` → `"<none>"`, so the
   probe never surfaces the report's distinct `window_unavailable` or `title_empty` states
   (`capture-feasibility-report.md` §3.2 table, §4). Harmless for a mechanics probe, but the
   TASK-005 adapter should carry the full §3.2/§4 taxonomy — worth a one-line note in the probe so
   the gap is intentional rather than accidental.

2. **`screenRecordingLikelyGranted` heuristic vs its comment.** `sampleQuartz()`
   (`probe/main.swift:109-122`) comments "if any *other-app* window exposes a name" but counts all
   named windows without filtering by owner pid. For a windowless CLI probe this is harmless, but
   the comment overstates the code's precision; a TASK-005 implementation should filter
   `kCGWindowOwnerPID != getpid()` to make the Screen-Recording proxy sound.

3. **Trivial: unreachable lower bound in `redact()`.** `case 0...20` (`probe/main.swift:35`) includes
   `0`, which can't be reached because the `guard` returns `"<none>"` for empty strings
   (`main.swift:33`). `1...20` would read more honestly. Cosmetic.

## 7. Escalations to SW Architect

None. Scope is component-aligned (single capture-adapter boundary) per the SW-1 arch review, and I
concur — no split, no redesign, no abstraction concerns.

## 8. Verdict

**PASS.** Craft and conventions are sound, complexity is low, there is no dead code, isolation and
the legacy reference-only boundary are verified, OpenSpec strict validation passes, traceability to
BA/APP-005 is concrete, and no implementation-path decision is prematurely made. The three items in
§6 are nice-to-have refinements for the TASK-005 adapter, not blockers. Proceed; await SW-5
(Security) before release to SW-6.
