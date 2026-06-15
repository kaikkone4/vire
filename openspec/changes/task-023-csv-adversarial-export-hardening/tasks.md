# Tasks — TASK-023 CSV adversarial export hardening

Single backend/core slice (the Tauri Rust core: note-input normalization + the existing CSV
exporter). Sub-tasks are an implementation sequence within one OpenSpec change, **not** a component
split. All edits land in `src-tauri/src/lib.rs`. Recommended order:

## 1. Stop the input layer from mutating note content

- [x] Change `clean_opt` (`src-tauri/src/lib.rs` ~line 60) to preserve the original non-empty value:
      `fn clean_opt(s: Option<String>) -> Option<String> { s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) }) }`.
      Empty / all-whitespace input still maps to `None`; non-empty input (leading/trailing whitespace
      and internal control characters) is preserved verbatim.
- [x] Confirm shared callers behave: `validate_entry_note` and `validate_project_input` (project
      notes) both route through `clean_opt`. `validate_len` still counts the preserved string. Leave
      `validate_project_input`'s `input.name.trim()` (project name remains a trimmed label — the CSV
      neutralizer is the security control for names; see arch-review §3).

## 2. Confirm the export neutralization contract (already correct — verify, do not rewrite)

- [x] `csv_formula_neutralized`: prefixes `'` when the first non-whitespace char is `= + - @`, or the
      first raw char is `\t \r \n`; the guard is prepended to the **original** `v` (whitespace
      preserved). No change required.
- [x] `csv_escape`: quotes any cell containing `, " \r \n` and doubles embedded `"`, applied to the
      neutralized value. No change required.
- [x] Neutralization is applied to the two user-controlled text cells (`project_name`, `note`) and not
      to date/time/numeric columns. No change required.

## 3. Lock the contract with focused unit coverage

- [x] Add a `clean_opt` unit assertion: non-empty value with leading whitespace is preserved
      (`Some(" +SUM")`), and whitespace-only / empty input collapses to `None`.
- [x] Extend `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes` (or add a sibling
      test) with `assert_eq!(csv_escape(" +SUM(1,2)"), "\"' +SUM(1,2)\"")`.

## 4. No export-scope expansion (guardrail, assert don't assume)

- [x] Export columns remain exactly `date, project, start_time, end_time, duration_minutes, note,
      total_duration_hours`. No raw activity/app-window log, AI prompt/response, command body, or
      secret-shaped field is added. No new CSV library.

## 5. Verification

- [x] `cargo test --test adversarial csv_export_neutralizes_formula_like_project_names_and_notes`
      passes (the pre-existing failing acceptance gate).
- [x] `cargo test` (full backend suite) — no regression, including `summaries_and_csv_filtering_escape`,
      `rejects_overlong_text_fields`, `project_crud_archive_and_active_filter`, and the new unit cases.
- [x] `cargo clippy --all-targets` clean (no new warnings).
- [x] `openspec validate task-023-csv-adversarial-export-hardening --strict`.

## Out of scope (do not build here)

- Review/approval UI (TASK-009); summary/approval model and export records (TASK-010); CSV export of
  reconciliation (TASK-010); macOS capture (TASK-005); classification (TASK-008); any new export
  column or detailed/raw export mode; any new CSV/serialization dependency.
