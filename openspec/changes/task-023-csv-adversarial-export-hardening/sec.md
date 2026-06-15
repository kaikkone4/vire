# Security Review (SW-5) — TASK-023 CSV adversarial export hardening

- **Reviewer:** Security Agent (SW-5)
- **Change:** `task-023-csv-adversarial-export-hardening`
- **Branch:** `fix/task-023-csv-adversarial-export-hardening` · **base:** `main` · **PR:** #15
- **Tier:** L2 · **Gate:** SW-5 security review (parallel with SW-4 Code Review)
- **Date:** 2026-06-15
- **Verdict:** **PASS** — no auto-fail conditions hit per the Tier-1 triage rubric.

---

## Scope of change reviewed

Diff vs `origin/main` (`origin/main...HEAD`) — code change is a single 3-line edit plus tests:

| File | Change |
|------|--------|
| `src-tauri/src/lib.rs` | `clean_opt` now stores note text **verbatim** (collapses only empty/all-whitespace to `None`) instead of trimming; adds two unit assertions (` +SUM(1,2)` neutralize+quote, `clean_opt` preservation). |
| `openspec/changes/.../{arch-review,proposal,spec,tasks}.md` | Docs only. |

No `Cargo.toml` / `Cargo.lock` change, no migration/schema change, no new `#[tauri::command]`, no change to the `invoke_handler` registration list. Confirms: **no dependency/schema changes, no new egress/network/IPC/auth/rate-limit surface.**

---

## Tier-1 scanner results (L2)

| Scanner | Version | Result | Auto-fail? |
|---------|---------|--------|------------|
| **gitleaks** (history) | 8.30.1 | 103 commits scanned — **no leaks** | No |
| **gitleaks** (working tree) | 8.30.1 | 2 hits, both in `src-tauri/target/debug/deps/*.rmeta` | No — see note |
| **OSV-scanner** (`Cargo.lock`, 487 pkgs) | 2.3.8 | 0 Critical, 0 High, 1 Medium (CVSS 6.9), 16 no-CVSS RUSTSEC advisories | No — highest CVSS 6.9 < 7.0 |
| **semgrep** (`p/rust`, `p/security-audit`, `p/secrets`) | 1.165.0 | **0 findings** (150 rules, 21 files) | No |
| **Trivy fs** (vuln+secret, HIGH/CRITICAL) | 0.71.1 | `package-lock.json` 0, `Cargo.lock` 0, 0 secrets | No |

### gitleaks working-tree hits — false positives
Both findings are `generic-api-key` entropy matches inside `src-tauri/target/debug/deps/libmuda-*.rmeta` — compiled Rust incremental-build metadata for the `muda` menu crate. The path is **untracked** (`git ls-files` empty) and **gitignored** (`git check-ignore` confirms). Not source, not committed, not a real secret. The git-history scan (the authoritative path for committed secrets) is clean.

### OSV-scanner advisories — pre-existing baseline, no auto-fail
17 advisories on transitive crates, **none introduced by this PR** (no lockfile change). Only one carries a CVSS: `glib 0.18.5` RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g at **6.9** (below the CVE≥7 L2 threshold; fixed in 0.20.0). The remaining 16 are no-CVSS RUSTSEC **unmaintained / soundness** advisories on the Tauri Linux GTK3 binding stack (`atk`, `gdk*`, `gtk*`), `proc-macro-error`, and the `unic-*` crates. Advisory only — documented, not blocking. (Carry-forward item for a future dependency-refresh task; out of scope for this hardening bugfix.)

---

## Manual secure-code review

