# QA Report — TASK-023 CSV adversarial export hardening

- **QA Engineer:** SW-3
- **Branch:** `fix/task-023-csv-adversarial-export-hardening`
- **Tier:** L2
- **Date:** 2026-06-15
- **Verdict:** **PASS**

---

## 1. OpenSpec validation

```
openspec validate task-023-csv-adversarial-export-hardening --strict
→ Change 'task-023-csv-adversarial-export-hardening' is valid
```

All four change artifacts (`proposal.md`, `tasks.md`, `specs/csv-export/spec.md`,
`arch-review.md`) are present and structurally valid.

---

## 2. Test results

### Targeted adversarial acceptance gate

```
cargo test --test adversarial csv_export_neutralizes_formula_like_project_names_and_notes
→ test csv_export_neutralizes_formula_like_project_names_and_notes ... ok
→ test result: ok. 1 passed; 0 failed
```

This was the pre-existing failing test that is the acceptance gate for this bugfix. It now passes.

### Full backend suite

```
cargo test
→ test result: ok. 65 passed; 0 failed; 0 ignored   (unit tests)
→ test result: ok. 3 passed; 0 failed; 0 ignored     (adversarial integration tests)
→ test result: ok. 0 passed; 0 failed; 0 ignored     (doc-tests)
```

No regression in any previously passing test.

### Clippy

```
cargo clippy --all-targets
→ 4 warnings (no errors)
```

All 4 warnings are **pre-existing**, confirmed by diff inspection:
- 3 warnings in `src/langfuse/importer.rs` (lines 182, 306, 316) — `map_or`/`manual_flatten` style
  suggestions; this file was not touched by TASK-023.
- 1 warning in `src/lib.rs` line 237 (`io_other_error`) — in `db_path`, not touched by TASK-023.

No new warnings introduced.

---

## 3. Scenario coverage matrix

| # | Spec requirement / scenario | Covered by | Result |
|---|---|---|---|
| S1 | Formula-like project name neutralized (`=WEBSERVICE…` → `'=WEBSERVICE…`) | `adversarial.rs:127` | PASS |
| S2 | Formula-like note after leading whitespace neutralized (` +SUM(1,2)…` → `"' +SUM(1,2)…"`) | `adversarial.rs:128` | PASS |
| S3 | Raw formula not emitted as executable CSV cell (`,=WEBSERVICE` absent) | `adversarial.rs:129` | PASS |
| S4 | Note formula not emitted as executable CSV cell (`,+SUM` absent) | `adversarial.rs:130` | PASS |
| S5 | Leading `= + - @` (after stripping leading whitespace) neutralized with `'` | `lib.rs:255` `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes` | PASS |
| S6 | Leading tab / CR / LF neutralized | `lib.rs:255` (`\nplain` → `"'\nplain"`) | PASS |
| S7 | Leading whitespace preserved in exported cell (`' +SUM` not `'+SUM`) | `adversarial.rs:128` + `lib.rs:255` (` +SUM(1,2)` → `"' +SUM(1,2)"`) | PASS |
| S8 | Internal CR/LF retained inside quoted field | `adversarial.rs:128` (bare `\r` survives inside `"…"`) | PASS |
| S9 | Empty/all-whitespace note stored as `None` | `lib.rs:256` `clean_opt_preserves_nonempty_value_and_collapses_whitespace_only` | PASS |
| S10 | Non-empty note preserved verbatim (leading/trailing whitespace retained) | `lib.rs:256` (`" +SUM"` → `Some(" +SUM")`, `"  done  "` → `Some("  done  ")`) | PASS |
| S11 | CSV delimiter/quote escaping applied after neutralization (`,` → quoted; `"` → doubled) | `lib.rs:250` `summaries_and_csv_filtering_escape` | PASS |
| S12 | Neutralized cell containing control char is both guarded and quoted | `lib.rs:255` (`\nplain` → `"'\nplain"`) + `adversarial.rs:128` | PASS |
| S13 | No raw activity / AI prompt / secret field in export | Header `date,project,start_time,end_time,duration_minutes,note,total_duration_hours` (lib.rs:153) — no new column | PASS |
| S14 | Numeric/date columns NOT passed through text neutralizer | `export_csv_repo` (lib.rs:154) uses `csv_escape` only for `project_name` and `note` | PASS |
| S15 | Project names remain trimmed labels; neutralizer is the security control | `validate_project_input` `input.name.trim()` preserved; adversarial name has no surrounding whitespace — unaffected | PASS |

---

## 4. Code correctness verification

**`clean_opt` fix (lib.rs:60):**

```rust
// Before (main): stores trimmed value
fn clean_opt(s: Option<String>) -> Option<String> {
    s.and_then(|v| { let t = v.trim().to_string(); if t.is_empty() { None } else { Some(t) } })
}

// After (this branch): stores original value
fn clean_opt(s: Option<String>) -> Option<String> {
    s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })
}
```

Exactly one line changed; the fix is minimal and precise. Both shared callers
(`validate_entry_note`, `validate_project_input` project notes) gain faithful preservation.
`validate_project_input`'s `input.name.trim()` for the project label is correctly left intact.

**`csv_formula_neutralized` (lib.rs:143–149):** No change. Guards prepend `'` to
**original** `v` (leading whitespace preserved); already correct.

**`csv_escape` (lib.rs:151):** No change. Quotes on `, \n \r "`, doubles `"`; already correct.

**Export header (lib.rs:153):** `date,project,start_time,end_time,duration_minutes,note,total_duration_hours` — 7 columns, unchanged.

**New unit tests added (lib.rs:255–256):**

- `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes` extended with
  `assert_eq!(csv_escape(" +SUM(1,2)"), "\"' +SUM(1,2)\"")` — locks the leading-whitespace-before-formula path.
- `clean_opt_preserves_nonempty_value_and_collapses_whitespace_only` — new; covers preserve and collapse branches.

---

## 5. Out-of-scope guardrail

- No review/approval UI touched (TASK-009 scope).
- No summary/approval model, export records, or reconciliation CSV touched (TASK-010 scope).
- No macOS capture (TASK-005), no classification (TASK-008).
- No new CSV library or dependency added (`Cargo.toml`/`Cargo.lock` not modified for this change).
- No IPC schema change, no `tauri.conf.json` change, no frontend files changed.

---

## 6. Non-blockers

- **4 pre-existing clippy warnings** in `langfuse/importer.rs` and `db_path` (`lib.rs:237`).
  Confirmed pre-existing; TASK-023 introduced zero new warnings.
- **DEC-023 BA feedback item** (verbatim note storage ratification) noted in `arch-review.md §7`
  as non-blocking; it does not affect the gate verdict.

---

## 7. Gate verdict

**PASS.** All spec scenarios have observable test coverage. The pre-existing failing adversarial
acceptance test (`csv_export_neutralizes_formula_like_project_names_and_notes`) now passes. Full
backend suite (65 unit + 3 integration) passes with zero failures. OpenSpec strict validation
clean. Clippy warnings are all pre-existing. No scope expansion confirmed.

Route to SW-4 (code reviewer) and SW-5 (security agent) in parallel.
