# SW-5 Security Review — TASK-019 Local Docker Langfuse Importer MVP

- **Change:** `task-019-local-langfuse-importer-mvp`
- **PR:** #11 (`feat/task-019-local-langfuse-importer-mvp` → `main`)
- **Reviewed commit:** `b2b28c2` (tree of `b30025e` feat slice)
- **Tier:** L2 (secrets + CVE≥7 + Trivy + semgrep ERROR)
- **Reviewer:** Security Agent (SW-5)
- **Verdict: PASS** — no auto-fail condition hit.

## 1. Scope

Code reviewed (the security-relevant surface of PR #11):

| File | Role |
|------|------|
| `src-tauri/src/langfuse/config.rs` | Credential types, env loading, URL allowlist / loopback enforcement |
| `src-tauri/src/langfuse/api.rs` | reqwest client, auth header, redirect policy, error mapping |
| `src-tauri/src/langfuse/importer.rs` | Import engine, absence-≠-zero invariant |
| `src-tauri/src/langfuse/store.rs` | SQLite persistence, health snapshot DTO |
| `src-tauri/src/langfuse/model.rs` | DTOs, health taxonomy, `ApiError` (secret-free) |
| `src-tauri/src/langfuse/mod.rs` | Module wiring, blocking-import entry point |
| `src-tauri/src/langfuse/tests.rs` | Unit tests (redaction, allowlist, read-only) |
| `src-tauri/src/lib.rs` | Tauri IPC commands `get_langfuse_source_health`, `import_langfuse_now` |
| `src/main.ts` | Renderer: source health banner / panel, import button |
| `src-tauri/Cargo.toml` | New deps `reqwest 0.12` (rustls), `url 2` |

## 2. Tier 1 scanner results

| Scanner | Version | Result | Auto-fail condition | Status |
|---------|---------|--------|---------------------|--------|
| **gitleaks** | 8.30.1 | history `main..HEAD` clean (38 commits); 1 working-tree hit is a **false positive** in a gitignored build artifact (`src-tauri/target/.../libmuda-*.rmeta`, an escaped keybinding string) | any committed secret | **PASS** |
| **semgrep** | 1.165.0 | 0 ERROR findings (also 0 WARNING/INFO) across 9 Rust + 4 TS files; rulesets `p/rust`, `p/typescript`, `p/security-audit`, `p/secrets` | any ERROR finding | **PASS** |
| **OSV-scanner** | 2.3.8 | 17 advisories, max CVSS **6.9** (`glib` RUSTSEC-2024-0429, MODERATE); all others are unscored "unmaintained crate" notices | CVE ≥ 7.0 (CVSS) | **PASS** |
| **Trivy** | 0.71.1 | 0 HIGH/CRITICAL vulns, 0 secrets, 0 misconfig (`Cargo.lock` + `package-lock.json`) | HIGH or CRITICAL | **PASS** |

### gitleaks detail
- `gitleaks git . --log-opts="main..HEAD"` → **no leaks** (the authoritative PR check).
- The single working-tree finding is inside `src-tauri/target/` (compiled dependency metadata), which is gitignored (`git check-ignore` confirms) and never committed. Not a leak.
- Test fixtures (`sk-lf-supersecret-value`, `pk-lf-public-id` in `tests.rs`) are obviously-fake redaction placeholders, not real key material, and history scan did not flag them.

### OSV detail
All 17 advisories are **pre-existing transitive dependencies of the Tauri/GTK desktop framework** (would also be present on `main`), not introduced by TASK-019:
- `gtk`/`gdk`/`atk`/`gdkx11`/`gtk3-macros` `0.18.x` — RUSTSEC-2024-041x "gtk-rs GTK3 bindings no longer maintained" (Linux GUI bindings; informational, no fix).
- `proc-macro-error 1.0.4` — RUSTSEC-2024-0370 unmaintained.
- `unic-*` `0.9.0` — RUSTSEC-2025-007x/008x/009x/010x unmaintained.
- `glib 0.18.5` — RUSTSEC-2024-0429 iterator unsoundness, **CVSS 6.9 (below the ≥7.0 auto-fail threshold)**.

The **new dependencies introduced by this PR** — `reqwest 0.12.28` and `url 2.5.8` — carry **zero advisories**. `reqwest` is configured `default-features = false` with `rustls-tls` (no OpenSSL/native-tls linkage). Licenses for both are MIT/Apache-2.0 (permissive — clean L2 license posture).

## 3. Manual review against TASK-019 security focus

### 3.1 No secrets committed / logged / persisted / exposed — PASS
- **`Secret` type** (`config.rs:43-59`) redacts in `Debug`/`Display` (`***redacted***`); value reachable only via `expose()`.
- **`Credentials` `Debug`** (`config.rs:68-76`) redacts both public *and* secret key.
- **Logging:** `ApiError.message` is documented and implemented secret-free; `map_transport_error` (`api.rs:123-136`) deliberately excludes URL, headers, and credentials — only a failure class is surfaced.
- **DB:** `store.rs` schema (`langfuse_import_runs`, `langfuse_raw_traces`, `langfuse_ai_evidence`) has **no credential column**. Enforced by test `import_run_table_has_no_credential_columns`.
- **IPC return type** `SourceHealthSnapshot` exposes only `base_url`, `source`, `environments`, timestamps, `health`, `message` — no key material.
- **CSV export** covers `projects`/`time_entries` only; no Langfuse config path.

### 3.2 Credentials only in Rust core / auth header; renderer never receives secrets — PASS
- Credentials are loaded from env vars (`VIRE_LANGFUSE_*` / `LANGFUSE_*`) in the Rust core (`config.rs:149-162`) and applied at exactly one call site: `req.basic_auth(public_key, Some(secret_key.expose()))` (`api.rs:65-67`).
- Renderer (`main.ts`) calls only `get_langfuse_source_health` and `import_langfuse_now`; both return the secret-free `SourceHealthSnapshot`.

### 3.3 Network boundary / SSRF / redirects / LAN — PASS
- **Default loopback:** `DEFAULT_BASE_URL = http://127.0.0.1:3000` (`config.rs:13`).
- **Allowlist invariant:** `validate_target` (`config.rs:184-204`) requires `Local` → loopback host (`127.0.0.1`/`localhost`/`::1`) and `Cloud` → non-loopback. Cloud is an explicit, non-default override (the only off-host egress path).
- **Constrained URL construction:** `build_url` (`config.rs:209-266`) builds from the configured base plus a fixed `/api/public/{health,traces,observations}` path set; query params are appended via `query_pairs_mut` (encoded). Defense-in-depth post-checks reject any URL not under `/api/public/` or that leaves the base host/scheme. No path or host from response data can be substituted (the `ApiPath` enum has no arbitrary-URL variant).
- **Scheme restricted** to `http`/`https` (`config.rs:167-173`); credential-in-host tricks (`http://127.0.0.1@evil.com`) resolve `host_str()` to the real host and are rejected.
- **Redirects disabled:** `redirect::Policy::none()` (`api.rs:50`) — a redirect can never bounce off the allowlisted origin. Matches the design claim.
- **No LAN exposure by default:** default is loopback; a LAN host under `Local` is refused.
- **Timeouts** (15 s request, 5 s connect) and a `MAX_PAGES = 1000` pagination backstop bound resource use.

### 3.4 Down/missing stack never read as zero usage/cost — PASS
- Availability probe gates the run (`importer.rs:60-70`): a down/unreachable stack records `unavailable`/`auth_or_network_error` per environment and writes **no** evidence rows.
- `sum_opt_i64`/`sum_opt_f64` (`importer.rs:298-316`) preserve absence as `None`; token/cost columns persist as SQL `NULL`, never `0`.
- 10-state health taxonomy (incl. `unavailable`, `unknown`) covers every "no data" path. Behaviour is locked by tests (`absence_is_never_zero_cost_when_stack_down`, `time_only_trace_has_null_cost_not_zero`, `schema_changed_when_generation_lacks_usage_and_cost`).

### 3.5 No raw macOS activity / window titles / prompts / command bodies sent to Langfuse — PASS
- The `LangfuseApi` trait is **GET-only** (`probe`/`get_traces`/`get_observations`); there is no write/push path. The only data leaving Vire is request query params (`environment`, `fromTimestamp`, `toTimestamp`, `traceId`, pagination) plus the auth header.
- Enforced by test `importer_only_issues_read_calls`.
- See advisory **A3** on local-at-rest payload handling (inbound, not egress).

### 3.6 DB schema: no secrets, safe null handling — PASS
- Token columns `INTEGER` nullable, `cost_total REAL` nullable; absence preserved as `NULL`. No secret-bearing columns. Additive migration; does not touch `projects`/`time_entries`.

### 3.7 Frontend / IPC cannot trigger SSRF or credential disclosure — PASS
- IPC commands `get_langfuse_source_health` and `import_langfuse_now` take **no URL/host arguments**; the target is env-derived only, so the renderer cannot influence egress (no SSRF surface).
- No IPC return type carries credentials.
- **XSS:** `main.ts` HTML-escapes (`esc()` = `escapeHtml`) every interpolated snapshot field — including `base_url`, `source`, `health`, `message`, `environments`, timestamps — before `innerHTML`, so an env-configured base URL cannot inject markup.

### 3.8 SQL injection — PASS
- All statements use bound parameters (`params![]`, `?n`). Dynamic SQL in `lib.rs` (`list_entries_repo`/`summary_repo`) only concatenates static fragments; values are always bound. No string interpolation of user input into SQL.

## 4. Findings

### Auto-fail (blocking): none

### Advisory (non-blocking, L2)
- **A1 — `Cargo.lock` untracked.** `src-tauri/Cargo.lock` exists but is neither committed nor gitignored. For a distributed binary/desktop app, commit the lockfile so builds are reproducible and dependency provenance is auditable. *(Supply-chain hygiene; does not affect this PR's verdict.)*
- **A2 — Pre-existing transitive advisories.** Tauri/GTK transitive crates carry unmaintained RUSTSEC notices (max CVSS 6.9, `glib`); none are introduced by TASK-019 and none reach the ≥7.0 auto-fail. Track for resolution when Tauri is next bumped. The GTK crates are Linux-only and not compiled on the macOS target.
- **A3 — Local-at-rest raw payloads.** `langfuse_raw_traces.payload` persists the full trace JSON which, per the DEC-020 MVP relaxation, may include prompt/session/metadata content. This is **inbound and local-only** (the SQLite DB is gitignored via `*.sqlite`), consistent with Vire's local-first posture — but the content is stored unencrypted at rest. Recommend documenting retention and considering field minimisation/encryption in a later tier.

## 5. Verdict

**PASS.** No L2 auto-fail condition was triggered: gitleaks history clean, semgrep 0 ERROR, OSV max CVSS 6.9 (< 7.0), Trivy 0 HIGH/CRITICAL. The credential-handling, network-boundary/SSRF, absence-≠-zero, and read-only (no raw-activity egress) properties required by the task are present and test-locked. Three advisory items (A1–A3) are recorded for follow-up but do not block.

→ Proceed: hand off to SW-6 Release Manager once SW-4 Code Review also passes.
