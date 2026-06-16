# Security Review — TASK-026 Desktop production readiness

- **Gate:** SW-5 (Security review) — parallel with SW-4 (Code review)
- **Change:** `task-026-desktop-production-readiness`
- **Branch:** `feat/task-026-desktop-production-readiness` (draft PR #21)
- **Tier:** L2 (secrets + CVE≥7 + Trivy HIGH/CRITICAL + semgrep ERROR)
- **Date:** 2026-06-16 (**final recheck** after the atomic credential-pair fix — commit `935877f`, SW-3 re-verified `8d745d4`; supersedes the `66a9a76` recheck)
- **Reviewer:** Security Agent
- **Verdict:** **PASS** (re-confirmed)

Primary control under review: **SEC-009** — the Langfuse secret key must never appear in
plaintext in SQLite, logs, evidence, exports, the settings UI, or read APIs; it lives only in the
macOS Keychain. Preserved invariants: **SEC-002** (loopback boundary for `source=local`) and
**SEC-003** (credential non-leak / redaction).

---

## 0′. Final recheck — atomic credential-pair fix (commit `935877f`) — PASS

After the `66a9a76` recheck (§0 below), SW-4 (`review.md`) returned **FAIL** with one remaining
blocker: a failed **replacement** of an already-stored Keychain pair could leave a deleted public
key beside the surviving *old* secret, after which `resolve_credentials` would fill the missing
public key from the `VIRE_LANGFUSE_PUBLIC_KEY` / `LANGFUSE_PUBLIC_KEY` env dev fallback and pair it
with the stale Keychain secret — a **mixed-source credential pair** (DEC-026 credential-pair
integrity). The developer landed `935877f`; SW-3 re-verified (`8d745d4`, 94 Rust + 39 frontend
tests). This final recheck re-audits the credential-pair surface and re-runs the full L2 scanner
stack. The fix is correct and introduces **no** security regression.

| Property | Evidence (read line-by-line, `935877f`) | Status |
|---|---|---|
| **Failed replacement cannot mix env public key with old Keychain secret** | `set_langfuse_secret_repo` (`settings/mod.rs:295–313`) now captures `prior_public_key = secrets.get(PUBLIC_KEY_ACCOUNT)?` **before** the writes. On a failed secret write it restores the exact prior state: `Some(prior)` → re-`set` the prior public key (reinstating the prior consistent pair); `None` → `delete` the just-written entry (back to both absent). The secret entry is never touched on the failure path, so the pair is left **EITHER both-new OR both-prior** — never one entry beside a stale other. | ✅ |
| **Resolver still reads both fields from one source** | `resolve_credentials` (`settings/mod.rs:193–211`) is unchanged: per-field Keychain-first, env only on `Ok(None)`, `Err` short-circuits. Because the rollback restores the prior public key, the public-key read returns `Ok(Some(prior))` and the env fallback is **never consulted** — so the env public key cannot pair with the surviving Keychain secret. | ✅ |
| **Rollback is secret-free and non-masking** | The restore (`set`/`delete`) is best-effort (`let _ = …`); its own error is intentionally swallowed so it cannot mask the original **secret-free** `Err(e.0)` (a coarse `SecretStoreError` string, `secret_store.rs:62–80`). Reading the public key exposes no secret (SEC-009 guards the **secret** key only). | ✅ |
| **Regression test proves the property** | `failed_replacement_restores_the_prior_pair_and_never_mixes_keychain_with_env` (`settings/tests.rs:404–449`): seeds a prior `(P_OLD,S_OLD)` pair, forces a secret-write failure mid-replacement via `SecretWriteFailsStore`, then asserts (a) the error is secret-free, (b) both entries are restored to `P_OLD`/`S_OLD`, and — the decisive check — (c) with `VIRE_LANGFUSE_PUBLIC_KEY`/`LANGFUSE_PUBLIC_KEY` **set**, `resolve_config_with` still returns `public_key == P_OLD` (Keychain), never the env value. | ✅ |

**No new attack surface.** The diff touches only `set_langfuse_secret_repo` (one added pre-read +
a restore branch replacing an unconditional delete) and one test. No new IPC command, no new string
rendered to the UI/logs, no change to the redacting `Secret`/`Credentials` types or the CSP/
capabilities. The added `secrets.get` is a public-key read — outside the SEC-009 secret boundary.

**Refreshed L2 scanner run (this final recheck).**

| Scanner | Coverage | Result | Verdict |
|---|---|---|---|
| **semgrep** 1.165.0 | `src-tauri/src` + `src`, **25 git-tracked files**, 151 rules (`p/rust`,`p/typescript`,`p/secrets`,`p/security-audit`), `--severity ERROR` | **0 findings (0 blocking)** | **PASS** |
| **gitleaks** 8.30.1 | git history `ab17c5e~1..HEAD` (7 commits scanned, full TASK-026 range) | **0 leaks** | **PASS** |
| **OSV-scanner** 2.3.8 | `Cargo.lock` (492) + `package-lock.json` (106) | 3 High (npm **dev**-deps `esbuild`/`vite`), 1 Med 6.9 (Rust `glib`), 1 Med 5.5 + 1 Low (npm dev), 16 no-CVSS (Rust GTK/unic transitive) | **PASS** (see §4) |
| **Trivy** 0.71.1 | `fs` (`vuln,secret`), HIGH/CRITICAL | **0 vulns, 0 secrets** | **PASS** |

No in-artifact dependency at CVSS ≥ 7.0 (the 3 High are dev-only build tooling, absent at runtime in
the packaged `.app` — §4); `glib` 6.9 < 7.0. **94 Rust unit + 3 integration tests pass**, including
the new regression test. No L2 auto-fail condition is met. **Verdict unchanged: PASS.**

---

## 0. Recheck — post SW-4 fixes (commit `66a9a76`) — PASS

SW-4 (Code review, `review.md`) returned **FAIL** with two blockers + one suggestion; the developer
landed fixes in `66a9a76` and SW-3 re-verified (`c9274d6`). This recheck re-audits the **full**
security surface after those fixes (scanners re-run, fix diff read line-by-line, callers traced).
Every fix is implemented correctly and introduces **no** security regression.

| SW-4 finding | Required security property | Fix verified | Status |
|---|---|---|---|
| **Blocker 1** — disabled integration could still run a Test connection probe (Keychain read + network) | Disabled **short-circuits before any Keychain read or network probe** | New `settings::test_connection_plan` (`settings/mod.rs:319–327`) returns `TestConnectionPlan::Disabled` **before** `resolve_config` (the only Keychain reader) when `!langfuse_enabled`; `test_langfuse_connection` (`lib.rs:237–245`) returns `TestConnectionResult::disabled()` on that arm — no probe, no secret store. UI mirrors it: the button is `disabled` while off (`langfuse-settings.ts:23`, `main.ts:444`). Test `disabled_test_connection_plan_short_circuits_without_touching_the_secret_store` uses a `TripwireSecretStore` that panics on any access. | ✅ |
| **Blocker 2** — Keychain read error silently flattened to "no entry", letting env fallback override a failed settings-first read | Distinguish `NoEntry` from backend error; **propagate a coarse secret-free error**; env fallback **only** when truly absent | `resolve_credentials` (`settings/mod.rs:189–212`) replaced `.ok().flatten()` with `match secrets.get(..)?`: `Ok(Some)`→stored, `Ok(None)`→env dev fallback, `Err`→short-circuit. `resolve_config` now returns `CmdResult<ImporterConfig>` and **all** call sites handle it (`langfuse/mod.rs:44` `?`, `test_connection_plan:326` `?`; remaining callers are tests). `SecretStoreError` strings are static & coarse (`secret_store.rs:62–64`). Tests `keychain_read_failure_is_propagated_not_masked_as_missing_credentials` (env creds present → still errors, secret-free) and `..._blocks_the_test_connection_plan_before_a_probe`. | ✅ |
| **Suggestion** — `set_langfuse_secret_repo` left a half-updated credential surface if the second write failed | A failed secret write must not leave a stored public key with no matching secret | `set_langfuse_secret_repo` (`settings/mod.rs:283–292`): on secret-write failure it deletes the public-key write (idempotent; its own error is intentionally swallowed so it can't mask the original **secret-free** error) and returns `Err`. Test `secret_write_failure_rolls_back_the_public_key_write` asserts neither key remains and the error is secret-free. | ✅ |

**No new attack surface.** The fix touches only Rust core + the settings TS module + one button
attribute. New strings are all static and secret-free: `TestConnectionResult::disabled()`
(`langfuse/mod.rs:99–105`), `testConnectionDisabledReason` (`langfuse-settings.ts:23`). The new
`TestConnectionPlan` derives `Debug`, but its embedded `ImporterConfig`/`Credentials` redact key
material in their own `Debug` impls (`config.rs:55–76`) — a plan can never render a secret. No
logging macro exists anywhere in `src-tauri/src` (grep clean), so the added error paths cannot leak
to logs.

**Refreshed scanner run (this recheck).**

| Scanner | Coverage | Result | Verdict |
|---|---|---|---|
| **semgrep** 1.165.0 | `src-tauri/src` + `src`, **25 git-tracked files**, **423 rules** (`p/rust`,`p/typescript`,`p/secrets`,`p/security-audit`) | **0 findings (0 blocking)** | **PASS** |
| **gitleaks** 8.30.1 | git history `ab17c5e~1..HEAD` (5 commits) | **0 leaks** | **PASS** |
| **gitleaks** 8.30.1 | working tree (`dir .`, 890 MB incl. build output) | **3 `generic-api-key` false positives**, all in **untracked, git-ignored** `src-tauri/target/{debug,release}/deps/libmuda-*.rmeta` (Rust compiler metadata for the `muda` Tauri dep — high-entropy bytes in a binary, not a secret) | **PASS** (see note) |
| **OSV-scanner** 2.3.8 | `Cargo.lock` (492) + `package-lock.json` (106) | 3 High (all npm **dev**-deps), 1 Med 6.9 (Rust `glib`), 1 Med 5.5 + 1 Low (npm dev), 16 no-CVSS (Rust GTK/unic transitive) | **PASS** (see §4) |
| **Trivy** 0.71.1 | `fs` (`vuln,secret`), HIGH/CRITICAL | **0 vulns, 0 secrets** | **PASS** |

**gitleaks working-tree note (new in this recheck).** The prior pass reported "0 leaks" on `dir .`
because the tree was clean of build output at the time. This recheck ran against a built tree, so
gitleaks additionally walked `src-tauri/target/` and flagged 3 hits — **all** in compiled `.rmeta`
binaries, **all** matched by `git check-ignore` (`.gitignore:3 src-tauri/target/`), **none**
git-tracked, **none** in the TASK-026 commits, **none** in the packaged source. The authoritative
"can a secret be committed" signal — the **git-history** scan — is **0 leaks** across all 5 commits.
These are heuristic false positives in disposable build artifacts; **not** an auto-fail. (Follow-up
ergonomics only: add `--no-git`-respecting config or a `.gitleaksignore` for `target/` to keep the
working-tree scan clean.)

No L2 auto-fail condition is met after the fixes. **Verdict unchanged: PASS.**

---

## 1. Scanner results (Tier 1 stack)

| Scanner | Version | Target / coverage | Result | Auto-fail condition | Verdict |
|---|---|---|---|---|---|
| **semgrep** | 1.165.0 | `src-tauri/src`, `src` — 408 rules (`p/rust`, `p/typescript`, `p/secrets`, `p/security-audit`) on 25 tracked files | **0 findings (0 blocking)** | any ERROR-severity | **PASS** |
| **gitleaks** | 8.30.1 | working tree (`dir .`) + git history of the 3 TASK-026 commits (`ab17c5e~1..HEAD`) | **0 leaks** | any detected secret | **PASS** |
| **OSV-scanner** | 2.3.8 | `src-tauri/Cargo.lock` (492 pkgs) + `package-lock.json` (106 pkgs) | 3 High (all npm **dev**-deps), 1 Med 6.9 (Rust), 16 no-CVSS (Rust transitive) | CVE ≥ 7.0 (CVSS) **reachable in artifact** | **PASS** (see §4) |
| **Trivy** | 0.71.1 | `fs` scan (`vuln,secret`), severity HIGH/CRITICAL | **0 vulns, 0 secrets** | HIGH or CRITICAL | **PASS** |

No L2 auto-fail condition is met for code shipped in this change.

---

## 2. SEC-009 — secret storage manual review (PASS)

The security-bearing core is Workstream A (`src-tauri/src/settings/`, `langfuse/config.rs`,
`langfuse/mod.rs`, `lib.rs`, `src/main.ts`, `src/langfuse-settings.ts`). All SEC-009 sub-checks
verified by reading the implementation:

| Check | Evidence | Status |
|---|---|---|
| No plaintext secret in **SQLite** | `settings/mod.rs` writes only `langfuse_base_url`/`_source`/`_environments`/`_enabled` to the `settings` table (`write_setting`, lines 90–97, 243–251). Credentials never touch SQLite. Test `secret_is_never_written_to_the_settings_table`. | ✅ |
| No secret in **logs** | No `println!`/`eprintln!`/`dbg!`/`log::`/`tracing::` anywhere in `src-tauri/src` (grep clean; only match is the benign `tauri_plugin_dialog` import). Frontend `console.error` (`main.ts:26`) logs only secret-free error objects. | ✅ |
| No secret in **evidence / exports** | `langfuse/store.rs` evidence/raw-trace tables and the CSV export (`lib.rs:153`) carry no credential columns/fields. `Secret`/`Credentials` redact in `Debug` (`config.rs:55–76`). | ✅ |
| No secret in **UI** | Secret field is `type="password" autocomplete="off"` with placeholder only and is **never pre-filled** (`main.ts:46`). UI shows `secretStateLabel` → `"set"/"not set"` (`langfuse-settings.ts:16`). | ✅ |
| No secret in **read APIs** | `get_langfuse_settings_repo` returns non-secret fields + `has_public_key`/`has_secret_key` booleans only — consults the store for presence (`is_some()`), never reads the value back (`settings/mod.rs:206–221`). Tests `get_settings_returns_presence_flags_never_secret_values`, `presence_flags_false_when_keychain_empty`. | ✅ |
| **Keychain** use | `KeyringSecretStore` (keyring v3, `apple-native`) under service `dev.vire.app`, distinct accounts `langfuse_public_key` / `langfuse_secret_key` (`secret_store.rs`). Keychain access stays in the Rust core — the CSP keeps the renderer off OS/network. | ✅ |
| **Env fallback is dev-only** | Resolver is Keychain/settings-first; env (`VIRE_LANGFUSE_*`, then bare `LANGFUSE_*`) is the explicit, comment-marked dev fallback (`settings/mod.rs:180–200`); README labels it a "marked dev fallback". | ✅ |
| **Presence flags only** | Renderer-facing `LangfuseSettings` struct carries no secret-typed field by construction (`settings/mod.rs:36–44`). | ✅ |
| Single redacting credential path | Settings-sourced credentials flow through the existing `Secret`/`Credentials` types; the secret is exposed only at `req.basic_auth(..., secret_key.expose())` (`api.rs:65–67`). No second un-redacted path. Test `settings_sourced_credentials_stay_redacted_in_debug`. | ✅ |
| Error strings are secret-free | `set_langfuse_settings`/`set_langfuse_secret` validation errors are static strings; `SecretStoreError` is coarse ("could not access the system keychain") and never echoes the value (`secret_store.rs:45–81`). Test `set_rejects_a_malformed_base_url_with_a_secret_free_error`. | ✅ |

**Clearing** a secret removes both Keychain entries idempotently (`clear_langfuse_secret_repo`),
after which the importer falls back to env (if set) else surfaces `auth_or_network_error` — never
zero (absence-≠-zero invariant honored).

---

## 3. Preserved invariants & cross-cutting checks (PASS)

- **SEC-002 loopback boundary preserved & now covers settings-sourced values.** `config.rs::validate_target`
  is unchanged: `source=local` requires a loopback host (`127.0.0.1`/`localhost`/`::1`); `cloud`
  forbids loopback. It runs on every `ReqwestLangfuseApi::new` (`api.rs:43–45`) and again inside
  `build_url`, which also re-checks the `/api/public/` path root, same host/scheme, and sets
  `redirect::Policy::none()`. Tests: `loopback_boundary_holds_for_settings_sourced_local`,
  `local_source_refuses_off_host_targets`, `cloud_off_host_allowed_only_on_explicit_source`,
  `test_connection_refuses_non_loopback_local_without_a_network_call`.
- **Test connection is bounded, coarse, secret-free.** `test_langfuse_connection` resolves config
  (one-shot Keychain read) under the DB lock, drops the lock, then runs the probe inside
  `run_bounded_result` with a **20 s** ceiling and a static secret-free timeout message (`lib.rs:231–244`,
  `262–267`). Verdicts are fixed strings (`reachable` / `unavailable` / `auth_or_network_error` /
  `unknown` / `invalid_config`); `from_api_error` emits its own stable copy and never includes the
  source error, response body, URL, or headers (`langfuse/mod.rs:66–130`). Tests
  `test_connection_verdicts_are_coarse_and_never_echo_the_error_message`,
  `run_bounded_result_times_out_with_the_supplied_secret_free_message`.
- **Disabled short-circuit before any network/Keychain read.** `import_langfuse_now` and the health
  snapshot check `settings::langfuse_enabled` first and return `disabled_snapshot` (`lib.rs:296–305`,
  `settings/mod.rs:287–293`). `source_health_snapshot` takes **no** `SecretStore`, so the disabled/
  persisted-state path structurally cannot read a credential. Disabled reports an explicit
  `disabled` state, never zero (`store.rs:313–325`). Test
  `disabled_short_circuits_to_a_disabled_snapshot_with_no_secret_store_access`.
- **CSP unchanged.** `tauri.conf.json` diff vs `main` touches only `bundle.icon`; the
  `connect-src ipc: http://ipc.localhost` policy is byte-identical — no new webview network surface.
- **Capabilities unchanged.** `git diff main...HEAD -- src-tauri/capabilities/*` is empty;
  `default.json` permissions remain `core:default`, `dialog:default`, `dialog:allow-save` (no new grant).
- **Build/package/icon docs leak-free.** gitleaks scanned the whole working tree (README, `docs/`,
  the generated icon binaries, and the generator script) → no secrets. The icon generator
  (`generate-vire-mark.mjs`) is dependency-free (Node built-in `zlib`/`fs` only) with no network,
  `exec`, or credential handling — pure pixel math. README install/run docs correctly instruct that
  the secret "is stored in the macOS Keychain, never in plaintext."

---

## 4. Advisory findings (non-blocking — documented per triage rubric)

These are real but do **not** meet the L2 auto-fail bar for code shipped by this change (not
reachable in the production artifact and/or not introduced by TASK-026). Recommended as follow-ups.

- **npm dev-deps `vite` 6.4.2 (GHSA-fx2h-pf6j-xcff, CVSS 8.2) and `esbuild` (GHSA-gv7w-rqvm-qjhr, CVSS 8.1).**
  Both are **`devDependencies`** (build tooling), both are **dev-server** vulnerabilities (Vite/esbuild
  serve-files / CORS), and both are **pre-existing** — not introduced by TASK-026 (this change adds
  only the Rust `keyring` crate). They were not surfaced by the TASK-024 scan because that scan
  covered `Cargo.lock` only; this review additionally scanned `package-lock.json`, closing that
  coverage gap. **Not reachable in the deliverable:** Workstream C's packaged `.app` runs with **no
  dev server** (Rust binary + static `dist/`), so the dev-server attack surface is absent at runtime.
  → **Recommend** a separate housekeeping bump: `vite` → 6.4.3 (fixes both vite CVEs) and the
  transitive `esbuild` → 0.28.1. Trivy reported 0 HIGH/CRITICAL for the same lockfiles (DB
  classification differs); OSV is the stricter signal and is the basis for this advisory.
- **Rust `glib` 0.18.5 — RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g, CVSS 6.9 (Medium).** Below the
  7.0 auto-fail threshold; pre-existing Tauri framework transitive dep (also present at TASK-024);
  fixed in glib 0.20.0. Track at the framework-upgrade level.
- **16 no-CVSS RustSec advisories** on `atk`, `gdk*`, `gtk*`, `gtk3-macros`, `proc-macro-error`,
  `unic-*`. Mostly the GTK/Linux backend (unused on the macOS-only target); all pre-existing
  transitive deps, none added by TASK-026. Advisory only.
- **`keyring` v3 (new dependency, the one dep TASK-026 adds).** No OSV/RustSec advisory against
  `keyring` or its added transitive crates (Cargo.lock package count 487→492; none of the +5 appear
  in the advisory list). Native binding to the macOS Security framework — appropriately flagged in
  the PR per DEC-026; `apple-native` is the only backend compiled (no Linux secret-service path).

No advisory here blocks the gate.

---

## 5. Verdict

**SEC STATUS: PASS** (re-confirmed after the atomic credential-pair fix `935877f` — see §0′, with
the earlier SW-4 fixes in §0). No L2 auto-fail condition is met for code introduced by this change:
gitleaks history clean (7 commits scanned), semgrep 0/0, Trivy 0 HIGH/CRITICAL, and no in-artifact
dependency at CVSS ≥ 7.0. SEC-009 is satisfied — the Langfuse secret is confined to the macOS
Keychain and never reaches SQLite, logs, evidence, exports, the UI, or any read API; presence is
exposed only via boolean flags. SEC-002 loopback and SEC-003 redaction are preserved and now also
guard settings-sourced values. The four SW-4 fixes (disabled Test-connection short-circuit,
Keychain-error propagation, partial-write rollback, and the **atomic credential-pair rollback on
failed replacement**) are correctly implemented and tested: a failed replacement now restores the
prior pair, so the resolver can never combine an env-fallback public key with a stale Keychain
secret. Test connection is bounded, coarse, and secret-free; CSP and capabilities are unchanged.

**No design-level escalation** to BA-flow Architect (DEC-026 Keychain model is sound and correctly
implemented). **No code-level FAIL** routed to the developer — the prior SW-4 blockers are resolved.

Advisory follow-ups recommended (not blocking): bump `vite`/`esbuild` (dev-only, dev-server, not in
the shipped artifact); track the framework-level `glib`/GTK advisories at upgrade time; and add a
`target/` exclusion for the gitleaks working-tree scan so it matches the (clean) history scan.

**Handoff:** PASS → SW-4 (Code review) is now also resolved; when both gates are green, route to
SW-6 (Release Manager).
