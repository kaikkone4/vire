# Release — TASK-023 CSV Adversarial Export Hardening

- **Gate:** SW-6 (Release Manager) · **Tier:** L2
- **Change:** `task-023-csv-adversarial-export-hardening`
- **Branch:** `fix/task-023-csv-adversarial-export-hardening` · **PR:** [kaikkonen4/vire#15](https://github.com/kaikkonen4/vire/pull/15)
- **Release unit:** Bugfix — `clean_opt` note-preservation fix restoring formula-injection neutralization fidelity
- **Date:** 2026-06-15
- **Product version baseline:** `0.1.0` (from `src-tauri/Cargo.toml`) · **Release tag:** `task-023/v0.1.1` (PATCH bugfix)
- **Verdict:** RELEASE-READY (all three required declarations complete; signed tag deferred per L2 key policy)
- **Gate chain:** SW-3 PASS (`a3d1a6a`) · SW-4 PASS (`63f5e10`) · SW-5 PASS (`63f5e10`) · SW-6 Docs PASS (`3730a50`) · SW-6 Release PASS (`274f71e`)

---

## Gate prerequisites

| Gate | Role | Verdict | Artifact | Commit audited |
|------|------|---------|----------|----------------|
| SW-3 | QA Engineer | **PASS** | `qa.md` | `a3d1a6a` (+ `63f5e10` docs) |
| SW-4 | Code Reviewer | **PASS** | `review.md` | `63f5e10` |
| SW-5 | Security Agent | **PASS** | `sec.md` | `63f5e10` |
| SW-6 Docs | Documentation Engineer | **PASS** | `docs.md` | `3730a50` |

SW-4 code review artifact committed at `63f5e10`. SW-3 QA and SW-5 security artifacts present in change directory. SW-6 L2 docs gate committed at `3730a50`: README.md test description updated to include note-text fidelity coverage; `docs/` required no changes (formula neutralization and note verbatim storage are internal/security hardening behaviors with no operator action required).

---

## Release contents

### What this bugfix fixes

The existing adversarial acceptance test
`csv_export_neutralizes_formula_like_project_names_and_notes` (`src-tauri/tests/adversarial.rs`)
was failing on clean `main`. The test requires that a note value ` +SUM(1,2) with bare\rcarriage
return` is exported as `"' +SUM(1,2) with bare\rcarriage return"` — i.e., the leading space is
preserved inside the `'`-guarded, quoted CSV cell.

Root cause: `clean_opt` (`src-tauri/src/lib.rs:60`) called `v.trim()` before storing note values,
which destroyed legitimate leading whitespace before the correct export-side guard
(`csv_formula_neutralized`) could see it. The note ` +SUM(1,2) …` became `+SUM(1,2) …` in the
database, so the adversarial cell contract could never be satisfied.

The export layer (`csv_formula_neutralized`, `csv_escape`) was already correct and is unchanged.

### Runtime changes (single commit `a3d1a6a`)

| File | Change |
|------|--------|
| `src-tauri/src/lib.rs:60` | `clean_opt` — one-line fix: detects empty/all-whitespace via `v.trim().is_empty()` but persists the **original** non-empty value verbatim (`Some(v)`) instead of the trimmed form. Empty/all-whitespace input still collapses to `None`. |
| `src-tauri/src/lib.rs:255–256` | Two new focused unit tests: `csv_cells_neutralize_formula_prefixes_and_escape_control_prefixes` (extended with `" +SUM(1,2)"` → `"\"' +SUM(1,2)\""` case) and `clean_opt_preserves_nonempty_value_and_collapses_whitespace_only` (new; five cases). |

### No schema changes, no new IPC commands, no frontend changes, no dependency changes

`projects`, `time_entries` table definitions unchanged. `export_csv` and all other Tauri command
signatures unchanged. `src/main.ts`, `tauri.conf.json`, `capabilities/`, `Cargo.toml`, and
`Cargo.lock` untouched. The fix is strictly internal to the Rust core input normalisation layer.

### Test result

```
cargo test  →  65 unit + 3 adversarial  →  68 passed, 0 failed
cargo test --test adversarial csv_export_neutralizes_formula_like_project_names_and_notes  →  PASS (was failing on main)
cargo clippy --all-targets  →  4 warnings, all pre-existing (langfuse/importer.rs, lib.rs:237)
```

---

## Required declaration 1 — Deployment size

**`patch` (correctness bugfix; no breaking changes; no new user-visible surface).**

- Single 1-line edit to `clean_opt`; no new helper, no renamed function
- No new Tauri IPC command, no new DB table or column, no schema migration
- No new dependency added, removed, or version-bumped
- No frontend file, no IPC contract, no `tauri.conf.json` changed
- `src-tauri/Cargo.toml` version string remains `0.1.0`; release tracked by tag `task-023/v0.1.1`
- Behavioral delta: entry and project notes that previously had leading/trailing whitespace trimmed
  at input will now be stored verbatim. Empty/all-whitespace notes still collapse to `None`.
  This aligns with the BA-mandated contract (DEC-023 pending formal ratification).

---

## Required declaration 2 — Rollback strategy

**`partial-automated`.**

| Step | Action | Mode |
|------|--------|------|
| 1 | `git revert a3d1a6a` on `main` post-merge | Automated |
| 2 | `cargo test` — verify 65 unit + 2 adversarial pass (the formula-note adversarial test will re-fail, which is expected) | Automated |
| 3 | Tag reverted state `task-023-rollback/vX` | Manual |
| 4 | Re-deploy app binary | Manual (no migration required; DB schema unchanged) |

**Data consideration:** Notes stored verbatim after this fix merges will retain their original
whitespace in the database. Rolling back `clean_opt` affects only **new** note writes (future
inputs will again be trimmed). Existing verbatim notes remain in the DB as-is and are still
handled correctly by `csv_formula_neutralized` + `csv_escape` (the export layer is
whitespace-neutral by design). No data migration is required for rollback.

**Staging drill:** `cargo test` suite is the staging gate for this change. No Docker/external
stack required. Recommend running the adversarial suite on a developer workstation before and
after rollback to confirm the pre-fix state is restored.

---

## Required declaration 3 — Component compatibility matrix

No dependency was added, removed, or version-bumped in this change. The matrix below reflects the
full runtime dependency envelope inherited from the branch tip.

### Rust core (`src-tauri/Cargo.toml`)

| Crate | Specified | Tested | Notes |
|-------|-----------|--------|-------|
| `tauri` | `^2.2` | 2.2.x | Tauri v2; no v1 compatibility |
| `tauri-plugin-dialog` | `^2.2` | 2.2.x | Paired with tauri ^2.2 |
| `rusqlite` | `^0.32` | 0.32.x | Bundled SQLite; no external SQLite required |
| `serde` | `^1` | 1.x | `derive` feature |
| `serde_json` | `^1` | 1.x | — |
| `chrono` | `^0.4` | 0.4.x | `serde`, `clock` features |
| `uuid` | `^1` | 1.x | `v4` feature |
| `thiserror` | `^2` | 2.x | — |
| `reqwest` | `^0.12` | 0.12.x | `blocking`, `json`, `rustls-tls`; no default-features |
| `url` | `^2` | 2.x | — |

### Frontend (`package.json`)

| Package | Specified | Tested | Notes |
|---------|-----------|--------|-------|
| `@tauri-apps/api` | `^2.2.0` | 2.2.x | — |
| `@tauri-apps/plugin-dialog` | `^2.2.0` | 2.2.x | — |

### Build toolchain

| Tool | Min | Tested |
|------|-----|--------|
| Rust / Cargo | 1.70 (edition 2021 min) | 1.95.0 |
| Node | 18 LTS | 26.3.0 |
| `@tauri-apps/cli` | `^2.2.0` | 2.2.x |

**No upstream/downstream service compatibility concerns.** Vire is a local-first desktop app;
there are no network API versions to declare. The only external contract (Langfuse REST) is
unchanged by this task.

---

## Non-blocking advisories rollup (SW-3 / SW-4 / SW-5)

These items are carried forward for future tasks; none affects the SW-6 gate verdict.

| ID | Source | Severity | Finding | Disposition |
|----|--------|----------|---------|-------------|
| NB-1 | SW-4 review | Style | `csv_formula_neutralized` (lib.rs:145): `matches!(v.chars().next(), Some('\t'|'\r'|'\n'))` arm is unreachable — `trim_start_matches(is_whitespace)` already strips those chars, so the `||` right operand never fires. Suggested simplification: remove the `matches!` branch and keep only `trimmed.starts_with(['=', '+', '-', '@'])`. Zero behavior change; slightly smaller guard surface. | Defer to a cleanup task |
| NB-2 | SW-3 / SW-5 | Advisory | 4 pre-existing `cargo clippy` warnings in `langfuse/importer.rs` (lines 182, 306, 316 — `map_or`/`manual_flatten` style) and `lib.rs:237` (`db_path` / `io_other_error`). None introduced by TASK-023. | Defer to a clippy-cleanup task |
| NB-3 | SW-5 | Advisory | 17 transitive RUSTSEC advisories: `glib 0.18.5` (RUSTSEC-2024-0429, CVSS 6.9 — below L2 threshold of 7.0; fixed in glib 0.20.0) and 16 no-CVSS unmaintained/soundness advisories on the Tauri Linux GTK3 binding stack (`atk`, `gdk*`, `gtk*`), `proc-macro-error`, and `unic-*` crates. Pre-existing; no lockfile change in this PR. | Defer to a dependency-refresh task |
| NB-4 | SW-3 | BA feedback | DEC-023 pending BA ratification: entry/project notes are stored verbatim (only empty/all-whitespace → `None`); the input layer no longer trims note whitespace. This is the behavior the adversarial acceptance test mandates. | Pi-Assistant to route to BA flow for formal ratification |

---

## Tag and signing

**Planned tag:** `task-023/v0.1.1`

**Signing status: DEFERRED — tag not created.**

L2 policy requires a signed tag. SSH signing is configured (`git config gpg.format=ssh`,
`tag.gpgsign=true`, `user.signingkey=/Users/kaikkonen/.ssh/id_ed25519.pub`), but the private key
at `/Users/kaikkonen/.ssh/id_ed25519` is absent in this environment (only the public key
is present). This is the same condition as TASK-021 and TASK-022.

**Tag creation is blocked until the signing key is provisioned.** No tag has been created and
none will be pushed. No unsigned fallback tag is created per L2 policy.

When the key is available, create the tag against the branch tip or post-merge commit on `main`:

```bash
git tag -s task-023/v0.1.1 <SHA> -m "task-023/v0.1.1 — CSV adversarial export hardening (PATCH)"
git push origin task-023/v0.1.1
```

---

## PR promotion

PR #15 promoted from draft to **ready-for-review** as final SW-6 step (see below). Janne can
merge directly — no additional un-draft step needed.
