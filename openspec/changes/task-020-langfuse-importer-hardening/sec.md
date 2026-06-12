# SW-5 Security Review — TASK-020 Langfuse Importer Hardening + L2 Release Hygiene

- **Change:** `task-020-langfuse-importer-hardening`
- **PR:** #12 (`feat/task-020-langfuse-importer-hardening` → `main`)
- **Reviewed range:** `main..HEAD` — 3 commits (`f8fd591` Cargo.lock, `22ad72b` OpenSpec change, `b4f0f3d` hardening feat)
- **Tier:** L2 (secrets + CVE ≥ 7.0 + Trivy + semgrep ERROR)
- **Reviewer:** Security Agent (SW-5)
- **Verdict: PASS** — no L2 auto-fail condition hit.

## 1. Scope

This is a **hardening + release-hygiene** change, not new capability. No new source, classifier,
runtime observer, review UI, or dependency. Security-relevant surface of PR #12:

| File | Change | Security relevance |
|------|--------|--------------------|
| `src-tauri/Cargo.lock` | **Newly tracked** (closes TASK-019 advisory A1) | Supply-chain: pins the exact dependency closure TASK-019 validated; SBOM-auditable. No version bump. |
| `src-tauri/src/langfuse/importer.rs` | `persist_run` → atomic + error-surfacing; `now()` → UTC RFC3339 | Persistence-failure handling, secret-free messages, absence-≠-zero |
| `src-tauri/src/langfuse/store.rs` | `+ persist_import_run` (one transaction) | Transaction/SQL safety |
| `src-tauri/src/lib.rs` | `import_langfuse_now` bounded via `run_bounded` | Bounded IPC, no new egress/surface |
| `src-tauri/src/langfuse/tests.rs` | `+152` mock-based tests (no network/creds) | Locks the new invariants |

