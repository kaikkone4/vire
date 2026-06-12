# Documentation Review — TASK-019 Local Docker Langfuse Importer MVP

- **Documentation Engineer:** SW-6
- **Change:** `task-019-local-langfuse-importer-mvp`
- **Branch:** `feat/task-019-local-langfuse-importer-mvp` · **PR:** #11
- **Tier:** L2 · **Gate context:** SW-6 docs-drift check after SW-4 PASS (`b2b28c2`)
- **Date:** 2026-06-11
- **Verdict:** **PASS — docs drift found and fixed**

---

## Scope reviewed

| Artifact | Description |
|---|---|
| `README.md` | Intro status line, §Local Langfuse Docker stack, §AI trace import, §Availability and UX, §Privacy status |
| `docs/langfuse-local-setup.md` | Full setup/operations guide: health states table, Vire import endpoint configuration |
| `docs/backup-restore.md` | Backup scope, consistency, restore procedure |
| `openspec/changes/task-019-local-langfuse-importer-mvp/specs/langfuse-importer/spec.md` | TASK-019 spec delta (5 ADDED requirements) |
| `openspec/changes/task-019-local-langfuse-importer-mvp/design.md` | Architect design: config model, health taxonomy, REST contract |
| `openspec/changes/task-019-local-langfuse-importer-mvp/review.md` | SW-4 code review (PASS) |
| `src-tauri/src/langfuse/config.rs` | Implementation: env var names, defaults, credential handling |
| `src-tauri/src/lib.rs` (lines 175–191) | IPC commands: `get_langfuse_source_health`, `import_langfuse_now` |

---

## 1. DEC-020 posture accuracy

Verified against the five required dimensions across `README.md`, `langfuse-local-setup.md`, `backup-restore.md`:

| Dimension | README.md | langfuse-local-setup.md | backup-restore.md | Result |
|---|---|---|---|---|
| Local Docker self-hosted Langfuse is the default | Intro paragraph; §Setup | Intro (DEC-020 cited); §Vire import endpoint | Intro (primary AI trace source) | **PASS** |
| Cloud is explicit non-default override only | §AI trace import | §Vire import endpoint: "`cloud` (explicit override — produces off-host egress)" | n/a | **PASS** |
| Default base URL `http://127.0.0.1:3000` | §AI trace import; §Setup step 3 | §Localhost binding; §Vire import endpoint table | Post-restore checklist | **PASS** |
| All service ports loopback-bound; MinIO not host-published | §Components table; §Security cautions | §Localhost binding; §MinIO console access | n/a | **PASS** |
| Docker/stack down ≠ zero AI usage or cost | §Availability and UX | Health states table + prose | n/a | **PASS** |

---

## 2. Drift findings and fixes

Three drift items were found and fixed in this gate pass.

### D-1 — README intro stated importer was "in active development" (FIXED)

**Finding:** `README.md` intro read: "Automatic macOS activity capture and Langfuse trace import are
in active development (TASK-005, TASK-007)." TASK-019 shipped the importer MVP; the status was
stale.

**Fix:** Updated to: "The local Docker Langfuse trace importer MVP is available (TASK-019);
automatic macOS activity capture is in active development (TASK-005)."

### D-2 — Env vars undocumented in `langfuse-local-setup.md` (FIXED)

**Finding:** The implementation in `config.rs` reads five environment variables that control all
importer behaviour. None were named in any documentation — the "Vire import endpoint configuration"
table only described settings semantically without identifying how to set them. An operator or
developer following the setup guide could not configure the importer from docs alone.

| Env var | Default | Purpose |
|---|---|---|
| `VIRE_LANGFUSE_BASE_URL` | `http://127.0.0.1:3000` | Importer base URL |
| `VIRE_LANGFUSE_SOURCE` | `local` | Source posture (`local` / `cloud`) |
| `VIRE_LANGFUSE_ENVIRONMENTS` | `vire` | Comma-separated allowed environments |
| `VIRE_LANGFUSE_PUBLIC_KEY` (fallback: `LANGFUSE_PUBLIC_KEY`) | — | API credential |
| `VIRE_LANGFUSE_SECRET_KEY` (fallback: `LANGFUSE_SECRET_KEY`) | — | API credential |

