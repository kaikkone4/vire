# Release — TASK-020 Langfuse Importer Hardening + L2 Release Hygiene

- **Gate:** SW-6 (Release Manager) · **Tier:** L2
- **Change:** `task-020-langfuse-importer-hardening`
- **Branch:** `feat/task-020-langfuse-importer-hardening` · **PR:** [kaikkone4/vire#12](https://github.com/kaikkone4/vire/pull/12)
- **Release unit:** Hardening of the TASK-019 Langfuse importer — atomic persistence, error surfacing, UTC timestamps, bounded IPC, and build-reproducibility lock
- **Date:** 2026-06-12
- **Product version baseline:** `0.1.0` (from `src-tauri/Cargo.toml`) · **Release tag:** `task-020/v0.2.1` (PATCH hardening)
- **Verdict:** RELEASE-READY (all three required declarations complete)

---

## Release contents

### Runtime changes (Rust core)

- `src-tauri/src/langfuse/store.rs` — new `persist_import_run` entry point that wraps raw-trace upserts, evidence upserts, and run-record insert in a single `conn.unchecked_transaction()`; a `Transaction` dropped without commit auto-rolls-back (S-3 atomicity).
- `src-tauri/src/langfuse/importer.rs` — `persist_run` now calls `store::persist_import_run` and on failure: degrades `summary.health` to `HealthState::Unknown`, pushes fixed secret-free `PERSIST_FAILURE_MSG`, inserts a marker run record (`cursor_ts: None`) so the failed run is visible in the health snapshot (S-4 error surfacing). `now()` changed to `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` for `started_at`/`finished_at`/`imported_at` (S-5 UTC RFC3339 timestamps).
- `src-tauri/src/lib.rs` — `import_langfuse_now` replaced with `run_bounded` wrapper: spawns blocking import on OS thread, sends result over `mpsc::channel`, returns `IMPORT_TIMEOUT_MSG` on `recv_timeout(30 s)` (S-6 bounded IPC). Orphaned worker is bounded by reqwest 15 s/5 s ceilings.
- `src-tauri/src/langfuse/tests.rs` — 14 new tests covering S-3 atomicity (SQLite `BEFORE INSERT` trigger abort), S-4 error surfacing (secret-free warning, non-healthy snapshot), S-5 timestamp ordering (RFC3339 vs legacy space-format), and S-6 timeout/passthrough. All 39 tests pass.

### Build reproducibility

- `src-tauri/Cargo.lock` — first-ever commit (A1 close). Pins exactly the TASK-019-validated dependency closure (487 pkgs). No `cargo update`, no version bumps. Now the SBOM input for `cargo audit`/`cargo deny`.

### No schema changes, no new IPC commands, no frontend changes

`projects`, `time_entries`, `langfuse_import_runs`, `langfuse_raw_traces`, `langfuse_ai_evidence` tables unchanged. IPC commands `get_langfuse_source_health` and `import_langfuse_now` signatures unchanged. `src/main.ts`, `tauri.conf.json`, capability files, and `Cargo.toml` version string are untouched.

### Summary

This is a hardening-only release over TASK-019's MVP importer. No new user-visible features. The four hardening scenarios (S-3 atomicity, S-4 error surfacing, S-5 UTC timestamps, S-6 bounded IPC) eliminate data-integrity risks and unbound-thread risks that were deferred from TASK-019 as S-3/S-4/S-5/S-6 follow-ups. Committing `Cargo.lock` (A1) closes the TASK-019 supply-chain advisory and enables reproducible, SBOM-auditable builds.

---

## Gate prerequisites (all PASS)

| Gate | Role | Verdict | Artifact | Commits reviewed |
|------|------|---------|----------|-----------------|
| SW-3 | QA Engineer | PASS | `qa.md` | `f8fd591`, `22ad72b`, `b4f0f3d` |
| SW-4 | Code Reviewer | PASS | `review.md` | `f8fd591`, `22ad72b`, `b4f0f3d` |
| SW-5 | Security Agent | PASS | `sec.md` | `f8fd591`, `22ad72b`, `b4f0f3d` |

All gate artifacts (`qa.md`, `review.md`, `sec.md`) are present in the change directory. No task is being released that did not pass both SW-4 and SW-5.

---

## Required declaration 1 — Deployment size

**`patch` (hardening of existing feature; no breaking changes; no new surface).**

- No new IPC commands, no new DB tables, no new frontend features
- No existing commands, columns, or behaviors removed or altered in a breaking way
- Behavior changes are entirely internal to the Langfuse importer (atomicity, error propagation, timestamp format, thread management)
- One build-configuration file added (`Cargo.lock`) — no runtime impact
- `src-tauri/Cargo.toml` version string remains `0.1.0` (bumped via release tag `task-020/v0.2.1`)

---

## Required declaration 2 — Rollback strategy

**`partial-automated`.**

| Layer | Rollback action | Classification |
|-------|-----------------|----------------|
| App binary | Replace with prior build artifact (TASK-019 / `task-019/v0.2.0`) | Automated |
| IPC commands | `get_langfuse_source_health` / `import_langfuse_now` are signature-unchanged; prior binary serves them identically | Automated |
| DB tables | No schema changes; the three TASK-019 tables (`langfuse_import_runs`, `langfuse_raw_traces`, `langfuse_ai_evidence`) are layout-identical. A rollback to the TASK-019 binary leaves the DB fully compatible. | Automated |
| `Cargo.lock` | Removing it from git does not affect running binaries; only affects future builds | Manual (revert commit if desired) |
| `projects` / `time_entries` | Entirely untouched; zero rollback action required | N/A |

**Forward path is strongly preferred.** The hardening changes are additive safety improvements with no schema diff. Rolling back sacrifices atomicity and error-surfacing guarantees but does not corrupt data.

**Pre-condition for rollback:** A prior build artifact (`.app` / installer) must be retained before deploying. No automated artifact pinning is configured at this tier; retaining the previous build is a manual operator step.

**Staging test status:** Live Docker/Langfuse integration test was not run in CI (Docker stack unavailable in session); all scenarios validated via `MockApi`. Live rollback drill is deferred per L2 staging policy — recommend executing before `main` merge on a developer workstation.

---

## Required declaration 3 — Component compatibility matrix

### Rust / Tauri runtime

| Component | Min version | Max version (tested) | Notes |
|-----------|-------------|---------------------|-------|
| `rustc` | 1.77 (Tauri 2 MSRV) | 1.95.0 (CI) | Unchanged from TASK-019 |
| `tauri` | 2.2 | 2.2 | Pinned; `tauri.conf.json` unchanged |
| `tauri-build` | 2.0 | 2.0 | Build-time only |
| `tauri-plugin-dialog` | 2.2 | 2.2 | Pre-existing, unchanged |
| `rusqlite` | 0.32 | 0.32 | `bundled` feature — embeds SQLite; no system SQLite dep |
| `reqwest` | 0.12 | 0.12.28 | `default-features = false`, `rustls-tls` + `blocking` + `json`; no OpenSSL; no version change |
| `url` | 2.0 | 2.5.8 | URL parsing/allowlist enforcement; no version change |
| `serde` / `serde_json` | 1.0 | 1.x | Pinned major; no version change |
| `chrono` | 0.4 | 0.4.x | `serde` + `clock` features; no version change |
| `uuid` | 1.0 | 1.x | `v4` feature; no version change |

**No dependency was added, removed, or version-bumped in this release.** `Cargo.lock` is newly committed but represents the same closure that `cargo build` was already resolving against.

### Frontend / toolchain

| Component | Min version | Max version (tested) | Notes |
|-----------|-------------|---------------------|-------|
| Node.js | LTS 20 | LTS 22 | Per `.nvmrc` / engines; unchanged |
| TypeScript | 5.x | 5.x | No frontend changes in this release |
| `package.json` deps | unchanged | unchanged | No npm changes |

### External services

| Component | Min version | Notes |
|-----------|-------------|-------|
| Langfuse (local Docker) | 2.0 | `http://127.0.0.1:3000`; default source; unchanged from TASK-019 |
| Langfuse Cloud | — | Explicit non-default override; `VIRE_LANGFUSE_SOURCE=cloud` required |
| Docker Engine | 24.x | Required for local stack; Compose v2 |
| macOS | 12 (Monterey) | Primary target; Linux/Windows via Tauri 2 but untested |

---

## Non-blocking advisories rollup

### Code review suggestions (SW-4, TASK-020)

| ID | Location | Finding | Recommended follow-up |
|----|----------|---------|----------------------|
| S1 | `importer.rs:435` | `let _ = store::insert_import_run(conn, &marker)` — double-discard on marker insert failure is intentional but undocumented | Add one-line comment: `// If even the marker cannot be written there is nothing further to degrade.` |
| S2 | `importer.rs:420–421` | `started_at` == `finished_at`: both capture the same stamp at persistence time; `started_at` semantically misleads ("persistence start" not "import start") | Carry pre-existing S-1 deferred item (`design.md §8`); capture a start stamp at the top of `run_import` or `persist_run`'s caller |
| S3 | `mod.rs:42` | `importer::run_import(…)` return value discarded without annotation | Add `let _ =` to make the intentional discard explicit and consistent with `importer.rs:435` |

### Security advisories (SW-5, TASK-020)

| ID | Finding | Status | Recommended follow-up |
|----|---------|--------|----------------------|
| A2 | 17 pre-existing OSV advisories in Tauri/GTK transitive deps (max CVSS 6.9, `glib` RUSTSEC-2024-0429); none introduced here; GTK crates Linux-only, not compiled on macOS target | Carry-forward from TASK-019 | Track for resolution when Tauri is next bumped |
| A3 | `langfuse_raw_traces.payload` persists full trace JSON unencrypted at rest; consistent with DEC-020 local-first MVP relaxation | Carry-forward from TASK-019 | Document retention policy; evaluate field minimisation/at-rest encryption at next tier |
| B1 | If `persist_import_run` fails *and* the subsequent `insert_import_run(marker)` also fails (total DB unwritability), the marker is silently dropped — a visibility gap only in catastrophic-DB scenarios. Absence-≠-zero contract still holds. | New, minor, non-blocking | No action required at L2; noted for completeness |

### Pre-existing defect (carry-forward)

`csv_export_neutralizes_formula_like_project_names_and_notes` adversarial test fails. Pre-existing on the TASK-019 baseline (`64d5f9f`); 0 diff lines from this PR. Root cause: `csv_escape` in the time-tracker core (`lib.rs`) mishandles leading whitespace and bare `\r`. Needs a dedicated follow-up task targeting `csv_escape`.

---

## Tag and signing

**Planned tag:** `task-020/v0.2.1`

**Signing status: DEFERRED — tag not created.**

L2 policy requires a signed tag. SSH signing is configured (`git config gpg.format=ssh`, `tag.gpgsign=true`), but the private key at `/Users/kaikkonen/.ssh/id_ed25519` is absent in this environment.

**Tag creation is blocked until the signing key is provisioned.** No tag has been created and none will be pushed. No unsigned fallback tag is created per L2 policy.

**Required action (developer / CI):** once the SSH private key is available, run:

```
git tag -s task-020/v0.2.1 b4f0f3d -m "task-020/v0.2.1"
git push origin task-020/v0.2.1
```

where `b4f0f3d` is the current branch tip; substitute the merge commit SHA if tagging post-merge on `main`.

Artifact signing (SBOM via `cargo audit --json` + `cargo deny`) required at L2; likewise deferred pending key availability. `Cargo.lock` is now committed and ready as the SBOM input.

---

## Handoff

- **Verification flow (Flow 3):** stubbed — deployed artifacts for environment testing.
- **Documentation Engineer:** L2 required — route SW-6 docs gate after PR merge.
- **Tag/signing follow-up:** provision SSH signing key; create and push `task-020/v0.2.1` per the instructions above.
- **SW-4 S1–S3 suggestions:** minor code-comment improvements; low-priority follow-up.
- **A2/A3 carry-forward advisories:** address at next Tauri bump and next tier milestone respectively.
- **csv_escape defect:** dedicated follow-up task.
