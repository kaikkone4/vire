# Docs gate — task-023-csv-adversarial-export-hardening

**Tier:** L2
**Branch:** fix/task-023-csv-adversarial-export-hardening (PR #15)
**Date:** 2026-06-15

## Assessment

TASK-023 is a security hardening bugfix: formula-injection neutralization was already specified and
partially implemented; the fix makes the note input layer stop trimming note content so the
(already-correct) exporter can satisfy the adversarial fidelity contract. No new user-visible
feature, no new configuration, no new export column, no new docs/ content required.

### README.md — test suite description (one phrase added)

The existing line described test coverage as "CSV filtering/escaping/formula neutralization".
TASK-023 adds a new unit test (`clean_opt_preserves_nonempty_value_and_collapses_whitespace_only`)
that covers note-text verbatim preservation — a distinct concern from formula neutralization.
Updated the phrase to "CSV filtering/escaping/formula neutralization/note-text fidelity" to keep
the description accurate.

No other README section required updating: the manual verification step (step 6) covers CSV
export at the right level of detail, and formula neutralization is an internal security control
that does not need a dedicated user-facing callout.

### docs/ — no changes

`docs/langfuse-local-setup.md` and `docs/backup-restore.md` are unrelated to CSV export.
No new operator or user guide section is warranted: the formula-injection neutralization and
note verbatim storage are internal/security hardening behaviors with no operator action required
and no configuration to document.

## Coherence check

| Requirement | README | docs/ | Implementation |
|---|---|---|---|
| Formula injection neutralized (`= + - @` after leading whitespace, leading `\t \r \n`) | ✓ test description | — | `csv_formula_neutralized` (lib.rs:142) |
| Note text preserved verbatim (only empty/all-whitespace → none) | ✓ test description | — | `clean_opt` (lib.rs:60) |
| CSV export stays scoped to reviewed summary columns | — (internal invariant) | — | header unchanged (lib.rs:153) |
| No raw activity / prompt / secret in CSV | — (internal security control) | — | `export_csv_repo` scope unchanged |
| Manual verification step describes CSV export | ✓ step 6 | — | — |

No drift between OpenSpec spec (`specs/csv-export/spec.md`) and implementation detected.
All spec requirements (R1–R4) are covered by existing tests (see QA gate).

## Docs not required at L2 / deferred

- Maintenance runbook (L3 only)
- Compliance/audit docs (L3 only)
- DEC-023 ADR entry — verbatim note storage ratification is a BA feedback item routed through
  Pi-Assistant; out of scope for the SW-6 docs gate