**Unchanged (verified absent from the PR file set):** `Cargo.toml` (no new dep — `reqwest`/`url`
were introduced in TASK-019, not here), `tauri.conf.json` CSP, `capabilities/`, the REST contract,
the 10-state health taxonomy, schema columns, and the renderer (`src/main.ts` is not in PR #12).

## 2. Tier-1 (L2) scanner results

| Scanner | Version | Result | Auto-fail condition | Status |
|---------|---------|--------|---------------------|--------|
| **gitleaks** | 8.30.1 | `git . --log-opts="main..HEAD"` → **no leaks** (45 commits, 918 KB scanned) | any committed secret | **PASS** |
| **semgrep** | 1.165.0 | **0 ERROR** findings across the 4 changed Rust files; rulesets `p/rust`, `p/security-audit`, `p/secrets` (`--severity ERROR --error`) | any ERROR finding | **PASS** |
| **OSV-scanner** | 2.3.8 | committed `Cargo.lock`: 487 pkgs, 17 advisories, **max CVSS 6.9** (`glib` RUSTSEC-2024-0429, MODERATE); the other 16 are unscored "unmaintained crate" notices | CVE ≥ 7.0 (CVSS) | **PASS** |
| **Trivy** | 0.71.1 | `fs --scanners vuln,secret,misconfig --severity HIGH,CRITICAL` on `Cargo.lock`: **0 HIGH/CRITICAL**, 0 secrets, 0 misconfig | HIGH or CRITICAL | **PASS** |

### OSV detail — closure is identical to the TASK-019-validated set
The committed lock is exactly the closure TASK-019 validated (no `cargo update`). All 17 advisories
are **pre-existing Tauri/GTK transitive dependencies** (also present on `main`), not introduced by
TASK-020:
- `gtk`/`gdk`/`atk`/`gdkx11`/`gdkwayland`/`gtk3-macros` `0.18.x` — RUSTSEC-2024-041x "GTK3 bindings
  unmaintained" (Linux-only GUI bindings; **not compiled on the macOS target**; informational).
- `glib 0.18.5` — RUSTSEC-2024-0429 iterator unsoundness, **CVSS 6.9 (< 7.0 threshold)**.
- `proc-macro-error 1.0.4`, `unic-* 0.9.0` — unmaintained (unscored).
- The TASK-019 TLS/HTTP additions `reqwest`/`url` carry **zero advisories**; `reqwest` is
  `default-features = false` + `rustls-tls` (no OpenSSL/native-tls). Pinning them in the now-tracked
  lock is a supply-chain **improvement**, not a regression.

## 3. Manual review against the TASK-020 security focus

### 3.1 Cargo.lock / dependencies — PASS (improves posture)
No new or changed dependency versions. Tracking the lock (A1) pins the closure for reproducible,
SBOM-auditable builds and is the input for future `cargo audit`/`cargo deny`. OSV/Trivy on the
committed lock confirm no CVE ≥ 7.0 and no HIGH/CRITICAL. Net supply-chain improvement.

### 3.2 Persistence failure handling: secret-free, no false healthy/zero — PASS
- On a persistence failure (`importer.rs:424-436`), the only surfaced text is the fixed const
  `PERSIST_FAILURE_MSG` (`importer.rs:392-393`) — *"importer could not persist this run to the local
  store; recorded state is unknown"*. **No** config/credential/driver-string interpolation; the
  `rusqlite::Error` is discarded via `.is_err()` rather than echoed, so no driver text can leak.
- The summary degrades to `HealthState::Unknown` (non-healthy) and a separate **marker run** (its own
  fresh `Uuid`, `cursor_ts: None`) records the non-healthy state, so the failed run's id stays fully
  rolled back and currency is never advanced on a failed write. A persistence failure therefore can
  **never** read as `healthy` and never contributes a numeric zero — the absence-≠-zero invariant is
  materially reinforced, not weakened.
- Test-locked by `tests.rs` (S-3/S-4 atomicity + error-surface cases) and the IPC re-reads the health
  snapshot from the DB after the run, so the marker is what surfaces the failure to the renderer.

### 3.3 Transaction changes: no data leakage, no unsafe/unparameterized SQL — PASS
- `store::persist_import_run` (`store.rs:102-118`) wraps raw-trace upserts + evidence upserts + the
  run-record insert in a single `conn.unchecked_transaction()`; on any `?` early-return the
  `Transaction` is dropped without `commit`, so rusqlite issues ROLLBACK — no partially-written run.
- It introduces **no new SQL**: it reuses the existing `upsert_raw_trace` / `upsert_ai_evidence` /
  `insert_import_run` helpers, all of which use bound parameters (`params![]`, `?n`) — grep confirms
  no `format!`/string concatenation into any SQL statement in `store.rs`.
- `unchecked_transaction` is safe here: the importer runs on its own dedicated SQLite connection on
  the worker thread (TASK-019 model), so there is no concurrent/nested transaction on that handle.
- No credential-bearing column exists or is added (schema unchanged; TASK-019 test
  `import_run_table_has_no_credential_columns` still applies).

### 3.4 Bounded timeout: no secret leak, no new egress/surface — PASS
- `run_bounded` (`lib.rs:191-205`) spawns the existing blocking import on a dedicated OS thread and
  waits with `mpsc::recv_timeout(30s)`. On timeout it returns the fixed const `IMPORT_TIMEOUT_MSG`
  (`lib.rs:185-186`) — secret-free and reinforcing absence-≠-zero (*"AI usage and cost are unknown,
  not zero"*).
- **No new network call or endpoint** — it only bounds the wait around the pre-existing import. The
  channel carries `CmdResult<()>`; the only error strings reaching it originate from the TASK-019
  secret-free `ApiError` mapping. An orphaned late-finishing worker remains bounded by the reqwest
  15 s/5 s ceilings and persists atomically (§3.3) if it completes — no leak, no new surface.
- Regression-tested: `run_bounded_times_out_promptly_with_a_secret_free_error` asserts the error
  contains none of `sk-`/`pk-`/`password`/`token`/`Bearer`/`Authorization`, and
  `run_bounded_returns_the_works_result_within_the_ceiling` asserts a normal failure is surfaced
  verbatim (not masked as a timeout).

### 3.5 DEC-020 / TASK-019 guardrails preserved — PASS
- **Renderer off-network:** `src/main.ts` and `Cargo.toml` are untouched in PR #12; no renderer
  network code added.
- **No credential exposure:** both new surfaced strings are fixed consts; no log/evidence/export path
  added that could carry credentials. `Secret`/`Credentials` redaction (TASK-019) intact.
- **No raw activity egress:** the `LangfuseApi` trait stays GET-only; no write/push path added. Only
  inbound persistence changed.
- **Local Docker default / Cloud explicit override:** `config.rs` (loopback default + allowlist) is
  untouched. No change to the network boundary.
- **Timestamp normalization (S-5):** `now()` → `Utc::now().to_rfc3339_opts(SecondsFormat::Secs,
  true)` is a value-format change only (columns already `TEXT`); not a security-relevant surface, and
  it removes a zone-ambiguity foot-gun.

### 3.6 No new auth bypass, no new endpoint requiring rate limiting — PASS
- The only Langfuse IPC commands remain `get_langfuse_source_health` and `import_langfuse_now`
  (`lib.rs:175-176`, `206-207`), with **unchanged signatures** that take no URL/host arguments — the
  target stays env-derived, so the renderer cannot influence egress (no SSRF surface introduced).
- `run_bounded` is a **private helper, not a `#[tauri::command]`** — no new IPC endpoint, no new
  auth surface, and nothing new that would require rate limiting.

### 3.7 TASK-006 not introduced — CONFIRMED ABSENT
- No AI runtime observer, reconciliation, duplicate ledger, or pi/Claude adapter code is present. The
  only `reconcil` match in the diff is a static user-facing health message for the `Delayed` state
  (`store.rs:297`), pre-existing — not reconciliation logic. DEC-017 boundary respected.

## 4. Findings

### Auto-fail (blocking): none

### Resolved since TASK-019
- **A1 — `Cargo.lock` now tracked.** The TASK-019 supply-chain advisory is **closed** by this PR;
  the lock pins the validated closure and is SBOM-auditable.

### Advisory (non-blocking, L2)
- **A2 (carry-forward) — Pre-existing transitive RUSTSEC notices.** Tauri/GTK transitive crates carry
  unmaintained advisories (max CVSS 6.9, `glib`); none introduced here, none ≥ 7.0, GTK crates are
  Linux-only and not on the macOS target. Track for the next Tauri bump.
- **A3 (carry-forward) — Local-at-rest raw payloads.** `langfuse_raw_traces.payload` persists full
  trace JSON unencrypted in the local (gitignored) SQLite DB. Inbound/local-only, consistent with
  Vire's local-first posture; unchanged by TASK-020. Consider retention/field-minimisation in a later
  tier.
- **B1 (new, minor) — Best-effort failure marker.** If `persist_import_run` fails *and* the
  subsequent `insert_import_run(marker)` also fails (total DB unwritability), the marker is dropped
  (`let _ = …`). This does **not** fabricate a false healthy/zero — the snapshot then reflects only
  prior genuine runs, never a synthetic zero for the failed run — so the absence-≠-zero contract
  holds; it is a visibility gap only in a catastrophic-DB scenario where the app is already
  non-functional. No action required for L2; noted for completeness.

## 5. Verdict

**PASS.** No L2 auto-fail was triggered: gitleaks history clean, semgrep 0 ERROR, OSV max CVSS 6.9
(< 7.0), Trivy 0 HIGH/CRITICAL. The hardening **strengthens** the importer's security posture:
persistence failures are now atomic and surfaced with fixed secret-free messages (never healthy,
never zero), the import IPC is bounded with a secret-free timeout and no new egress/surface, SQL stays
fully parameterized inside one transaction, and the now-tracked `Cargo.lock` improves supply-chain
auditability (closing A1). DEC-020/TASK-019 guardrails (renderer off-network, no credential exposure,
no raw-activity egress, local Docker default / Cloud explicit override) are preserved, no new
auth/endpoint surface is added, and TASK-006 is confirmed not introduced. Advisories A2/A3 carry
forward; B1 is a minor non-blocking note.

→ Proceed: hand off to SW-6 Release Manager once SW-4 Code Review also passes.
