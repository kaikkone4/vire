# Release — TASK-019 Local Docker Langfuse Importer MVP

- **Gate:** SW-6 (Release Manager) · **Tier:** L2
- **Change:** `task-019-local-langfuse-importer-mvp`
- **Branch:** `feat/task-019-local-langfuse-importer-mvp` · **PR:** [kaikkone4/vire#11](https://github.com/kaikkone4/vire/pull/11)
- **Release unit:** First AI-evidence runtime path — read-only Langfuse REST importer in Rust core (TASK-019 / TASK-007 MVP slice)
- **Date:** 2026-06-11
- **Product version baseline:** `0.1.0` (from `src-tauri/Cargo.toml`) · **Release tag:** `task-019/v0.2.0` (MINOR feature addition)
- **Verdict:** RELEASE-READY (all three required declarations complete)

---

## Release contents

### Runtime changes (Rust core)

- `src-tauri/src/langfuse/config.rs` — `ImporterConfig`, `Secret` redaction type, `LangfuseSource`, URL allowlist (`validate_target` + `build_url`), env-var loading
- `src-tauri/src/langfuse/api.rs` — `LangfuseApi` trait, `reqwest::blocking` client (rustls-tls, 15 s / 5 s timeouts, `redirect::Policy::none()`), auth header, error mapping
- `src-tauri/src/langfuse/importer.rs` — probe → paginate → dedup → normalize engine; 10-state `classify_health`; absence-≠-zero `sum_opt_*`; `persist_run`
- `src-tauri/src/langfuse/model.rs` — `HealthState` (10 variants), `LangfuseTrace`, `LangfuseObservation`, `ApiError`, `ImportSummary`
- `src-tauri/src/langfuse/store.rs` — additive SQLite migration (`langfuse_import_runs`, `langfuse_raw_traces`, `langfuse_ai_evidence`); `SourceHealthSnapshot` DTO (credential-free)
- `src-tauri/src/langfuse/mod.rs` — module wiring, blocking-import entry point
- `src-tauri/src/langfuse/tests.rs` — 25 new tests (all 10 health states, pagination/dedup, absence-≠-zero, credential redaction, URL allowlist, read-only posture)
- `src-tauri/src/lib.rs` — two new Tauri IPC commands: `get_langfuse_source_health`, `import_langfuse_now` (lines 175–191 + handler registration)

### Runtime changes (frontend)

- `src/main.ts` — `sourceBanner()` / `sourcePanel()` for degraded-health surfacing; `import_langfuse_now` trigger; HTML-escaped (`esc()`) throughout

### Additive only — no existing tables or commands modified

`projects`, `time_entries`, `tauri.conf.json`, capability files, `Cargo.toml` version, and `package.json` are **unchanged**.

### Documentation / OpenSpec

- `openspec/changes/task-019-local-langfuse-importer-mvp/specs/langfuse-importer/spec.md` — importer spec
- `docs/langfuse-local-setup.md` — updated with MinIO/S3 bucket detail (§MinIO: `langfuse` bucket, private access, volume `langfuse_minio_data`)
- `docs/backup-restore.md` — MinIO in backup scope, divergence failure modes documented
- `openspec/changes/task-019-local-langfuse-importer-mvp/design.md`, `proposal.md`, `tasks.md`, `arch-review.md`

### Documentation drift fixes (SW-6 docs gate — commit `7e76584`)

Three drift items found and corrected by the Documentation Engineer in the SW-6 gate pass:

- **D-1** `README.md` — stale status line "in active development (TASK-005, TASK-007)" corrected to "local Docker Langfuse trace importer MVP is available (TASK-019)".
- **D-2** `docs/langfuse-local-setup.md` — env var table expanded with all five `VIRE_LANGFUSE_*` environment variables (base URL, source posture, environments, public key, secret key) including credential fallback vars and `.env` guidance. Previously no env var names appeared in any documentation.
- **D-3** `docs/langfuse-local-setup.md` — health states table completed: four previously missing states added (`auth_or_network_error`, `schema_changed`, `delayed`, `duplicate`). Table now covers all 10 states from `model.rs`.

Gate artifacts committed at `7e76584`: `openspec/changes/task-019-local-langfuse-importer-mvp/review.md` (SW-4), `sec.md` (SW-5), `docs.md` (SW-6).

### Baseline-build remediation (behavior-preserving)

- Added missing `src-tauri/icons/icon.png` (never committed; `generate_context!` panics without it)
- Mechanical compiler lifetime/type-annotation fixes in `lib.rs` (rustc 1.95.0 required)

### Summary

Vire's first AI-evidence runtime path. A read-only Langfuse REST importer runs entirely in the Rust core (`reqwest::blocking`); the renderer is never given network access (CSP unchanged). Default source is local Docker Langfuse at `http://127.0.0.1:3000` (loopback only); Cloud is an explicit non-default override — the sole off-host egress path. Credentials are env-sourced, redacted at the `Debug` boundary, and absent from every persisted/IPC surface. The importer implements the BA §7 10-state health taxonomy with a hard absence-≠-zero invariant: no "no data" path ever contributes a numeric zero to AI totals.

---

## Gate prerequisites (all PASS)

| Gate | Role | Verdict | Artifact | Commits reviewed |
|------|------|---------|----------|-----------------|
| SW-3 | QA Engineer | PASS | `qa.md` | `b30025e`, `b2b28c2` |
| SW-4 | Code Reviewer | PASS | `review.md` | `b30025e`, `b2b28c2` |
| SW-5 | Security Agent | PASS | `sec.md` | `b2b28c2` |
| SW-6 | Documentation Engineer | PASS | `docs.md` | `0de4f4a`, `7e76584` |

All gate artifacts (`qa.md`, `review.md`, `sec.md`, `docs.md`) are committed at `7e76584`. No task is being released that did not pass both SW-4 and SW-5.

---

## Required declaration 1 — Deployment size

**`minor` (new feature; first runtime importer; no breaking changes).**

- New Rust module (`src-tauri/src/langfuse/`) — 6 source files, ~1 400 lines of production code, 560 lines of tests
- Two new Tauri IPC commands
- Three new SQLite tables (additive migration; `CREATE TABLE IF NOT EXISTS`; no DDL on existing tables)
- New frontend banner and settings panel
- No removal of existing commands, columns, or behaviors
- No change to Tauri capability configuration or CSP
- `src-tauri/Cargo.toml` version string remains `0.1.0` (bumped via release tag `task-019/v0.2.0`)

---

## Required declaration 2 — Rollback strategy

**`partial-automated`.**

| Layer | Rollback action | Classification |
|-------|-----------------|----------------|
| App binary | Replace with prior build artifact | Automated |
| IPC commands | Removed with binary; frontend banner silently no-ops | Automated |
| DB tables | `langfuse_import_runs`, `langfuse_raw_traces`, `langfuse_ai_evidence` persist after binary rollback; no corruption of `projects`/`time_entries` | Manual (`DROP TABLE IF EXISTS` ×3 if desired) |
| Config env vars | `VIRE_LANGFUSE_*` / `LANGFUSE_*` ignored by prior binary; inert if left set | Automated |

**Forward path is preferred.** The additive migration is idempotent; re-installing the prior binary leaves existing tables intact and harmless. A full table-drop is only required if the storage footprint of raw trace payloads is a concern.

**Pre-condition for rollback:** A prior build artifact (`.app` / installer) must be retained before deploying. No automated artifact pinning is configured in this L1/L2 tier; retaining the previous build is a manual operator step.

**Staging test status:** Live Docker/Langfuse integration test was not run in CI (Docker stack unavailable); all scenarios validated via `MockApi` per `design.md §9`. Live rollback drill is deferred per L2 staging policy — recommend executing before `main` merge on a developer workstation.

---

## Required declaration 3 — Component compatibility matrix

### Rust / Tauri runtime

| Component | Min version | Max version (tested) | Notes |
|-----------|-------------|---------------------|-------|
| `rustc` | 1.77 (Tauri 2 MSRV) | 1.95.0 (CI) | 1.95.0 required to compile `lib.rs` lifetime fixes |
| `tauri` | 2.2 | 2.2 | Pinned; `tauri.conf.json` unchanged |
| `tauri-build` | 2.0 | 2.0 | Build-time only |
| `tauri-plugin-dialog` | 2.2 | 2.2 | Pre-existing, unchanged |
| `rusqlite` | 0.32 | 0.32 | `bundled` feature — embeds SQLite; no system SQLite dep |
| `reqwest` | 0.12 | 0.12.28 | `default-features = false`, `rustls-tls` + `blocking` + `json`; no OpenSSL |
| `url` | 2.0 | 2.5.8 | URL parsing/allowlist enforcement |
| `serde` / `serde_json` | 1.0 | 1.x | Pinned major |
| `chrono` | 0.4 | 0.4.x | `serde` + `clock` features |
| `uuid` | 1.0 | 1.x | `v4` feature |

### Frontend / toolchain

| Component | Min version | Max version (tested) | Notes |
|-----------|-------------|---------------------|-------|
| Node.js | LTS 20 | LTS 22 | Per `.nvmrc` / engines |
| TypeScript | 5.x | 5.x | `tsc --noEmit` clean at `b2b28c2` |
| `package.json` deps | unchanged | unchanged | No new npm dependencies introduced |

### External services

| Component | Min version | Notes |
|-----------|-------------|-------|
| Langfuse (local Docker) | 2.0 | `http://127.0.0.1:3000`; default source. Requires Langfuse web + worker + PostgreSQL + ClickHouse + Redis + MinIO/S3 stack per `docs/langfuse-local-setup.md` |
| Langfuse Cloud | — | Explicit non-default override; `VIRE_LANGFUSE_SOURCE=cloud` required |
| Docker Engine | 24.x | Required for local stack; Compose v2 |
| macOS | 12 (Monterey) | Primary target; Linux/Windows via Tauri 2 but untested in this release |

---

## Non-blocking advisories rollup

The following items from SW-4 (Code Review, S-1–S-8) and SW-5 (Security, A1–A3) are carried forward as post-MVP follow-up candidates. None block release.

### Code review suggestions (SW-4)

| ID | Location | Finding | Recommended follow-up |
|----|----------|---------|----------------------|
| S-1 | `importer.rs:129` | `_config` parameter accepted but unused | Remove or promote to use when per-env config customisation is added |
| S-2 | `importer.rs:172` | `ts.as_str() < cur.as_str()` is lexicographic RFC3339 comparison — silent failure on `+00:00`-offset timestamps | Parse both with `parse_ts()` and compare as `DateTime<Utc>` |
| S-3 | `importer.rs:384–420` | `persist_run` has no surrounding transaction — partial failure leaves invisible state | Wrap in `BEGIN`/`COMMIT`/rollback for atomic per-run write |
| S-4 | `importer.rs:395,401,419` | `let _ = ...` silently swallows all three DB call errors | Propagate errors into `summary.warnings` or return `Err` |
| S-5 | `importer.rs:46` | `now()` produces local-time strings; trace timestamps are UTC RFC3339 — two formats in same DTO | Use `Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` uniformly |
| S-6 | `lib.rs:188` | `thread::spawn(...).join()` has no timeout — import thread hang would block UI indefinitely | Add `mpsc::channel` + `recv_timeout(30 s)` or document reqwest timeouts as effective ceiling |
| S-7 | `main.ts:16` | `degradedHealth` omits `schema_changed`, `missing`, `wrong_env` | Evaluate adding `schema_changed` for TASK-009 review UI; `missing`/`wrong_env` lower priority |
| S-8 | `store.rs:237` | `source_health_snapshot` uses `latest_run` (all envs) — a `wrong_env` run can mask the last `vire` state | Low-priority MVP acceptable; consider filtering to `allowed_environments` when multi-env UX lands |

### Security advisories (SW-5)

| ID | Finding | Recommended follow-up |
|----|---------|----------------------|
| A1 | `src-tauri/Cargo.lock` exists but is neither committed nor gitignored — builds are not reproducible and dependency provenance is unauditable | Commit `Cargo.lock` to the repository (desktop binary convention); required for SBOM at L2+ |
| A2 | 17 pre-existing OSV advisories in Tauri/GTK transitive deps (max CVSS 6.9, `glib` RUSTSEC-2024-0429); none introduced by TASK-019; GTK crates Linux-only | Track for resolution when Tauri is next bumped; no action required now |
| A3 | `langfuse_raw_traces.payload` persists full trace JSON (unencrypted at rest); consistent with DEC-020 MVP relaxation but may include prompt/session/metadata content | Document retention policy; evaluate field minimisation/at-rest encryption at next tier |

### Pre-existing defect (SW-3 NB-1 / SW-4 §10)

`csv_export_neutralizes_formula_like_project_names_and_notes` adversarial integration test fails. Pre-existing (`adversarial.rs` last changed at `b1a9c6f`, before TASK-018/TASK-019; 0 diff lines from this PR). Root cause: `csv_escape` in the time-tracker core mishandles leading whitespace and bare `\r`. Needs a dedicated follow-up change targeting `csv_escape` in `lib.rs`.

---

## Tag and signing

**Tag:** `task-019/v0.2.0`

**Signing status: DEFERRED — no GPG key available in this environment.**

`gpg --list-secret-keys` returned no keys. Per L2 policy, signed tags are required; however, the Pi-assistant delegation for this gate authorises recording a deferral rather than blocking the PR promotion.

**Action taken (this run):** unsigned tag created at final HEAD `7e76584` (docs gate commit — the complete release state including all gate artifacts). Signing must be completed by the developer or CI pipeline before merge is treated as a full L2 release.

```
git tag task-019/v0.2.0 7e76584  # unsigned — pending signing key
```

Note: the prior RELEASE.md (commit `0de4f4a`) recorded the tag as pointing to `b2b28c2`. No tag existed in the repository at that time; the tag is created here at the final HEAD `7e76584`, which is the authoritative release state.

Artifact signing (SBOM) required at L2 but likewise deferred pending key availability. Track via A1 (commit `Cargo.lock`) as a prerequisite to producing a verifiable SBOM.

---

## Handoff

- **Verification flow (Flow 3):** stubbed — deployed artifacts for environment testing.
- **Documentation Engineer:** L2 required — route SW-6 docs gate after PR merge.
- **A1 follow-up (Cargo.lock):** commit `src-tauri/Cargo.lock` in a follow-up PR for supply-chain auditability.
- **S-3/S-4 follow-up:** transaction wrap + error propagation in `persist_run` (highest-priority post-MVP hygiene; data-integrity risk on partial failure).
- **A3 follow-up:** trace payload retention policy documentation and encryption consideration at next tier milestone.
