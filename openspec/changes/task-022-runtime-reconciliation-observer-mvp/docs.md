# Docs gate — task-022-runtime-reconciliation-observer-mvp

**Tier:** L2
**Branch:** feat/task-022-runtime-reconciliation-observer-mvp (PR #14)
**Date:** 2026-06-12

## Changes made

### README.md — new section "Runtime reconciliation" (inserted between "AI trace import" and "Privacy status")

Added a user- and operator-visible section covering:
- Source: `pi-observe` `events.jsonl` at `~/.local/state/pi-observe/events.jsonl`, configurable via `VIRE_RUNTIME_LOG_PATH` or `PI_OBSERVE_STATE_DIR`; absence is a state, never zero
- Privacy boundary: strict ingest allowlist (type-enforced via serde), no process scanning, no command-lines, no prompts/env/secrets/free-text, no network calls, no new macOS permission
- Reconciliation states table (`matched`, `observed_no_trace`, `reconciliation_unknown`, `unmatched_runtime`, `unmatched_trace`) with meanings and the import-health gating rule
- Settings panel status line wording and "absent = unknown, not zero" framing
- TASK-009 callout for the full review UI

### README.md — "Privacy status" section

Added one sentence noting that the runtime reconciliation observer reads local coarse metadata only through a strict allowlist, makes no network calls, and stores no prohibited fields.

### docs/langfuse-local-setup.md — new section "Runtime reconciliation and import health" (before "Vire import endpoint configuration")

Added a section cross-referencing the import health states table to the reconciliation outcome:
- `observed_no_trace` gated on `healthy` import
- `unavailable`/`unknown`/`auth_or_network_error` → `reconciliation_unknown`, never zero
- Absent/unreadable runtime log → `reconciliation_unknown`
- Local-only, no network, no cost/time authority
- Cross-reference to README.md runtime reconciliation section for full detail

## Coherence check

| Requirement | README | langfuse-local-setup.md | Implementation |
|---|---|---|---|
| Source: local pi-observe events.jsonl, configurable | ✓ | ✓ (cross-ref) | `config.rs` `resolve_log_path()` |
| Absence → reconciliation_unknown, never zero | ✓ | ✓ | `mod.rs` `build_snapshot()`, `reconcile.rs` |
| Privacy: no process scanner, no argv/prompts/env/secrets | ✓ | ✓ | `model.rs` `RawEvent` allowlist type |
| Strict ingest allowlist (type-level, not runtime filter) | ✓ | — | `model.rs` serde drops non-listed keys |
| observed_no_trace gated on healthy import | ✓ | ✓ | `reconcile.rs` import-health check |
| No network egress, renderer off-network | ✓ | ✓ | `mod.rs` zero HTTP, CSP unchanged |
| Runtime evidence is reconciliation/health only, not cost/time | ✓ | ✓ | `model.rs` no token/cost column |
| Settings status line wording | ✓ | — | `main.ts` `runtimeLine()` |
| Local Langfuse Docker default preserved | ✓ | ✓ | existing docs sections unchanged |
| Cloud override explicit-only | ✓ | ✓ | existing docs sections unchanged |
| localhost bindings documented | ✓ | ✓ | existing docs sections unchanged |

## OpenSpec coherence

The implementation in `src-tauri/src/runtime_observer/` matches `specs/runtime-reconciliation/spec.md`:
- All five reconciliation states implemented (`ReconciliationState` enum in `model.rs`)
- `RawEvent` type enforces the allowlist boundary — serde silently drops every non-listed key
- `config.rs` `resolve_log_path()` follows `VIRE_RUNTIME_LOG_PATH` → `PI_OBSERVE_STATE_DIR` → `$HOME` fallback
- `get_runtime_reconciliation` IPC returns `RuntimeReconciliationSnapshot` with coarse counts, `runtime_log_present`, `source: "local"`, and a human-readable message — no secrets or session content
- `main.ts` `runtimeLine()` renders the status line with the "unknown, never zero" framing

No drift between OpenSpec spec and implementation detected.

## Docs not required at L2 / deferred

- Maintenance runbook (L3 only)
- Compliance/audit docs (L3 only)
- Retention automation docs (future task)
- Full review/approval UI docs (TASK-009, out of scope for this release)
