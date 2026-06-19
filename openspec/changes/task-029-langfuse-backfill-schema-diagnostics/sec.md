# Security review — TASK-029 (SW-5, DEC-032 re-run after saturation fix + SW-4 PASS)

**Change:** task-029-langfuse-backfill-schema-diagnostics
**Branch:** `feat/task-029-langfuse-backfill-schema-diagnostics` · **PR:** #23 (draft) · **Base:** `main`
**Tier:** L2 (secrets + CVE ≥ 7.0 + Trivy HIGH/CRITICAL + semgrep ERROR)
**Reviewed:** backend fix `11c8e1c`, QA recheck `0d6037e`; full TASK-029 range `6f90661..HEAD`.
**Verdict:** **PASS** — no L2 auto-fail condition hit. SEC-011 holds across the DEC-032 saturation fix:
`instant_saturated` is a boolean terminal flag end-to-end and the inclusive `resume_from`/`fromTimestamp`
cursor is a position timestamp confined to the local store + the outbound request URL — never serialized,
rendered, or logged.

This regenerates the SW-5 artifact against **DEC-032** as required by SW-4 (`review.md:30`). The prior
sec.md described the **superseded** exclusive resume-to / minimum-timestamp scheme; that scheme no longer
exists in the code. This pass re-audits the current implementation — the `instant_saturated` terminal
diagnostic (frontend + backend) and the inclusive-from chronological cursor — and re-runs the full Tier 1
stack. The prior SW-5 PASS conclusions for unchanged surfaces (loopback gate, never-logged auth,
non-echoing `since:` errors) still stand and are re-verified below.

---

## 1. Scanner stack (Tier 1, re-run for L2)

| Scanner | Target | Result | Auto-fail? |
|---------|--------|--------|------------|
| **gitleaks** 8.30.1 | full history (140 commits, 2.48 MB), redacted | **no leaks found** | No |
| **OSV-scanner** 2.3.8 | `src-tauri/Cargo.lock` (492 packages) | 17 advisories, **max CVSS 6.9 (< 7.0)** | No |
| **Trivy** 0.71.1 | `fs` vuln+secret+misconfig, severity HIGH,CRITICAL | **0 / 0 / 0** | No |
| **semgrep** 1.166.0 | `langfuse/`, `settings/`, `lib.rs`, `src/` (21 files, ERROR) | **0 findings (0 blocking)** | No |

**gitleaks** — `gitleaks git . --redact`: clean across the whole history including the DEC-032 fix commits.

**OSV-scanner** — 17 advisories, all **pre-existing transitive** dependencies (deprecated GTK3-rs binding
stack `atk*/gdk*/gtk*`, `proc-macro-error`, `unic-*`). The single CVSS-scored item is `glib` 0.18.5
(RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g) at **CVSS 6.9 — below the L2 CVE ≥ 7.0 auto-fail threshold**; the
rest are unscored "unmaintained/unsound" RUSTSEC notices. **None introduced by TASK-029** — `Cargo.toml`
and `Cargo.lock` are unchanged across `6f90661..HEAD` (verified empty diff), so no crate was added.
*Advisory (non-blocking):* the GTK3-rs / `glib` stack is unmaintained upstream; track for a future Tauri
dependency refresh. Out of scope for this task.

**Trivy** — `fs --scanners vuln,secret,misconfig --severity HIGH,CRITICAL`: 0 vulnerabilities, 0 secrets,
0 misconfigurations (`package-lock.json`, `src-tauri/Cargo.lock`). No auto-fail.

**semgrep** — `--config=auto --severity=ERROR --error` on the changed source: 118 rules over 21 git-tracked
files, ~100% parsed, **0 findings (0 blocking)**. semgrep's auto Rust coverage is partial, so the Rust core
was reviewed manually (below).

---

## 2. SEC-011 re-verification — DEC-032 saturation terminal & inclusive-from cursor

The DEC-032 fix introduced two surfaces relative to the prior SW-5 pass: a single-instant **saturation
terminal** diagnostic, and an **inclusive** `resume_from` / `fromTimestamp` cursor (`timestamp.asc`). Each
is re-checked against SEC-011.

### 2.1 `instant_saturated` is a boolean flag end-to-end — never a timestamp or secret — **PASS**
The terminal condition is carried as a `bool` at every layer: `ImportSummary.instant_saturated`
(`importer.rs:92`), `ImportReport.instant_saturated` + `EnvImportLine.instant_saturated`
(`importer.rs:129,152`), and the TS contract `instant_saturated: boolean`
(`import-report.ts:34,47`). No timestamp value is attached to the flag anywhere. The saturated instant is
surfaced only as the existing `traces_seen` **count**, never as the instant's value.

