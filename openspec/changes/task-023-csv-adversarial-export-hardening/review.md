# Code Review — TASK-023 CSV Adversarial Export Hardening

- **Reviewer:** SW-4 (Code Reviewer)
- **Branch:** `fix/task-023-csv-adversarial-export-hardening`
- **PR:** #15
- **Tier:** L2
- **Date:** 2026-06-15
- **Verdict:** **PASS**

---

## Verdict

**PASS.** No blocking issues. One non-blocking suggestion. No scope creep detected. No
dead code, no unused imports, no commented-out code introduced.

---

## Reviewed locations

| File | Lines | Topic |
|------|-------|-------|
| `src-tauri/src/lib.rs` | 60 | `clean_opt` — note preservation fix |
| `src-tauri/src/lib.rs` | 62 | `validate_project_input` — project name trim unchanged |
| `src-tauri/src/lib.rs` | 142–150 | `csv_formula_neutralized` |
| `src-tauri/src/lib.rs` | 151 | `csv_escape` |
| `src-tauri/src/lib.rs` | 153–154 | `export_csv_repo` — header and column set |
| `src-tauri/src/lib.rs` | 255–256 | New unit tests |
| `src-tauri/tests/adversarial.rs` | 97–131 | Adversarial integration test |

---

## Blocking issues

None.

---

## Findings (non-blocking)

### S-1 — `csv_formula_neutralized`: the `matches!` arm is unreachable for all practical inputs

**File:** `src-tauri/src/lib.rs:145`

```rust
if trimmed.starts_with(['=', '+', '-', '@']) || matches!(v.chars().next(), Some('\t' | '\r' | '\n')) {
```

`trimmed` is produced by `v.trim_start_matches(|c: char| c.is_whitespace())`.
`char::is_whitespace()` returns `true` for `'\t'`, `'\r'`, and `'\n'`, so any string
that starts with those bytes will have them stripped into `trimmed`. Therefore
`trimmed.starts_with(…)` already fires (or does not) without the `matches!` arm ever
adding coverage — the `matches!` branch on `v.chars().next()` can only be `true` when
the first character of `v` is a raw control character *and* the trimmed form does not
start with `= + - @`. That combination does not arise in practice and is untested.

**Impact:** zero — the overall guard is sound (the `||` makes the neutralization
conservative). The line comment (`// Spreadsheet apps can interpret…`) accurately
describes intent. This is a minor clarity issue, not a correctness bug.

**Suggestion:** remove the `matches!` branch; the `trimmed.starts_with(…)` check alone
is sufficient and self-documenting:

```rust
if trimmed.starts_with(['=', '+', '-', '@']) {
    format!("'{v}")
} else {
    v.to_string()
}
```

No behavior change; shrinks the surface area of the guard.

---

## Convention / style pass

- One-liner `clean_opt` at `lib.rs:60`: consistent with surrounding helper style.
- `validate_project_input` at `lib.rs:62` remains a single dense line; no change in
  character from the pre-existing file style. Not introduced by this PR.
- No new `unwrap()` calls in production paths.
- No `console.log` / `eprintln!` / `dbg!` left in.
- No `#[allow(dead_code)]` or commented-out code added.

---

## Scope check

- Export header (`lib.rs:153`): `date,project,start_time,end_time,duration_minutes,note,total_duration_hours` — 7 columns, identical to main. No new column added.
- No review/approval UI files modified.
- No summary or approval model, reconciliation CSV, or export-record table touched.
- No new Cargo dependency (`Cargo.toml`/`Cargo.lock` unchanged for this change).
- No IPC schema change, no `tauri.conf.json` change, no frontend files changed.

Scope is tightly contained to the stated fix.

---

## Test quality

**`lib.rs:255` — `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes`**

Covers `=`, `@`, `\n` (control prefix), internal `\r` (not a prefix → quoted only),
and the critical `" +SUM(1,2)"` (leading whitespace before `+`). Assertions are precise
string comparisons; no loose `contains` checks.

**`lib.rs:256` — `clean_opt_preserves_nonempty_value_and_collapses_whitespace_only`**

Covers preserve (leading/trailing whitespace retained), collapse (all-whitespace → `None`),
empty-string → `None`, and `None` passthrough. Five cases, all exact equality.

**`adversarial.rs:97–131` — `csv_export_neutralizes_formula_like_project_names_and_notes`**

End-to-end: creates real project + entry through public repo functions, exports to a
temp file, and asserts on the emitted CSV bytes. Covers the previously failing
acceptance gate (formula prefix on project name, `' +SUM` prefix on note preserving
leading whitespace, bare `\r` surviving inside a quoted field, and absence of raw
formula strings). Four assertions on a single export; each targets a distinct
requirement.

Test quality is good. The adversarial integration test is the strongest signal — it
exercises the full stack (create → store → export → read file).

---

## Escalations

None. No architectural concerns.
