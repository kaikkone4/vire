# SEC — TASK-021: Surface persist failure to import IPC, marker-independent

- **Gate:** SW-5 Security Review
- **Verdict:** **PASS**
- **Tier:** L2 (secrets + CVE≥7 + Trivy HIGH/CRITICAL + semgrep ERROR)
- **Branch:** `fix/task-021-langfuse-persist-failure-ipc-surfacing`
- **Reviewed head:** `99e1118` (single-commit hotfix; PR #13)
- **Base:** `main` @ `dd5d3b9` (TASK-020 PR #12)
- **Scope of audit:** the TASK-021 commit only. The `main…HEAD` diff includes the whole TASK-020 merge; this review keys on `99e1118`, which touches `mod.rs`, `importer.rs`, `tests.rs` (+ change docs).

## Change surface (what `99e1118` actually modifies)

| File | Nature | Security relevance |
|------|--------|--------------------|
| `src-tauri/src/langfuse/importer.rs` | `PERSIST_FAILURE_MSG` made `pub`; doc/comment on best-effort marker | Sentinel is a fixed string; no behavior change |
| `src-tauri/src/langfuse/mod.rs` | `run_blocking_import` inspects summaries; new private `import_result()` returns `Err(sentinel)` on persist failure | In-band failure channel |
| `src-tauri/src/langfuse/tests.rs` | both-writes-fail regression test | Asserts secret-free `Err`, no stale-healthy |
| docs (`proposal/spec/tasks/arch-review`) | text only | — |

No edits to `lib.rs`, `store.rs`, `config.rs`, `api.rs`, `model.rs`, `src/main.ts`, `tauri.conf.json`, `capabilities/`, `Cargo.toml`, or `Cargo.lock` — verified via `git show --stat`.

## Scanner results (Tier L2 stack)

| Scanner | Version | Scope | Result | Auto-fail? |
|---------|---------|-------|--------|------------|
| **gitleaks** | 8.30.1 | commit `99e1118`, full history (92 commits), working tree | **0 leaks** in tracked code/history & commit | No |
| **semgrep** | 1.165.0 | `p/rust` + `p/security-audit` + `p/secrets`, ERROR severity, `src-tauri/src` + `src` | 51 rules on 14 files — **0 findings, 0 ERROR** | No |
| **OSV-scanner** | 2.3.8 | `src-tauri/Cargo.lock` (487 pkgs) | 18 advisories, **none CVE≥7.0** | No |
| **Trivy** | 0.71.1 | `fs` vuln+secret+misconfig, HIGH/CRITICAL | **0 HIGH/CRITICAL** (Cargo.lock 483 pkgs, npm 2 pkgs detected) | No |

### gitleaks detail
- Tracked code, full 92-commit history, and the TASK-021 commit: **clean**.
- Working-tree (`dir`) scan flagged **2** `generic-api-key` hits — both in `src-tauri/target/debug/deps/libmuda-*.rmeta`. These are **untracked Rust build artifacts**; `src-tauri/target/` is gitignored (`.gitignore:3`) and **0 files under it are tracked**. False positives in compiled binary metadata; cannot be committed. Not an auto-fail.
- The test introduces a deliberately **secret-shaped canary** (`sk-leak-canary token`) inside a SQL `RAISE` message. This is a test fixture proving the surfaced error never echoes the driver string — not a real credential. gitleaks commit scan did not flag it.

### OSV-scanner detail
All 18 advisories are **pre-existing transitive framework dependencies** (Tauri's GTK3/glib Linux tree, plus `unic-*`, `proc-macro-error`), unchanged by TASK-021 (`Cargo.lock` untouched):
- 12× gtk-rs GTK3 bindings — **unmaintained** (RUSTSEC-2024-04xx), no CVSS.
- `glib` RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g — **unsound**, MODERATE, CVSS v4 `AV:N/AC:L/.../VI:L` ≈ 5.3 (< 7.0).
- 5× `unic-*` + `proc-macro-error` — **unmaintained**, no CVSS.

None meet the L2 auto-fail threshold (CVE ≥ 7.0 CVSS). All are **advisory**, carried from TASK-019/020, and out of this hotfix's scope.

## Manual review — security focus checklist

| # | Focus | Finding | Result |
|---|-------|---------|--------|
| 1 | IPC error fixed & secret-free; no DB driver/credential interpolation | `PERSIST_FAILURE_MSG` is a `pub const &str` fixed string. `import_result` returns `Err(PERSIST_FAILURE_MSG.to_string())` — no formatting. `persist_run` checks `store::persist_import_run(...).is_err()` (error value **discarded**, never captured); both warning pushes use the constant; marker error is `let _`. No driver/config/credential text on the persist path. (SEC-003) | ✅ PASS |
| 2 | No secrets/credentials in code/tests/docs | gitleaks clean on tracked code/history; only canary fixture (test-only, proves secret-free). No real secrets in docs/qa. | ✅ PASS |
| 3 | No new egress, no renderer HTTP, no raw activity egress | No new network calls; reuses existing `ReqwestLangfuseApi`. `src/main.ts` (renderer) not in commit — off-network posture intact. CSP/capabilities untouched. | ✅ PASS |
| 4 | No schema change or credential persistence | `store.rs` untouched; no `migrate()` change; no new columns/tables; marker insert unchanged. Nothing written persists credentials. | ✅ PASS |
| 5 | No false healthy/zero on persist failure | Core fix. `import_langfuse_now` → `run_bounded(... run_blocking_import ...)?` short-circuits on the new `Err` **before** `health_snapshot` is read, so the stale `healthy` snapshot is never returned. Absence-≠-zero upheld. | ✅ PASS |
| 6 | No new auth bypass / endpoint / rate-limit surface | `invoke_handler` command list unchanged (no new Tauri command). `import_result` is private. No new IPC/exposed surface. | ✅ PASS |
| 7 | TASK-006 not introduced | No retry/backoff/reconciliation. `import_result` is a pure inspection of summaries; keys on the exact sentinel, not `health == Unknown`. | ✅ PASS |

### Test verification
Ran the regression directly:
```
cargo test --lib persist_failure_surfaces_in_band_even_when_marker_write_also_fails
test ... ok  (1 passed)
```
It asserts the in-band `Err` contains none of `sk-`, `Bearer`, `Authorization`, `password`, `token`, `canary`, `forced`, `RAISE`, `ABORT`, and that the DB snapshot stays stale-healthy while the **command** result is a non-healthy `Err` — i.e. the in-band channel, not the snapshot, is authoritative. Directly validates SEC-003.

## Triage summary

- **Auto-fail conditions hit:** none.
- **Advisory (non-blocking):**
  - A1 — 18 OSV advisories in pre-existing GTK3/glib/`unic`/`proc-macro-error` framework deps (none CVE≥7). Inherited from TASK-019/020; track for a future Tauri/dependency bump. Out of scope for this hotfix.
  - A2 — gitleaks 2 false positives in gitignored `src-tauri/target/` build artifacts. No action; cannot be committed.

## Escalations

None. No design-level (trust-boundary / missing-auth-layer) issues. No `feedback_to_ba[]`.

## Verdict

**PASS.** No L2 auto-fail conditions hit. The fix makes persist-failure surfacing in-band and marker-independent, the error is a fixed secret-free string, and no new attack surface, egress, schema, or credential persistence is introduced. Proceed — wait for SW-4 (Code Review) before SW-6 (Release).
