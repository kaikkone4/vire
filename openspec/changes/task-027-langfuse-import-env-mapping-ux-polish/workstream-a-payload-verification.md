# Workstream A — A1 live payload-shape verification (secret-free)

**Author:** Backend Developer (SW-2) · **Date:** 2026-06-16 · **Scope:** TASK-027 Workstream A only.

Records what was verified about the live Langfuse payload shape before widening the parser (A4).
Per SEC-003 / SEC-010 and the A1 constraint, this note contains **no credentials, no
`Authorization` headers, no raw response bodies, and no trace prompt/session content** — only
reachability facts, version, and field-name shape notes.

## What was checked (localhost only, read-only)

- **Live stack reachable.** Janne's local Langfuse web is published on the host at
  `http://127.0.0.1:3010` (the container's `3000` is mapped to host `3010`; the importer default
  `127.0.0.1:3000` was *not* listening). The unauthenticated health endpoint returned
  `{"status":"OK","version":"3.178.0"}` — a **current v3** Langfuse.
- **Endpoints exist and are auth-gated.** `GET /api/public/traces` and
  `GET /api/public/observations` both returned **HTTP 401** without credentials, confirming the
  paths the importer uses are present and protected. No authenticated body capture was performed:
  the API keys live in the macOS Keychain and extracting them to capture a raw body would violate
  the SEC-003 / A1 no-credentials boundary, so it was deliberately **not** done.
- **Prior shape-only capture reused.** The committed TASK-007 validation report
  (`openspec/changes/task-007-langfuse-importer-validation/langfuse-validation-report.md`, §2.3)
  already recorded the observation usage/cost shape **shape-only** against this same project. That
  record plus the live version are the verification basis here.

## Where usage and cost live (confirmed)

A **generation** observation on current Langfuse (v3) carries token usage and cost in **both**
the legacy and the current locations; the live stack emits the current maps:

| Quantity | Legacy location (already parsed) | Current location (added in A4) |
| --- | --- | --- |
| prompt tokens | top-level `promptTokens`, else `usage.input` | `usageDetails["input"]` |
| completion tokens | top-level `completionTokens`, else `usage.output` | `usageDetails["output"]` |
| total tokens | top-level `totalTokens`, else `usage.total` | `usageDetails["total"]` |
| per-call cost | `calculatedTotalCost` | `costDetails["total"]` |

- `usageDetails` / `costDetails` are **maps that may be absent or empty** (TASK-007 §2.3 observed
  "0 or 4 keys" / "0 or 3 keys"). Per-model extra keys (cache/reasoning tokens) are ignored; only
  the conventional `input`/`output`/`total` keys are read.
- Trace-level `totalCost` remains an aggregate-only convenience; the authoritative per-call values
  stay on observations (unchanged).

## Consequence for the importer (A4)

The previous parser read usage/cost from the legacy locations only, so a generation that populates
**only** `usageDetails` / `costDetails` would have tripped `lacks_usage_and_cost()` →
`schema_changed` with totals withheld — the "reachable but no useful data" symptom. A4 widens
`Observation::{prompt,completion,total,cost}` to also read the current maps, in legacy→current
precedence, while preserving the invariants:

- **absence-≠-zero** — an absent key in every supported location stays `None`, never `0`; a present
  numeric `0` reads as `Some(0)`.
- **`schema_changed` for genuinely unknown shapes** — empty/absent maps with no legacy fields still
  degrade to `schema_changed` with the affected count surfaced in the import report (`skipped`).

Both the legacy and the current shapes are covered by fixtures in `langfuse/tests.rs`
(`observation_*`, `current_shape_*`).
