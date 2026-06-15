# TASK-023 — CSV adversarial export hardening (formula-injection neutralization fidelity)

## Why

The local CSV exporter (FR-016) is the application's **export boundary** (BA `11_security_review.md`
Boundary E / SEC-006): a CSV leaves Vire and may be opened in spreadsheet software, so
formula/CSV-injection neutralization of user-controlled text is mandatory
(`03_architecture_plan.md` line 93 "support formula-neutralized text"; `04_technical_plan.md` §10
"CSV formula neutralization is mandatory"). The repo already ships an adversarial regression test
for exactly this — `src-tauri/tests/adversarial.rs::csv_export_neutralizes_formula_like_project_names_and_notes`
— and it **fails on clean `main`**, a pre-existing non-blocker carried since before TASK-022.

Root cause (diagnosed, not guessed):

- The note input is ` +SUM(1,2) with bare\rcarriage return` (leading space, internal bare CR).
- Captured export bytes (`od -c` on the live test): the project name is correctly neutralized
  (`"'=WEBSERVICE(""…"")"`), the internal CR **is** preserved and forces quoting, but the note's
  **leading space is stripped**, producing `"'+SUM(1,2) with bare\rcarriage return"` instead of the
  required `"' +SUM(1,2) with bare\rcarriage return"`.
- The stripping happens at the **input layer**, not the exporter: `clean_opt` (`src-tauri/src/lib.rs`
  line 60) runs `v.trim()` on note content via `validate_entry_note`, mutating the stored value. The
  export-side `csv_formula_neutralized` / `csv_escape` are already correct — they prepend the `'`
  guard to the original value, keep internal control chars, and quote on `, " \r \n`.

So the cell can never satisfy the adversarial contract because the leading whitespace is destroyed
before export. The exporter cannot re-insert it. The fix is to stop the input layer from mutating
legitimate note content — aligning with the contract requirement to *preserve legitimate text as much
as possible* — while leaving the (already-correct) export neutralization in place.

This is a **bugfix that implements existing BA architecture** (SEC-006, FR-016, arch-plan §93,
tech-plan §10). It does **not** reopen any decision, does **not** add export fields, and does **not**
touch review UI or the summary/approval model.

## What Changes

- **Stop mutating note content at input (1-line correction in `clean_opt`).** `clean_opt` SHALL detect
  emptiness via `trim()` but persist the **original** non-empty value verbatim, instead of storing the
  trimmed value:
  `s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })`.
  Empty / all-whitespace input still collapses to `None`; non-empty input (including leading/trailing
  whitespace and internal control characters) is preserved so it reaches the exporter intact. This is
  shared by entry notes and project notes; both gain faithful preservation.
- **Keep the export neutralization contract explicit and verified (no behavior change required).**
  `csv_formula_neutralized` prefixes a cell with `'` when its first non-whitespace character is `=`,
  `+`, `-`, or `@`, or when its first character is a tab, CR, or LF; the guard is prepended to the
  **original** value (whitespace preserved). `csv_escape` then wraps any cell containing `,`, `"`,
  `\r`, or `\n` in double-quotes and doubles embedded `"`. This already passes for the project name and
  for internal control chars — the change makes it pass for the note's leading whitespace too.
- **Lock the contract with focused unit coverage.** Add small `#[cfg(test)]` assertions: (a) `clean_opt`
  preserves a non-empty value with leading whitespace and collapses whitespace-only to `None`; (b) a
  `csv_escape(" +SUM(1,2)")` case asserting `"' +SUM(1,2)"`. The existing adversarial integration test
  is the acceptance gate.
- **No export-scope expansion (guardrail, asserted).** The CSV continues to emit only the reviewed
  summary columns (`date, project, start_time, end_time, duration_minutes, note, total_duration_hours`).
  No raw captured activity, app/window log, AI prompt/response text, command body, or secret-shaped
  field is added (SEC-006; task contract). Numeric/date columns are not passed through the text-cell
  neutralizer and are unaffected.

## Impact

- **Affected specs:** `csv-export` (new capability, ADDED). Codifies the formula-injection
  neutralization set (`= + - @` after leading-whitespace, plus leading `\t \r \n`), the
  preserve-legitimate-text fidelity rule (guard prepended to the original value; internal CR/LF
  retained and force quoting), CSV delimiter/quote/newline escaping order, and the no-raw-export
  guardrail.
- **Affected code (product runtime):** `src-tauri/src/lib.rs` only — `clean_opt` (input
  normalization), plus unit assertions in the existing `#[cfg(test)]` module; the existing
  adversarial test in `src-tauri/tests/adversarial.rs` is the acceptance gate (no test edit required).
  No UI, no IPC, no schema, no `tauri.conf.json` change. No network.
- **Neutralization set vs BA SEC-006:** BA lists `= + - @ tab CR`; the implementation also neutralizes
  leading LF (`\n`) — a strict **superset**, privacy/security-positive, consistent with the technical
  plan's non-exhaustive "such as" phrasing. No conflict, no relaxation.
- **Out of scope (clean boundaries):** review/approval UI (TASK-009), summary/approval model and
  export records (TASK-010), CSV export of reconciliation (TASK-010), macOS capture (TASK-005),
  classification (TASK-008), any new export column, and any new CSV library. None of their component
  boundaries are crossed.
- **`feedback_to_ba[]` (non-blocking):** one item — ratify that **entry/project notes are stored
  verbatim** (only empty/all-whitespace collapses to "no note"), i.e. input no longer silently trims
  note whitespace. This is the behavior the adversarial test mandates and aligns with "preserve
  legitimate text," but it is a small input-semantics decision worth recording (proposed ADR DEC-023).
- **Branch:** `fix/task-023-csv-adversarial-export-hardening`, base `main`.
