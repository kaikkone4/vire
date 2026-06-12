# Release — TASK-021 Langfuse Persist-Failure IPC Surfacing (Hotfix)

- **Gate:** SW-6 (Release Manager) · **Tier:** L2
- **Change:** `task-021-langfuse-persist-failure-ipc-surfacing`
- **Branch:** `fix/task-021-langfuse-persist-failure-ipc-surfacing` · **PR:** [kaikkone4/vire#13](https://github.com/kaikkone4/vire/pull/13)
- **Release unit:** Hotfix to TASK-020 — make persist-failure surfacing authoritative in-band and marker-independent
- **Date:** 2026-06-12
- **Final branch head:** `8c8146a` (docs gate artifacts committed)
- **Product version baseline:** `0.1.0` (from `src-tauri/Cargo.toml`) · **Release tag:** `task-021/v0.2.2` (PATCH hotfix)
- **Verdict:** RELEASE-READY (all three required declarations complete)

---

## Release contents

### What this hotfix fixes

TASK-020 (PR #12, merged `main@dd5d3b9`) introduced the S-4 guarantee that persistence failures are surfaced and never read as healthy or zero. Post-merge verification found a gap: when both the import run transaction **and** the durable failure-marker insert fail under the same fault (read-only DB, disk full, lock contention, or a constraint/trigger covering `langfuse_import_runs`), `import_langfuse_now` returned the prior persisted snapshot — which could be `healthy`. The absence-≠-zero invariant was violated by the very scenario S-4 was designed to prevent.

Root cause: `run_blocking_import` discarded `run_import`'s `Vec<ImportSummary>` and always returned `Ok(())`, so the in-memory degrade to `Unknown` (already produced by `persist_run`) never reached the IPC caller. The only durable surfacing channel (the marker insert) was precisely what the fault disabled.

### Runtime changes (Rust core, single commit `99e1118`)

| File | Change |
|------|--------|
| `src-tauri/src/langfuse/importer.rs` | `PERSIST_FAILURE_MSG` made `pub`; doc comment updated to reflect best-effort marker role. No behavior change. |
| `src-tauri/src/langfuse/mod.rs` | `run_blocking_import` now captures `run_import`'s summaries and passes them to a new private `import_result()` helper; returns `Err(PERSIST_FAILURE_MSG)` when the persist-failure sentinel is present. Keys on the exact sentinel — not `health == Unknown`, which is also reachable from legitimately-persisted `Indeterminate`. |
| `src-tauri/src/langfuse/tests.rs` | New regression test `persist_failure_surfaces_in_band_even_when_marker_write_also_fails`: installs a trigger on `langfuse_import_runs` (forcing both writes to fail), seeds a prior `healthy` snapshot, and asserts the in-band `Err` fires while the stale snapshot remains `healthy` — proving the DB snapshot is not the authoritative channel. Needle scan confirms secret-free `Err`. |
| `src-tauri/src/lib.rs` | **No edit.** `import_langfuse_now`'s existing `run_bounded(…)?` already propagates the new `Err`, preventing any stale-snapshot read. |

### No schema changes, no new IPC commands, no frontend changes, no dependency changes

`langfuse_import_runs`, `langfuse_raw_traces`, `langfuse_ai_evidence`, `projects`, `time_entries` tables unchanged. `get_langfuse_source_health` and `import_langfuse_now` command signatures unchanged. `src/main.ts`, `tauri.conf.json`, `capabilities/`, `Cargo.toml`, and `Cargo.lock` untouched.

### Test result

`cargo test --lib` — 40 passed, 0 failed (includes the new both-writes-fail regression and unchanged TASK-020 tests). Zero new clippy warnings on changed code.

---

## Gate prerequisites (all PASS)

| Gate | Role | Verdict | Artifact | Commit audited |
|------|------|---------|----------|----------------|
| SW-3 | QA Engineer | PASS | `qa.md` | `99e1118` |
| SW-4 | Code Reviewer | PASS | `review.md` | `99e1118` |
| SW-5 | Security Agent | PASS | `sec.md` | `99e1118` |
| SW-6 Docs | Documentation Engineer | PASS | `docs.md` | `99e1118` |

All four gate artifacts present in the change directory (`qa.md`, `review.md`, `sec.md`, `docs.md`). `docs.md` confirms no drift — the single user-observable behavioral change (non-healthy error under total DB unwritability) is covered by the pre-existing `unknown` health state documentation; no documentation update was required or made.

---

## Required declaration 1 — Deployment size

**`patch` (correctness hotfix; no breaking changes; no new surface).**

- Single commit (`99e1118`); three production files touched
- No new IPC commands, no new DB tables or columns, no schema migration
- No new health states, no REST contract change, no frontend change
- No dependency added, removed, or version-bumped
- `src-tauri/Cargo.toml` version string remains `0.1.0` (release tracked by tag `task-021/v0.2.2`)
- The change tightens an existing correctness guarantee (S-4) — strictly safer than prior behaviour; no rollback risk from the fix direction itself

---

## Required declaration 2 — Rollback strategy

**`partial-automated`.**

| Layer | Rollback action | Classification |
|-------|-----------------|----------------|
| App binary | Replace with TASK-020 build artifact (`task-020/v0.2.1`) | Automated |
| IPC commands | `get_langfuse_source_health` / `import_langfuse_now` signatures unchanged; TASK-020 binary serves them identically | Automated |
| DB tables | No schema change; all five tables layout-identical to TASK-020 baseline. TASK-020 binary is fully compatible with TASK-021 database state. | Automated |
| Rust source revert | Single-commit revert (`git revert 99e1118`) restores the TASK-020 in-band behaviour | Manual (trivial — single commit, no merge complexity) |

**Forward path is strongly preferred.** This hotfix closes a correctness gap; rolling back restores false-healthy risk under total-DB-unwritability faults. No data is at risk either direction — there is no schema diff.

**Pre-condition for rollback:** Retain the TASK-020 build artifact (`.app` / installer) before deploying TASK-021. No automated artifact pinning is configured at this tier; retaining the previous build is a manual operator step.

**Staging test status:** `cargo test --lib` green (40/40) on developer workstation. Live Docker/Langfuse stack integration test not run in CI (Docker stack unavailable in session); all fault scenarios validated via SQLite trigger mocking. Live rollback drill deferred per L2 staging policy — recommend executing on a developer workstation before merging to `main`.

---

## Required declaration 3 — Component compatibility matrix

This hotfix makes no dependency changes. The matrix is inherited from TASK-020 (`task-020/v0.2.1`) — all versions are identical.

### Rust / Tauri runtime

| Component | Min version | Max version (tested) | Notes |
|-----------|-------------|---------------------|-------|
| `rustc` | 1.77 (Tauri 2 MSRV) | 1.95.0 | Unchanged from TASK-020 |
| `tauri` | 2.2 | 2.2 | Pinned; `tauri.conf.json` unchanged |
| `tauri-build` | 2.0 | 2.0 | Build-time only |
| `tauri-plugin-dialog` | 2.2 | 2.2 | Pre-existing, unchanged |
| `rusqlite` | 0.32 | 0.32 | `bundled` feature — embeds SQLite; no system SQLite dep |
| `reqwest` | 0.12 | 0.12.28 | `rustls-tls` + `blocking` + `json`; no OpenSSL; unchanged |
| `url` | 2.0 | 2.5.8 | URL parsing/allowlist enforcement; unchanged |
| `serde` / `serde_json` | 1.0 | 1.x | Pinned major; unchanged |
| `chrono` | 0.4 | 0.4.x | `serde` + `clock` features; unchanged |
| `uuid` | 1.0 | 1.x | `v4` feature; unchanged |

### Frontend / toolchain

| Component | Min version | Max version (tested) | Notes |
|-----------|-------------|---------------------|-------|
| Node.js | LTS 20 | LTS 22 | Per `.nvmrc` / engines; unchanged |
| TypeScript | 5.x | 5.x | No frontend changes in this release |
| `package.json` deps | unchanged | unchanged | No npm changes |

### External services

| Component | Min version | Notes |
|-----------|-------------|-------|
| Langfuse (local Docker) | 2.0 | `http://127.0.0.1:3000`; default source; unchanged |
| Langfuse Cloud | — | Explicit non-default override; `VIRE_LANGFUSE_SOURCE=cloud` required |
| Docker Engine | 24.x | Required for local stack; Compose v2 |
| macOS | 12 (Monterey) | Primary target; Linux/Windows via Tauri 2 but untested |

---

## Non-blocking advisories rollup

### Code review suggestions (SW-4, TASK-021)

| ID | Location | Finding | Recommended follow-up |
|----|----------|---------|----------------------|
| S1 | `mod.rs:54–56` | `import_result` double-`.any()` — readable as-is; `.flat_map` alternative marginally shorter but no clearer | No action unless a style pass standardises warning iteration across the module |
| S2 | `mod.rs:58` | When multiple environments fail, `import_result` returns the same fixed `PERSIST_FAILURE_MSG` regardless of which were affected — correct for IPC contract; per-env detail is in the in-memory summaries | Add one-line doc note to `import_result` if multi-environment use becomes more prominent |
| S3 | `importer.rs:182, 305–316` | 4 pre-existing clippy warnings in untouched TASK-020 code; zero new warnings from TASK-021 changes | Tracked separately; address in a future cleanup task |

### Security advisories (SW-5, TASK-021)

| ID | Finding | Status | Recommended follow-up |
|----|---------|--------|----------------------|
| A1 | 18 OSV advisories in pre-existing Tauri/GTK3/glib/`unic`/`proc-macro-error` transitive deps; none CVE≥7.0; none introduced by TASK-021 (`Cargo.lock` untouched) | Carry-forward from TASK-019/020 | Track for resolution at next Tauri bump |
| A2 | gitleaks 2 false positives in gitignored `src-tauri/target/` build artifacts; cannot be committed; no action possible | Carry-forward / false-positive | No action |

### Carry-forward from TASK-020

| Item | Status |
|------|--------|
| TASK-020 S1 — marker-insert double-discard comment | Partially addressed: TASK-021 updated the `importer.rs` doc comment to reflect the best-effort role; `mod.rs` marker path unchanged. Consider closing if the doc update satisfies intent. |
| TASK-020 S2 — `started_at`/`finished_at` same-stamp | Open; deferred (S-1 carry from `design.md §8`) |
| TASK-020 S3 — `let _ =` annotation on `mod.rs:42` discard | Open; TASK-021 restructured `mod.rs` but did not touch this annotation |
| TASK-020 A2/A3 — OSV advisories + unencrypted trace payload | Open; address at next Tauri bump / tier milestone |
| Pre-existing CSV adversarial test failure (`csv_export_neutralizes_formula_like_project_names_and_notes`) | Open; unrelated to TASK-021; confirmed pre-existing on base `dd5d3b9` |

---

## Tag and signing

**Planned tag:** `task-021/v0.2.2`

**Signing status: DEFERRED — tag not created.**

L2 policy requires a signed tag. SSH signing is configured (`git config gpg.format=ssh`, `tag.gpgsign=true`), but the private key at `/Users/kaikkonen/.ssh/id_ed25519` is absent in this environment (only the public key is present).

**Tag creation is blocked until the signing key is provisioned.** No tag has been created and none will be pushed. No unsigned fallback tag is created per L2 policy.

**Required action (developer / CI):** once the SSH private key is available, run:

```
git tag -s task-021/v0.2.2 99e1118 -m "task-021/v0.2.2"
git push origin task-021/v0.2.2
```

Substitute the merge commit SHA if tagging post-merge on `main`.

Artifact signing (SBOM via `cargo audit --json` + `cargo deny`) required at L2; likewise deferred pending key availability.

---

## Handoff

- **Verification flow (Flow 3):** stubbed — deployed artifacts for environment testing.
- **Documentation Engineer:** not dispatched (patch hotfix; no user-facing surface or docs drift introduced).
- **Tag/signing follow-up:** provision SSH signing key; create and push `task-021/v0.2.2` per the instructions above.
- **SW-4 S1–S2 suggestions:** minor; low-priority follow-up.
- **Carry-forward advisories:** address at next Tauri bump and tier milestone respectively.
- **CSV defect:** dedicated follow-up task (pre-existing).
