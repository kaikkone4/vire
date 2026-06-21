# Security Review — TASK-044 keychain-public-key-to-settings (SW-5)

**Date**: 2026-06-21 (recheck after Architect fix `db2eeef` + SW-3 QA PASS)
**Branch / PR**: feat/task-044-keychain-public-key-to-settings · PR #32
**Commit (task-044 scope)**: `1160f04` (impl) → `db2eeef` (Architect-decided fix) → `f552b64` (QA)
**Tier**: L2 (secrets + CVE≥7 + Trivy + semgrep ERROR)
**Verdict**: **PASS (security gate) — release no longer blocked from the security side.**

Relocates the non-secret Langfuse public key (HTTP Basic-Auth username) from the macOS
Keychain into the plaintext SQLite `settings` table; the secret key stays Keychain-only.
This recheck re-reviews the **full** change after `db2eeef` applied the binding
`arch-review.md` Addendum (Decision 1 pair-level env fallback; Decision 2 two-store
consistency contract). It **supersedes** the prior sec.md verdict, which was PASS-but-gated
on the open Architect escalation — that escalation is now **resolved** (see §3).

---

## 1. Scanner results (Tier 1 stack)

| Scanner | Scope | Auto-fail condition | Result |
|---|---|---|---|
| **semgrep** 1.166.0 | changed Rust files (`settings/mod.rs`, `settings/tests.rs`, `lib.rs`), `--config=auto --severity=ERROR` | any ERROR finding | **0 findings** ✅ |
| **gitleaks** 8.30.1 | full task-044 source range `1160f04^..HEAD` (4 commits) + changed source dir | any detected secret | **0 in commits/source** ✅ (3 FPs — see §1.1) |
| **OSV-scanner** 2.3.8 | `src-tauri/Cargo.lock` + `package-lock.json` | CVE ≥ 7.0 CVSS | **no CVSS ≥ 7.0** ✅ (see §1.2) |
| **Trivy** 0.71.1 | fs vuln scan, `Cargo.lock` + `package-lock.json`, HIGH/CRITICAL | HIGH or CRITICAL | **0 HIGH/CRITICAL** ✅ |

**No auto-fail condition hit on any scanner.**

### 1.1 gitleaks false positives (not blocking)

The authoritative git-log scan (`1160f04^..HEAD`, all 4 task-044 commits incl. `db2eeef`)
reports **no leaks found**. The 3 filesystem hits are all in **gitignored build artifacts**
(`git ls-files` → not tracked), not source and not in any task-044 diff:

- `src-tauri/target/debug/deps/libmuda-*.rmeta` (×2)
- `src-tauri/target/release/deps/libmuda-*.rmeta` (×1)

All match the `generic-api-key` heuristic against the literal keybinding string
`` `shift+alt+KeyQ` `` compiled into the `muda` menu crate — not credential material. Same
FPs triaged in the prior review.

### 1.2 OSV advisories (advisory, pre-existing, not blocking)

18 advisories across transitive Tauri/GTK3 deps. **task-044 changed no lockfile** — the full
source range `1160f04^..HEAD` touches only `src-tauri/src/{settings/mod.rs,settings/tests.rs,lib.rs}`
(no `Cargo.toml`/`Cargo.lock`/`package*.json`), so none are introduced by this change. Only
one advisory carries a CVSS vector:

- `glib` — GHSA-wrw7-89jp-8q8g / RUSTSEC-2024-0429 — CVSS:4.0 vector `AV:N/AC:L/.../VI:L/...`
  (**well below the 7.0 threshold**; fix in glib 0.20.0).

The remaining 17 (atk/gdk/gtk family, `proc-macro-error`, `unic-*`) are **unmaintained-crate**
RUSTSEC advisories carrying **no CVSS vector** — they do not meet the `CVE ≥ 7.0 (CVSS)`
auto-fail criterion. This is the same GTK3/transitive set triaged under TASK-043 and tracked
there for remediation, gated on the upstream Tauri GTK4 migration.

---

## 2. Manual secure-code review — full re-review after `db2eeef`

Reviewed `settings/mod.rs` (resolver + set/clear), `settings/tests.rs` (security tests), and
`lib.rs` IPC commands against the binding `arch-review.md` Addendum. The implementation
matches the Addendum's required semantics line-for-line.

