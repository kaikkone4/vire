# Security Review — TASK-027 Langfuse import + env discovery/mapping + desktop UX polish

- **Gate:** SW-5 (Security review) — parallel with SW-4 (Code review)
- **Change:** `task-027-langfuse-import-env-mapping-ux-polish`
- **Branch:** `feat/task-027-langfuse-import-env-mapping-ux-polish` (draft PR #22)
- **Tier:** L2 (secrets + CVE ≥ 7.0 + Trivy HIGH/CRITICAL + semgrep ERROR)
- **Date:** 2026-06-17 (full re-run after the SW-4 fix commits — `02f25c6` rustfmt, `7601811` RELEASE.md)
- **Reviewed head:** `7601811`
- **Base:** `df263ab` (TASK-026 PR #21 merge) — TASK-027 = the 5 commits `df263ab..HEAD`
- **Reviewer:** Security Agent
- **Verdict:** **PASS**

Primary control under review: **SEC-010** — the new import-diagnostics report, environment-discovery,
and environment→project-mapping surfaces must be secret-free (no credential, `Authorization` header,
raw API response body, or trace prompt/session content in the UI, logs, or reports). Preserved
invariants: **SEC-002** (loopback boundary for `source=local`; cloud is explicit), **SEC-003**
(credential / error redaction), and the absence-≠-zero contract. Out-of-scope guards re-verified: no
new renderer network call, no new egress host, CSP/capabilities unchanged, and the app self-updater
(TASK-028, DEC-029) is **not** implemented here.

---

## 1. Scope of audit

TASK-027 is the 5 commits on top of the TASK-026 merge (`df263ab..HEAD`). The audit keys on that range
(local `main` lags far behind the merge train, so `main…HEAD` is not the change surface). Security-bearing
files:

| Area | Files | SEC relevance |
|------|-------|---------------|
| Import report (A) | `langfuse/{importer.rs,mod.rs}` | New `ImportReport`/`EnvImportLine` — counts/health/warnings only |
| Payload tolerance (A) | `langfuse/model.rs` | `usageDetails`/`costDetails` parser; absence-≠-zero |
| Env discovery (C) | `langfuse/{discovery.rs,api.rs,config.rs,store.rs}` | Read-only scan; allowlist + loopback gate; name-only persistence |
| Env→project map (D) | `env_mapping/mod.rs`, `lib.rs` | Additive table; suggestion-first; no silent creation |
| Auto-import (B) | `lib.rs` | Startup + periodic thread; serialized; disabled short-circuit; loopback |
| IPC | `lib.rs` (`invoke_handler`) | 5 new commands, all secret-free |
| Renderer | `src/{main.ts,env-mapping-ui.ts,shell-chrome.ts,html.ts}` | Report/picker/mapping render; IPC-only; escaped |
| UX assets (E) | `src/style.css`, `icons/source/generate-vire-mark.mjs`, `icons/*` | Presentation; generator is pure pixel-math |

`runtime_observer/*` appears in the diff but the non-test changes are **rustfmt-only** (`git diff -w`
shows only line-wrapping); no behavioural change.

---

## 2. Scanner results (Tier L2 stack)

| Scanner | Version | Target / coverage | Result | Auto-fail condition | Verdict |
|---|---|---|---|---|---|
| **gitleaks** | 8.30.1 | git history `df263ab..HEAD` (5 TASK-027 commits, ~223 KB) | **0 leaks** | any detected secret | **PASS** |
| **gitleaks** | 8.30.1 | working tree (`src`, `src-tauri/src`, build output reachable) | **3 false positives**, all in untracked, git-ignored `src-tauri/target/{debug,release}/deps/libmuda-*.rmeta` | — | **PASS** (see §2.1) |
| **semgrep** | 1.165.0 | `src-tauri/src` + `src`, **30 files**, `p/rust` + `p/typescript` + `p/secrets` + `p/security-audit`, `--severity ERROR` | **0 findings, 0 errors** | any ERROR-severity | **PASS** |
| **OSV-scanner** | 2.3.8 | `Cargo.lock` (492 pkgs) + `package-lock.json` (107 pkgs) | 23 advisories — all pre-existing; none introduced by TASK-027 | CVE ≥ 7.0 reachable in artifact | **PASS** (see §4) |
| **Trivy** | 0.71.1 | `fs` (`vuln,secret`), HIGH/CRITICAL | **0 vulns, 0 secrets** | HIGH or CRITICAL | **PASS** |

No L2 auto-fail condition is met for code introduced by this change.

### 2.1 gitleaks working-tree note

The authoritative "can a secret be committed" signal is the **history** scan over the 5 TASK-027
commits — **clean**. The working-tree scan additionally walked the built `src-tauri/target/` and flagged
3 `generic-api-key` hits, all in compiled `muda` (menu) `.rmeta` metadata; the matched "secret" is a
keyboard-accelerator string (`shift+al…`), not a credential. All three are `git check-ignore`-matched
(`.gitignore:3 src-tauri/target/`), **0 files under `target/` are tracked**, and none appears in any
TASK-027 commit. These are the same disposable-artifact false positives documented in the TASK-026
SEC review — **not** an auto-fail.

---

## 3. SEC-010 + invariant manual review (all PASS)

Every check was verified by reading the implementation and confirmed by a passing test (the full
`cargo test --lib` run is **120 passed, 0 failed**; the SEC-relevant tests are named below).

| # | Focus (from the task brief) | Evidence | Result |
|---|---|---|---|
| 1 | **Import diagnostics secret-free** | `ImportReport::from_summaries` (`importer.rs`) folds **only** counts, the health enum as a fixed string, env names, and the existing secret-free warnings; per-trace `evidence` (token/cost) is deliberately excluded. Warnings can only be: coarse `ApiError.message` (fixed strings), `"a trace did not match the expected shape"`, or `PERSIST_FAILURE_MSG` — never payload-derived. Test `import_report_is_secret_free` stuffs a trace with `sk-`/`pk-`/`Bearer`/`Authorization`/`sessionId`/metadata secrets, imports it (`total_unique == 1`, non-vacuous), and asserts the serialized report contains none of them. | ✅ |
| 2 | **No raw bodies/prompts/session/keys in UI/logs/reports** | Renderer `importReportLine`/`mappingPanel`/`sourcePanel` render only counts, health, env names, and `esc()`-escaped warnings. `ApiError` messages are fixed coarse strings (`api.rs` `map_status_error`); `map_transport_error` never embeds the `reqwest::Error` (which can carry the URL). **No logging sink** exists in non-test `src-tauri/src` (`println!`/`eprintln!`/`dbg!`/`log::`/`tracing::` grep-clean; only matches are the `tauri_plugin_dialog` import/init). Raw trace payloads persist **only** to local SQLite (`store::upsert_raw_trace`, DEC-020 MVP relaxation — pre-existing, unchanged) and never cross into the report/UI/logs. | ✅ |
| 3 | **No new renderer network call** | CSP (`tauri.conf.json`, unchanged) is `connect-src ipc: http://ipc.localhost` — webview reaches only the IPC bridge. `src/*.ts` has **no** `fetch`/`XMLHttpRequest`/`WebSocket`/`EventSource`/`.src=` (grep-clean); all backend access is `invoke()` over IPC. | ✅ |
| 4 | **No new egress beyond `/api/public` with SEC-002 loopback/cloud** | The one new path, `ApiPath::TracesAllEnvironments` (drops only the `environment` filter), is built by the **same** `ImporterConfig::build_url`, which calls `validate_target()` (loopback for `local`, off-host for `cloud`), pins the path under `/api/public/`, and rejects host/scheme drift; `ReqwestLangfuseApi::new` keeps `redirect::Policy::none()`. No new host. Test `discovery_url_keeps_the_allowlist_and_loopback_gate_without_an_env_param`. | ✅ |
| 5 | **Auto-import disabled path: no Keychain, no network** | `run_auto_import_cycle` (`lib.rs`) acquires the slot, opens a throwaway connection **only** to read the non-secret `langfuse_enabled` switch, and returns before any probe/Keychain/socket when disabled. Test `auto_import_cycle_runs_nothing_when_disabled`. `import_langfuse_now` short-circuits identically before any network/Keychain. | ✅ |
| 6 | **Mapping: no silent project creation** | `set_env_mapping_repo` refuses an unknown `project_id` (`project_exists` guard) and **never** creates a project; creation flows only through the existing `create_project` IPC, triggered by an explicit user `prompt()` in `bindEnvMapping`. Tests `mapping_to_a_missing_project_is_refused_and_creates_nothing`, `discovered_unmapped_suggests_create_then_explicit_action_maps_it`. | ✅ |
| 7 | **CSP / capabilities unchanged** | `git diff df263ab..HEAD` for `src-tauri/tauri.conf.json` and `src-tauri/capabilities/default.json` is **empty**. Capabilities stay `core:default`, `dialog:default`, `dialog:allow-save`. | ✅ |
| 8 | **Updater not implemented** | No `updater`/`tauri-plugin-updater`/`minisign`/`latest.json` reference anywhere in source or config; `Cargo.toml`/`Cargo.lock` are **unchanged** by TASK-027 (no updater plugin, no new dependency). F is correctly deferred to TASK-028 (DEC-029). | ✅ |

### 3.1 Cross-cutting hardening (verified, non-blocking-clean)

- **Serialization (B3).** A shared `Mutex<()>` slot: `try_acquire_import_slot` (non-blocking — auto
  ticks skip when busy) and `acquire_import_slot` (blocking, bounded by the 30 s `run_bounded_result`
  ceiling — manual click waits). Poisoned lock is recovered (guarded `()` carries no invariant). Tests
  `import_slot_serializes_concurrent_imports`, `auto_import_cycle_skips_when_an_import_is_already_in_progress`,
  `auto_import_interval_floors_and_defaults` (interval floored at 30 s — cannot hammer the source).
- **SQL injection.** Every new statement (`env_mapping/mod.rs`, `store.rs` discovery) is parameterized
  (`params![]`); no string interpolation into SQL. The renderer uses `CSS.escape(env)` for selector
  lookups.
- **XSS.** All renderer interpolation passes through `escapeHtml` (`& < > ' "`); the sole `innerHTML`
  sink (`shell()`) escapes every dynamic value, and CSP `script-src 'self'` blocks inline execution as
  defence in depth.
- **absence-≠-zero.** The widened `usageDetails`/`costDetails` parser (`model.rs`) reads legacy→current
  in precedence; an absent/null key returns `None` (a present `0` reads as `Some(0)`); a genuinely
  unknown shape still degrades to `schema_changed`. Tests `observation_absent_usage_stays_none_not_zero`,
  `current_shape_generation_is_healthy_and_cost_captured`, `current_shape_with_empty_detail_maps_degrades_to_schema_changed`.
- **Discovery persistence is name-only.** `langfuse_discovered_environments` holds `(environment,
  first_seen, last_seen)` only; `discover_and_record` is best-effort (a discovery failure never fails an
  otherwise-successful import) and writes no credential/count/trace content.
- **Icon generator** (`generate-vire-mark.mjs`) imports only `node:zlib/fs/url/path` — pure pixel math,
  no network/exec/env/credential handling. New icon binaries are secret-free (history scan clean).

---

## 4. Advisory findings (non-blocking — documented per triage rubric)

These are real but do **not** meet the L2 auto-fail bar for code shipped by this change. **TASK-027
changed no dependency lockfile** (`Cargo.toml`, `Cargo.lock`, `package-lock.json`, `package.json` all
unchanged in `df263ab..HEAD`), so every advisory below is pre-existing/inherited, not introduced here.

- **npm dev-deps `vite` 6.4.2 and `esbuild`.** OSV flags `vite` (GHSA-fx2h-pf6j-xcff / CVE-2026-53571;
  GHSA-v6wh-96g9-6wx3 / CVE-2026-53632) and `esbuild` (GHSA-gv7w-rqvm-qjhr). All are **`devDependencies`**
  (build tooling) and **dev-server** vulnerabilities, and are **not reachable in the packaged `.app`**
  (Rust binary + static `dist/`, no dev server at runtime). Carried from TASK-026; recommend the same
  housekeeping bump (`vite` → latest 6.x, transitive `esbuild`). Trivy reported 0 HIGH/CRITICAL for the
  same lockfiles.
- **Rust `glib` 0.18.5 — RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g.** CVSS v4 ≈ 5.3 (Medium), **< 7.0**;
  pre-existing Tauri framework transitive dep. Track at framework-upgrade time.
- **~16 no-CVSS RustSec advisories** on `atk`/`gdk*`/`gtk*`/`gtk3-macros`/`proc-macro-error`/`unic-*` —
  the GTK/Linux backend (unused on the macOS target) plus transitive helpers. All pre-existing; advisory
  only.
- **gitleaks working-tree false positives** (§2.1) — 3 hits in git-ignored `target/` build artifacts.
  Ergonomics only: add a `target/` exclusion to the working-tree scan so it matches the (clean) history
  scan.

No advisory here blocks the gate.

---

## 5. Escalations

None. No design-level (trust-boundary / missing-auth-layer / wrong-egress) issue: the env-first
PROJECT_MAPPING and discovery realize the architecture plan within the existing importer boundary, the
loopback/allowlist boundary (SEC-002) is preserved and extended to the discovery path, and project
creation stays suggestion-first / human-approved (DEC-006). No `feedback_to_ba[]`.

---

## 6. Verdict

**SEC STATUS: PASS.** No L2 auto-fail condition is met for code introduced by TASK-027: gitleaks history
clean across all 5 commits, semgrep 0/0 ERROR on 30 files, Trivy 0 HIGH/CRITICAL, and no in-artifact
dependency at CVSS ≥ 7.0 (the npm HIGH advisories are dev-server-only, absent from the packaged app, and
not introduced by this change — no lockfile was touched). SEC-010 is satisfied — the import report,
environment-discovery, and mapping surfaces carry only counts, health, environment names, project
references, and secret-free coarse warnings; the dedicated `import_report_is_secret_free` test proves a
secret-stuffed trace leaks nothing into the serialized report. SEC-002 loopback and SEC-003 redaction are
preserved and now also guard the new no-filter discovery path; auto-import honours the disabled
short-circuit (no Keychain/network when off) and serializes with the manual path; mapping never
auto-creates a project; and CSP, capabilities, dependencies, and the (absent) updater are all unchanged.

**No design-level escalation** to BA-flow Architect. **No code-level FAIL** routed to the developer.

**Handoff:** PASS → wait for SW-4 (Code review); when both gates are green, route to SW-6 (Release Manager).