### 2.2 The resume cursor is confined to the importer + local store — never serialized/rendered/logged — **PASS**
The leak surface is closed **structurally**, not merely by test:

- **`ImportSummary` is `#[derive(Debug, Clone)]` only — *not* `Serialize`** (`importer.rs:59`). It carries
  the timestamp-bearing fields `page_limit_resume_ts: Option<String>` and `cursor_ts: Option<String>`
  (`importer.rs:102-103`), so those values cannot cross the IPC boundary.
- **`ImportReport::from_summaries` does not copy either timestamp** (`importer.rs:160-189`). Each
  `EnvImportLine` is built from only `environment`, `health`, `pages`, `traces_seen`, `unique`,
  `duplicates`, `skipped_schema`, `skip_reasons`, `skip_samples`, `reached_page_limit`,
  `instant_saturated` (bool), and `warnings`. No `page_limit_resume_ts`/`cursor_ts` reaches the report.
- **The IPC commands return only the serialized report.** `import_langfuse_now` / `backfill_langfuse_now`
  → `CmdResult<ImportOutcome>` whose `report: Option<langfuse::ImportReport>` (`lib.rs:763-765,813,837-839`).
  The `Vec<ImportSummary>` that carries the cursor stays inside the importer.
- **The persisted cursor is read back only internally.** `langfuse_backfill_progress(marker, resume_from,
  updated_at)` (`store.rs:58-62`) holds a fixed marker key, the inclusive RFC3339 resume boundary, and an
  update timestamp — **no trace content, prompt, session, count, or credential.** All three accessors
  (`backfill_resume_from`, `set_backfill_resume_from`, `clear_backfill_resume_from`) use `params![…]` bound
  parameters; no IPC command exposes the table. The value lives only in the local SQLite store (same trust
  boundary as the DEC-007 raw-trace store) — persisted because continuation needs it, never exposed.
- **No logging.** Scan of the whole `langfuse/` module (excl. tests) for
  `println!`/`eprintln!`/`dbg!`/`print!`/`log::`/`tracing::`/`console` → **none**. The cursor reaches no
  stdout/log surface.

### 2.3 The cursor reaches only the outbound request URL — intended, not a leak — **PASS**
The inclusive `resume_from` becomes the `fromTimestamp` query pair on the outbound `GET /api/public/traces`
(`config.rs:244,259`), alongside the fixed `orderBy=timestamp.asc` literal (`config.rs:249`). That URL is
built under `ImporterConfig::build_url`, which the same loopback/allowlist gate enforces on every call
(`api.rs:50`). Sending the cursor *to* the source API is its purpose; it is not rendered or logged.

### 2.4 Renderer adds only fixed flag-driven text — no value interpolation, escaping intact — **PASS**
The saturation surface in `import-report.ts` is entirely static copy gated on the `instant_saturated`
boolean: the per-env capped wording (`:111-115`) and the global terminal note (`:136-138`). Neither
interpolates a timestamp, cursor, boundary, or `since:` value. Every caller-supplied value — environment
name, health, JSON key names, field name, JSON type names, backend warnings — is wrapped in `escapeHtml`
(`:94,98-100,102,121,123`) and the headline is escaped (`:150`). XSS posture from imported trace data is
unchanged: no raw trace field value is ever rendered; only counts and key/type **names** reach the DOM.

### 2.5 Structural samples are key/type names only — **PASS** (re-verified)
`structural_sample` (`importer.rs:272-291`) emits the entry's top-level object **key names**, the offending
**field name**, and JSON **type names** (`json_type_name`) — never a field value or any payload content.
Samples are bounded per reason (`MAX_SAMPLES_PER_REASON`).

### 2.6 Malformed `since:` errors remain fixed strings (no echo) — **PASS** (re-verified)
`ImportRange::parse` (`mod.rs:63-83`) rejects a bad value with one of two **fixed `&'static str`** errors —
`"import range 'since:' value is not a valid RFC3339 timestamp"` /
`"import range value is not one of last_7d/last_30d/last_90d/all/since:<timestamp>"` — neither interpolates
the input. A `since:` value is validated as RFC3339 and normalized to UTC before storage.