| Property | Finding (code refs `settings/mod.rs`) | Verdict |
|---|---|---|
| **Public = non-secret in SQLite** | Public key written via `write_setting(conn, KEY_PUBLIC_KEY, …)` to the plaintext `settings` table; this is the HTTP Basic-Auth username (non-secret), approved as such in `arch-review.md`. | ✅ |
| **Secret = Keychain-only / presence-only** | The secret value (`secret_key`) flows **only** to `secrets.set(SECRET_KEY_ACCOUNT, …)` (L430). Audited every `write_setting` call — all six target non-secret keys (`KEY_IMPORT_RANGE/BASE_URL/SOURCE/ENVIRONMENTS/ENABLED/PUBLIC_KEY`); none carries the secret. `LangfuseSettings` exposes `has_secret_key` presence only (no secret field). T5 scans every settings row for the secret value + asserts absence. SEC-009 / C2 hold. | ✅ |
| **Pair-level env fallback (D1) — no mixed credentials** | `resolve_credentials` (L300–333) reads both stores strictly, then matches the pair as a **unit**: both present ⇒ stored pair (env never consulted); both absent ⇒ env pair iff **both** env keys set; **exactly one store ⇒ `None`** (lone key discarded, env not consulted for the missing side). "No mixed-source pair" is now a structural property of the match (DEC-026), not a per-field accident. Verified by T-PAIR-A/B/C and the explicit "no mixed pair" assertions. | ✅ |
| **No env downgrade on a real read failure (C3)** | Public via `read_setting_strict` (`.optional()?` ⇒ `Ok(None)` only for a genuinely absent row, `Err` for a real failure) with `.map_err(…)?` short-circuiting **before** the env branch; secret side `secrets.get(…).map_err(\|e\| e.0)?` likewise. A broken store can never be downgraded to env. | ✅ |
| **Error messages secret-free** | `INCONSISTENT_SET_ERR` / `INCONSISTENT_CLEAR_ERR` are fixed strings; `secret_err.0` is the coarse `SecretStoreError` string (driver error discarded via `map_err`); the public-read error is a fixed string; no input is echoed. Tests assert the INCONSISTENT errors and the settings view contain no secret needle (L285/314/878–900). | ✅ |
| **set/clear residual one-store windows inert** | `set`: SQLite first, Keychain last; on Keychain failure the public row is restored to prior (reinstate or delete), so the pair ends both-new or both-prior. `clear` (reordered to **SQLite-first** per Decision 2): clears public first, aborts before Keychain on its failure (both remain); on Keychain-delete failure restores the non-secret public key from `prior_public` (the only recoverable artifact — a deleted secret is never re-read, SEC-009). The single one-store state reachable via a returned result requires a catastrophic local SQLite failure and is surfaced explicitly as the secret-free `INCONSISTENT_*_ERR`, never swallowed. Any such half-state (incl. crash between mutations) is rendered **inert** by the D1 resolver (one-store ⇒ `None`) ⇒ at worst "not configured," fixed by one re-save. Verified by T-SET-ROLLBACK-FAIL ×2 and T-CLEAR-COMP. | ✅ |
| **No new deps / egress / capabilities** | Full range `1160f04^..HEAD` touches no `Cargo.toml`/lockfile/`tauri.conf.json`/`capabilities/*`/entitlements/`.plist`. `lib.rs` change adds only the **Tauri-injected** `State<AppState>` handle to `set_/clear_langfuse_secret`; the renderer argument shape (`{publicKey, secretKey}`) is unchanged → no new IPC surface, no new network primitive. Only SQLite + Keychain I/O. | ✅ |

---

## 3. Prior release gate — RESOLVED

The prior sec.md (§3) recorded PASS-but-blocked: SW-4 had escalated the DEC-026
credential-pair-integrity concern (per-field env fallback → mixed pair) and the two-store
failure contract to the Architect, and I gated release on that decision. That decision has
**landed** (`arch-review.md` Addendum, 2026-06-21: "PASS — design decided, no task split, no
BA escalation"), and `db2eeef` implements it exactly:

- **Decision 1 (pair-level env fallback)** closes the mixed-source-pair window — the precise
  low-severity integrity weakening I flagged. Now structurally impossible (§2, T-PAIR-A/B/C).
- **Decision 2 (two-store consistency contract)** — fragile Keychain mutation last + reliable
  SQLite compensation, clear reordered to SQLite-first, INCONSISTENT errors never swallowed.

There is **no remaining security-side release block.** No FAIL-DESIGN is raised: the design
question is decided and correctly realized.

---

## 4. Verdict

**PASS (security gate).** No scanner/SAST/secrets auto-fail condition is hit. The trust
boundary (secret in Keychain, non-secret public key in local plaintext settings) was approved
in `arch-review.md` and is correctly realized. SEC-009 and C1–C4 hold; the DEC-026 pair
invariant is now structural (no mixed-source pair, no usable one-store credential); all error
paths are secret-free. No new deps/egress/capabilities. Advisory OSV items (max CVSS 4.0 with
a vector; rest no-CVSS unmaintained-crate) are pre-existing, not introduced here, tracked under
TASK-043.

**Routing**: PASS → proceed. Hold for SW-4 (sw-code-reviewer) before release to SW-6
(sw-release-manager). SW-6 RELEASE.md must carry the two user-facing notes from the handoff:
(1) one-time re-save for existing installs (public key absent in settings until re-saved);
(2) pair-level env behavior change (one key in a store + the other in env now resolves to
`None` — env is a whole-pair dev override).
