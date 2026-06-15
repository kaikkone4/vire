# Architecture Review (SW-1) — TASK-023 CSV adversarial export hardening

- **Reviewer:** SW Architect (SW-1)
- **Change:** `task-023-csv-adversarial-export-hardening`
- **Branch (proposed):** `fix/task-023-csv-adversarial-export-hardening` · **base:** `main`
- **Tier:** L2 · **Gate context:** SW-1 task-design review before the backend developer implements the
  CSV adversarial-export hardening bugfix. Recommended next small slice after TASK-022 merged.
- **Date:** 2026-06-15
- **Verdict:** **PASS** — one cohesive backend/core slice (Tauri Rust core: note-input normalization
  + the existing CSV exporter), all in `src-tauri/src/lib.rs`. No component boundary is crossed;
  **not split-required**. No BA escalation required (FR-016, SEC-006, arch-plan §93, tech-plan §10
  already establish the contract). One non-blocking `feedback_to_ba[]` item (§7).

---

## 1. Inputs read

- BA requirements: `01_requirements.md` FR-016 (local CSV export of reviewed summaries, no cloud sync).
- BA architecture: `03_architecture_plan.md` line 93 (Exporter "support formula-neutralized text and
  redacted details"), Boundary/component model.
- BA security review: `11_security_review.md` Boundary E (export boundary — OWASP CSV/formula
  injection + privacy), **SEC-006** (CSV export privacy & formula injection; control = neutralize
  formula-like prefixes `= + - @ tab CR`, escape delimiters/quotes/newlines, no broadened export
  defaults; verification = unit tests for neutralization/escaping + adversarial spreadsheet test +
  integration test confirming raw logs absent by default).
- BA technical plan: `04_technical_plan.md` §10 (CSV export rules; "CSV formula neutralization is
  mandatory"; line 282 neutralize `= + - @ tab CR`), §12 (CSV escaping/neutralization/column stability
  test focus; adversarial spreadsheet test).
- Current code: `src-tauri/src/lib.rs` — `clean_opt` (line 60), `validate_entry_note` (63),
  `validate_project_input` (62), `csv_formula_neutralized` (142), `csv_escape` (151),
  `export_csv_repo` (152), unit test `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes`
  (255), `summaries_and_csv_filtering_escape` (250).
- Failing acceptance test: `src-tauri/tests/adversarial.rs::csv_export_neutralizes_formula_like_project_names_and_notes`
  (line 97).

## 2. Defect diagnosis (evidence-based, not assumed)

Ran the failing test and captured the exported row with `od -c` (terminal rendering of bare CR is
misleading; raw bytes are authoritative):

```
2026-03-14,"'=WEBSERVICE(""https://example.invalid/"")",09:00,10:00,60,"'+SUM(1,2) with bare\rcarriage return",1.00
```

Comparing to the test's required note cell `"' +SUM(1,2) with bare\rcarriage return"`:

- Project name: **correct** — neutralized (`'=`) and quoted; no executable `,=WEBSERVICE` cell.
  Assertions 1 and 3 already pass.
- Note internal bare CR: **preserved** and forces quoting — correct.
- Note leading space: **stripped** — exported `'+SUM…` instead of `' +SUM…`. This single discrepancy
  fails assertion 2.

Root cause is the **input layer, not the exporter**. `clean_opt` (line 60) does
`v.trim().to_string()` and stores the trimmed value via `validate_entry_note`, so the note's leading
whitespace is destroyed before it ever reaches `export_csv_repo`. The exporter's
`csv_formula_neutralized` already prepends the `'` guard to the *original* value (whitespace
preserved) and `csv_escape` already quotes on `, " \r \n` — both correct. The exporter cannot
re-insert whitespace the input layer deleted. Therefore the fix must restore input fidelity, not
change the neutralizer.

## 3. Architecture-consistency findings

The slice implements the existing BA export contract; three findings shape the fix:

1. **Fix the input mutation, keep the export neutralizer.** The minimal correct change is a 1-line
   `clean_opt` correction — detect emptiness via `trim()` but persist the original non-empty value:
   `s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })`. This directly satisfies the
   contract requirement to *preserve legitimate text as much as possible* and makes the note's leading
   whitespace survive to the (already-correct) exporter. No exporter rewrite is warranted.
2. **Project name stays a trimmed label; the neutralizer is its security control.**
   `validate_project_input` trims the name (`input.name.trim()`) — a label-normalization choice, not a
   security gap. A name like `"  =SUM"` trims to `"=SUM"` and is then neutralized to `"'=SUM"` on
   export. The adversarial test's name has no surrounding whitespace, so no name change is needed.
   Leaving name-trimming intact keeps the change minimal and avoids altering label semantics.
3. **Neutralization set is a strict superset of SEC-006, intentionally.** SEC-006/tech-plan §10 list
   `= + - @ tab CR`; the implementation also neutralizes leading LF (`\n`) and handles leading
   whitespace before formula detection (spreadsheets strip leading spaces first). This is
   security-positive and consistent with the plan's non-exhaustive "such as" phrasing — not a
   relaxation. The spec codifies the full set so future reviewers do not read the extra coverage as
   drift.

## 4. Split analysis — one slice, not split-required

Per the role rule, "split-required" means scope crosses component boundaries in
`03_architecture_plan.md`. It does not. Every piece is within the single Tauri Rust core, and the two
logical concerns it touches (note-input validation in the Store path, and the CSV Exporter) are both
owned by that core and edited in the same file with no new cross-component contract.

| Listed piece | Component | Boundary crossing? |
| --- | --- | --- |
| `clean_opt` note/notes preservation | Rust core — input normalization (Store path) | No |
| `csv_formula_neutralized` / `csv_escape` (verify) | Rust core — CSV Exporter | No (no change) |
| Unit + adversarial test coverage | Rust core tests | No |
| No export-field expansion (guardrail) | Cross-cutting invariant (SEC-006), not a component | No |

The review UI (TASK-009), summary/approval model and export records (TASK-010), macOS capture
(TASK-005), and classification (TASK-008) are untouched. No new export column, mode, or CSV library.
**Verdict: one cohesive bugfix slice**, internal sequence in `tasks.md`, not a split.

## 5. Security/correctness contract (the exact contract per task ask)

| # | Contract clause | Where enforced |
| --- | --- | --- |
| 1 | Formula injection neutralized for project names, notes, and any other exported user-controlled text field | `csv_formula_neutralized` applied to `project_name` + `note` cells (spec R1) |
| 2 | Leading `= + - @` (after stripping leading whitespace) neutralized with `'` guard | `csv_formula_neutralized` trims leading whitespace then matches the set (spec R1) |
| 3 | Leading tab, CR, LF neutralized | `csv_formula_neutralized` first-char `\t \r \n` check (spec R1) |
| 4 | Legitimate text preserved as much as possible (guard prepended to original; leading/trailing whitespace and internal chars retained) | `clean_opt` preserves original non-empty value; guard prepends to original `v` (spec R2) |
| 5 | Internal CR/LF retained and force CSV quoting; delimiters/quotes escaped after neutralization | `csv_escape` quotes on `, " \r \n`, doubles `"` (spec R3) |
| 6 | Whitespace edge cases consistent (empty/all-whitespace → no note; non-empty preserved) | `clean_opt` empty-after-trim → `None` (spec R2) |
| 7 | No raw activity / prompt / secret export expansion | Export columns unchanged; guardrail asserted (spec R4) |
| 8 | No unrelated review UI or summary-model scope | Out-of-scope list in `tasks.md`; nothing outside `lib.rs` touched |

## 6. Empirical facts the developer must honor

- Only two exported cells are user-controlled free text: `project_name` and `note`. Both already pass
  through `csv_escape`. Date/time columns are fixed-format; durations are numeric — do **not** route
  them through the text neutralizer.
- The guard is `'` prepended to the **original** value; internal control characters are retained and
  trigger quoting. Do not "sanitize" by deleting control characters — the adversarial test asserts the
  internal CR survives inside the quoted field.
- `clean_opt` is shared by entry notes and project notes; the fix preserves both. Project notes are
  not CSV-exported but faithful preservation is consistent and harmless.
- Existing tests that must stay green: `summaries_and_csv_filtering_escape` (comma/quote escaping),
  `rejects_overlong_text_fields` (length limits count the preserved string),
  `project_crud_archive_and_active_filter` (creates `" Alpha "` but only asserts the later `"Beta"`, so
  the preservation change does not break it), and
  `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes`.

## 7. ADR + open items

**Proposed ADR DEC-023 (record in BA decision log on ratification): notes are stored verbatim.**
Entry and project note content is persisted exactly as entered; only empty or all-whitespace input
collapses to "no note". Rationale: the export boundary's adversarial fidelity contract (SEC-006)
requires user text — including leading whitespace and internal control characters — to survive to the
CSV cell where it is guarded and quoted, and silently trimming user notes contradicts "preserve
legitimate text as much as possible". Scope: note text only; project **name** remains a trimmed label.

**`feedback_to_ba[]` (non-blocking, route to ba-architect via Pi-Assistant):** confirm DEC-023 — that
input no longer trims note whitespace (empty/all-whitespace still → none). This is the behavior the
adversarial test mandates; it changes a minor input-normalization detail and is worth a recorded
decision. Does not block developer start.

## 8. Recommendation — next role and branch

- **Change name:** `task-023-csv-adversarial-export-hardening` (this dir).
- **Branch:** `fix/task-023-csv-adversarial-export-hardening`, base `main`.
- **Next role (primary):** **backend-developer (Rust/Tauri)** — apply the `clean_opt` correction, add
  the two focused unit assertions, and run the adversarial + full backend suite. No frontend,
  integration, data-migration, or devops role is required (no UI, IPC, schema, or config change).
- **Then:** SW-4 (code review), SW-5 (security — SEC-006 neutralization/escaping + no-raw-export
  guardrail), SW-6 (release). SW-3/QA is light (the adversarial test is the acceptance gate).

## 9. Verdict

**PASS.** TASK-023 is one cohesive backend/core bugfix slice inside the Tauri Rust core — not
split-required, no BA escalation. It implements the existing CSV export contract (FR-016, SEC-006,
arch-plan §93, tech-plan §10): the failing adversarial test is caused by input-layer note trimming
(`clean_opt`), and the fix restores note fidelity so the already-correct export neutralizer satisfies
the contract, with no export-field expansion and no UI/summary-model scope. Deliverables
(`proposal.md`, `tasks.md`, `specs/csv-export/spec.md`, this review) are in place. One non-blocking
`feedback_to_ba[]` (DEC-023, verbatim note storage). Route to backend-developer (Rust/Tauri) on
`fix/task-023-csv-adversarial-export-hardening`.