### 2.7 No new egress; loopback boundary (SEC-002) preserved; GET-only — **PASS**
`api.rs` changed in this range (+6/-1) but the change is **security-neutral**: it threads an `order_by:
&str` parameter (the importer always passes the fixed `timestamp.asc` literal) through `get_traces` to the
URL builder. Still **GET-only**, same loopback/allowlist gate, `redirect::Policy::none()` (`api.rs:65`), and
`basic_auth` whose secret is `.expose()`d only into the Authorization header and never logged
(`api.rs:81`). **No new egress host, method, or auth surface.** A run that hits `MAX_PAGES` advances the
durable inclusive cursor so re-runs reach strictly-newer history (no silent truncation); the saturated
corner parks the cursor (never pushed past unread data) and is surfaced as a terminal/capped state.

### 2.8 No CSP / capability / dependency / updater / permissions drift — **PASS**
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/capabilities/`
are **all unchanged across `6f90661..HEAD`** (verified empty diff). No command, permission, allowlist entry,
crate, updater/signing config, or credential-storage change was added by TASK-029.

---

## 3. Supporting evidence
- **SEC-011 negative tests (Rust):** `import_report_is_secret_free` (`tests.rs:1345`) stuffs
  `name`/`sessionId`/`metadata` with `sk-lf-supersecret-canary`, `Bearer pk-lf-leak`, `sk-ant-oat01-leak`
  and asserts the report carries none; `skip_diagnostics_are_secret_free` (`tests.rs:1770`);
  `assert_test_result_secret_free` (`tests.rs:729`) asserts no Bearer/Authorization/sk-/pk-/password.
- **Saturation diagnostic is secret-free (frontend):** the saturated-instant test
  (`importReport.test.mjs:112`) asserts the terminal/capped wording renders AND that the HTML contains none
  of `sk-`/`pk-`/`Bearer`/`Authorization`/`oat01`/`prompt`/`session` **and no timestamp value** (`T00:`,
  `Z<`, `2026-`) — directly covering SEC-011 for the new surface. The mixed-env test
  (`importReport.test.mjs:137`) verifies the saturation terminal note and the ordinary rerunnable note are
  surfaced distinctly.
- **Structural guarantee** (stronger than tests): the cursor fields live on a non-`Serialize` struct and
  are omitted from `from_summaries`, so no boundary timestamp can reach IPC/report/UI.
- **SW-4 (Code Review) PASS** (`review.md`) and **SW-3 QA PASS** (`0d6037e`): 142 Rust + frontend tests
  green, `npm run build` / `cargo fmt --check` / `cargo clippy` / `openspec validate --strict` pass.

## 4. Findings
- **Auto-fail (blocking):** none.
- **Advisory (non-blocking, not introduced by this task):** unmaintained GTK3-rs / `glib` (CVSS 6.9, < 7.0)
  / `unic-*` / `proc-macro-error` transitive advisories. Track for a future Tauri dependency refresh.
- **Out of security scope (noted, owned by SW-4/SW-2):** the *ordinary* page-limit continuation copy at
  `import-report.ts:140` ("oldest history … progressively further back") reads backwards for the ascending
  sweep — SW-4 flagged it as a non-blocking copy correction (`review.md:28`). It is **static text with no
  value interpolation**, so it has **no SEC-011 / leak impact**; not a security blocker.

## 5. Escalations
None. No design-level security issue; no code-level FAIL. The DEC-032 fix is an implementation change
(terminal-saturation diagnostic + inclusive chronological cursor) with no security regression. The raw-trace
retention question (DEC-007) is already flagged to the BA-flow Architect by the change itself
(`arch-review.md` `feedback_to_ba[]`) and is out of this task's scope.

## 6. Verdict
**PASS.** No L2 auto-fail condition hit (gitleaks clean; max CVSS 6.9 < 7.0; Trivy 0 HIGH/CRITICAL; semgrep
0 ERROR). SEC-011 is preserved across the DEC-032 saturation fix: `instant_saturated` is a boolean terminal
flag, the inclusive `resume_from`/`fromTimestamp` cursor is a position timestamp confined to the local store
and the outbound request URL — excluded from every serialized/rendered/logged surface; the saturation/capped
diagnostics are fixed flag-driven copy; all imported values are HTML-escaped; `since:` errors remain
non-echoing; no new egress/CSP/capability/dependency/updater/auth. Proceed — release to SW-6 alongside the
SW-4 PASS.