### 1. Formula-injection neutralization — correct & complete for the threat model
`csv_formula_neutralized` (`src-tauri/src/lib.rs:142`):
```rust
let trimmed = v.trim_start_matches(|c: char| c.is_whitespace());
if trimmed.starts_with(['=', '+', '-', '@']) || matches!(v.chars().next(), Some('\t'|'\r'|'\n')) {
    format!("'{v}")          // prepend guard to the ORIGINAL value
} else { v.to_string() }
```
- Strips leading Unicode whitespace **before** testing the formula lead — defeats the classic "spreadsheet trims leading space then evaluates" bypass (` +SUM(1,2)` → `'  +SUM…`). Verified by test `csv_escape(" +SUM(1,2)") == "\"' +SUM(1,2)\""`.
- Covers the full OWASP danger set `=`, `+`, `-`, `@`, plus leading `\t` / `\r` / `\n` as raw first char.
- Guard is prepended to the **original** value, so legitimate leading/trailing/internal whitespace and internal CR/LF are preserved (data fidelity per spec) while the cell is rendered inert as text.
- Negative lead (`-`) neutralization does **not** corrupt numeric data: only `project_name` and `note` pass through `csv_escape`; `date`, `start/end_time`, `duration_minutes`, `total_duration_hours` are system-generated/validated and emitted raw (`lib.rs:154`).

### 2. CSV delimiter/quote/newline escaping — applied after neutralization
`csv_escape` (`lib.rs:151`) quotes any cell containing `, " \r \n` and doubles embedded `"`. Order is neutralize → escape, so a `'`-guarded cell that also contains a control char is both guarded and quoted (`csv_escape("\nplain") == "\"'\nplain\""`). Embedded CR/LF stay inside the quoted field — no CSV row/cell injection. Verified by `summaries_and_csv_filtering_escape` (`"A, Inc"`, `"said ""hi"""`).

### 3. Export stays scoped — no raw/secret/column expansion
The CSV header and column set (`lib.rs:153`) are **unchanged** by this PR: `date, project, start_time, end_time, duration_minutes, note, total_duration_hours`. No raw captured activity, app/window logs, AI prompt/response text, terminal command bodies, or secret-shaped values are added. No credential or log value reaches the exporter.

### 4. Verbatim storage introduces no new injection surface
- **SQLi:** notes/names are bound via rusqlite `params![]` (parameterized) — content is data, never concatenated SQL; verbatim whitespace is irrelevant to this.
- **XSS:** every UI render of `project_name` / `note` / ids passes through `escapeHtml` (`src/html.ts`, used as `esc()` throughout `src/main.ts:33-44`), which escapes `& < > ' "` regardless of surrounding whitespace. Preserving leading/trailing whitespace does not add HTML metacharacters, so no stored-XSS regression.
- **Length bound:** `validate_len` still applies to the verbatim value (`MAX_PROJECT_NAME_LEN`/`NOTES`/`ENTRY_NOTE`), so the change is not a bypass of input bounds (if anything marginally stricter, as whitespace now counts toward the limit).

### 5. No new attack surface
No new Tauri command, no addition to `invoke_handler` (`lib.rs:240`), no network/reqwest/IPC/auth/rate-limit code touched. The CSV write path retains its destination validation (`validate_csv_destination`: `.csv` extension required, directory rejected).

---

## Acceptance gate (tests)
`cargo test --lib` → **65 passed, 0 failed**. Relevant cases green:
- `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes` (incl. new ` +SUM(1,2)` case)
- `clean_opt_preserves_nonempty_value_and_collapses_whitespace_only`
- `summaries_and_csv_filtering_escape` (delimiter/quote escaping)

---

## Findings summary

| ID | Severity | Finding | Status |
|----|----------|---------|--------|
| — | — | No auto-fail conditions (no secrets, no CVE≥7, no semgrep ERROR, no HIGH/CRITICAL Trivy) | — |
| SEC-A1 | Advisory | Transitive RUSTSEC unmaintained/soundness advisories (GTK3 stack, `proc-macro-error`, `unic-*`) + `glib` 6.9; pre-existing, not introduced here | Document; defer to a dependency-refresh task |

## Escalations
None. No design-level (trust-boundary / missing-auth) issues identified.

## Verdict
**PASS** → proceed. Hold for SW-4 (Code Review) before SW-6 release per the gate protocol.
