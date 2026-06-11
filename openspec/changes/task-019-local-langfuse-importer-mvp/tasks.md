# Tasks — TASK-019 Local Docker Langfuse importer (TASK-007 MVP slice)

Single component (Langfuse importer in the Rust core). Sub-tasks are an implementation sequence
within one OpenSpec change, **not** a component split. Recommended order:

## 1. Config + HTTP client foundation

- [ ] Add `reqwest` (rustls + json) and `tokio` to `src-tauri/Cargo.toml` — Rust core only; add no
      new Tauri capability and do not touch `tauri.conf.json` CSP.
- [ ] Importer config model: `base_url` (default `http://127.0.0.1:3000`), per-project
      `allowed_environments` (default `["vire"]`), `source` (`local` default / `cloud` explicit
      override), credentials from Keychain or chmod-600 gitignored `.env`.
- [ ] Enforce the Rust-side URL allowlist: every request URL is built from the configured base URL +
      a fixed `/api/public/*` path set; refuse any other host/scheme; loopback unless explicit
      Cloud/LAN override.
- [ ] Credentials used only for the `Authorization: Basic` header; never printed/logged/persisted/
      exported (SEC-003).

## 2. Importer-owned persistence (TASK-019 slice of TASK-004)

- [ ] Additive migration (extend `init_db`): `langfuse_import_runs`, `langfuse_raw_traces`,
      `langfuse_ai_evidence`. No `projects`/`time_entries` changes. No credentials stored.

## 3. Availability probe + REST pull

- [ ] Check local Docker/Langfuse availability **before** each run; if down ⇒ `unavailable`
      (never zero cost).
- [ ] `GET /api/public/traces?environment=&fromTimestamp=&toTimestamp=&page=&limit=`; paginate to
      `page >= totalPages`.
- [ ] Per-environment cursor/checkpoint in `langfuse_import_runs`; re-import with an overlap window
      to capture `delayed` traces.
- [ ] Dedup by `(environment, trace_id)`.

## 4. Schema/time/usage/cost normalization

- [ ] Read usage/cost from generation **observations** (`/api/public/observations` or
      `traces/{id}.observations[]`), aggregate to the trace; do not assume usage on the trace body.
- [ ] Read by observed shape (`usage` = `{input,output,total,unit}`, top-level token counts);
      tolerate nullable `sessionId`, empty `name`, variable `metadata`.
- [ ] Validate field presence/type; nullable cost/model or empty usage/cost details ⇒
      `schema_changed`, never zero cost.
- [ ] Environment mapping: treat `default`/unexpected-env traces as `wrong_env` (surface for
      review), not a silent pass.

## 5. Health taxonomy (BA §7, 10 states)

- [ ] Implement `healthy / missing / stale / wrong_env / delayed / duplicate / schema_changed /
      auth_or_network_error / unavailable / unknown` (map feeder `valid→healthy`,
      `schema_mismatch→schema_changed`, `auth_or_config_error`+`rate_limited`→`auth_or_network_error`).
- [ ] Assert the absence-≠-zero invariant in every no-data path (empty env ⇒ health flag, never `0`).

## 6. Docker-down surfacing (minimal)

- [ ] Read-only IPC command `get_langfuse_source_health` → active base URL, configured
      environment(s), last import, latest trace timestamp, current health state; **never** secrets.
- [ ] Thin frontend banner reflecting `unavailable`/`stale`/`unknown` (full review UI = TASK-009).

## 7. Verification

- [ ] `cargo test` (importer unit tests: pagination/dedup, all 10 health transitions, absence-≠-zero,
      credential redaction, loopback-default, Cloud-explicit-override, no raw-activity egress).
- [ ] `openspec validate task-019-local-langfuse-importer-mvp --strict`.
- [ ] Keep `docs/langfuse-local-setup.md` + `docs/backup-restore.md` (MinIO/S3 private, three-store
      backup consistency) accurate as the importer lands.

## Out of scope (do not build here)

- AI runtime observer (TASK-006), classification (TASK-008), full review/approval UI (TASK-009),
  CSV export of AI evidence (TASK-010), macOS capture adapter (TASK-005), and any new pi/Claude
  adapter (DEC-017).
