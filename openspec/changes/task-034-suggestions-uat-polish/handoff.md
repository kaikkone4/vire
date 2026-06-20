<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste. -->

# Handoff — TASK-034 Suggestions UAT polish

- **Change / branch / PR**: `task-034-suggestions-uat-polish` /
  `feat/task-034-suggestions-uat-polish` / #29
- **Phase / gate**: SW-2 fix loop 2 **done** (2026-06-21) → route to SW-3/SW-4 recheck
- **Tier**: L1

## Gate result

DEC-035 escalation resolved (backend + UI normalize a `23:59` same-minute suggestion to
`23:58 → 23:59`). AI cost persistence/reporting/CSV, fixed 30-minute contract correction, and accept
transaction boundaries pass review.

## Blocker resolved (SW-4 fix loop 2)

Disabled AI evidence source is now surfaced **even when pending suggestions exist**. New pure
`sourceDisabledNotice()` (`src/suggestions-ui.ts`) renders above the groups with an Open-Settings action;
`suggestionsBody` takes `{sourceDegraded, sourceDisabled}` and emits it for the non-empty path (empty
state already names disabled, so no duplication). `renderSuggestions` (`src/main.ts`) computes
`sourceDisabled` from `sourceHealth`. Down/stale were never the gap — the shared `sourceBanner()` already
renders them above the body. **No backend change.** Test added: disabled source + non-empty list. Stale
`Summary` comment + `tasks.md` checkboxes fixed. `engine.rs` comment left untouched (backend, out of scope).

## Exact next action

SW-3/SW-4: recheck the disabled-source surface fix and the cheap-suggestion cleanups; nothing else changed.

## Checks

- Rust: not re-run this pass (no backend change); last green 165/165, fmt clean, pre-existing Clippy only.
- Focused `suggestionsUi`: 18/18 (+2 disabled-source tests); my-domain `suggestionsUi`+`summaryCards`: 25/25; build green.
- Full frontend (`LANGFUSE_*` unset): 105/105 (+2 new).
- OpenSpec strict validation and `git diff --check`: pass.

## Review artifact

`openspec/changes/task-034-suggestions-uat-polish/review.md`