**Fix:** Expanded the `docs/langfuse-local-setup.md` §Vire import endpoint configuration table to
include an `Env var` column for all five settings, noted the credential fallback vars, and added a
preamble sentence directing operators to use a gitignored `.env` file. Also clarified that setting
`VIRE_LANGFUSE_SOURCE=cloud` requires setting `VIRE_LANGFUSE_BASE_URL` to the Cloud endpoint as
well.

### D-3 — Health states table missing 4 of 10 states (FIXED)

**Finding:** The health states table in `docs/langfuse-local-setup.md` listed only 6 of the 10
states defined in `model.rs` / design.md §6. Missing:

| State | Why it matters to operators |
|---|---|
| `auth_or_network_error` | Misconfigured credentials or network outage triggers this; AI totals are withheld |
| `schema_changed` | Langfuse API field shape mismatch; AI totals withheld; signals Langfuse version change |
| `delayed` | Informational: late-arriving trace reconciled, not dropped |
| `duplicate` | Informational: trace deduplicated across re-imports |

**Fix:** Added all four states to the health states table in `docs/langfuse-local-setup.md` with
their meaning and Vire behaviour.

---

## 3. Unchanged docs accuracy check

| Criterion | Status |
|---|---|
| MinIO not host-published | `langfuse-local-setup.md` component table: `none (internal); API minio:9000, console :9001 inside container — not host-published` | **PASS** |
| MinIO bucket must not be set to public | `langfuse-local-setup.md` §MinIO/S3 access control | **PASS** |
| Three-store backup consistency (PG + ClickHouse + MinIO) | `backup-restore.md` §What must be backed up | **PASS** |
| Volume names consistent across all docs | `langfuse_postgres_data`, `langfuse_clickhouse_data`, `langfuse_minio_data` — identical in README, `langfuse-local-setup.md`, `backup-restore.md` | **PASS** |
| Divergence failure modes documented | `backup-restore.md` §Divergence failure modes (6 scenarios) | **PASS** |
| Credentials never committed/logged/exported | README §Object storage; `langfuse-local-setup.md` §Credentials; env var table "Notes" column | **PASS** |

---

## 4. OpenSpec / implementation coherence

| Criterion | Status |
|---|---|
| `specs/langfuse-importer/spec.md` present and non-empty | **PASS** |
| All 5 ADDED requirements covered: default local source, down-stack ≠ zero, wrong_env surfaced, pagination/dedup/observation-usage, credential/activity isolation | **PASS** |
| `config.rs` defaults match spec `DEFAULT_BASE_URL = "http://127.0.0.1:3000"`, `DEFAULT_ENVIRONMENT = "vire"` | **PASS** |
| 10-state health taxonomy in `model.rs` matches `design.md` §6 and BA `04_technical_plan.md` §7 | **PASS** (confirmed by SW-4 review §3.1) |
| Absence-≠-zero: nullable cost columns, `sum_opt_*` helpers, probe gate | **PASS** (confirmed by SW-4 review §3.2) |
| No credential material in `SourceHealthSnapshot` IPC DTO | **PASS** (confirmed by SW-4 review §5.1) |

---

## 5. Non-blocking notes

**NB-01 — IPC commands not in user-facing docs.**
`get_langfuse_source_health` and `import_langfuse_now` are documented in `design.md` §1 and the SW-4
review §5 but not in `docs/`. They are internal Tauri commands, not a public API; this is acceptable
at L2. If TASK-009 (review UI) adds end-user-visible import triggers, update the user guide then.

**NB-02 — SW-4 non-blocking suggestions S-1 through S-8 not reflected in docs.**
All eight are implementation-hygiene items (dead parameter, timestamp format inconsistency,
transaction boundary, etc.). None have docs implications; they remain candidates for a post-MVP
hygiene pass.

**NB-03 — Pre-existing `csv_export_neutralizes_formula_like_project_names_and_notes` test failure.**
Classified as pre-existing (SW-4 §6); not a TASK-019 docs item.

---

## 6. Verdict

**PASS — three docs drift items found and fixed.**

All five DEC-020 posture dimensions remain accurate across `README.md`, `docs/langfuse-local-setup.md`,
and `docs/backup-restore.md`. The drift items (stale README status line, undocumented env vars, and
incomplete health states table) have been corrected in this gate pass. OpenSpec spec delta, design,
and implementation are coherent. No blocking issue remains.
