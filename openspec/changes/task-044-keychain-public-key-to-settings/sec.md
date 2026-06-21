# Security Review — TASK-044 keychain-public-key-to-settings (SW-5)

**Date**: 2026-06-21
**Branch / PR**: feat/task-044-keychain-public-key-to-settings · PR #32
**Commit (task-044 scope)**: 1160f04
**Tier**: L2 (secrets + CVE≥7 + Trivy + semgrep ERROR)
**Verdict**: **PASS (security gate)** — but release is gated on SW-4's open Architect
escalation (DEC-026 credential-pair integrity); see §3.

Relocates the non-secret Langfuse public key (HTTP Basic-Auth username) from the macOS
Keychain into the plaintext SQLite `settings` table; the secret key stays Keychain-only.
Backend-only (Rust), renderer IPC contract unchanged. Realizes F2a under the SW-5
public-key non-secrecy sign-off already granted in `arch-review.md`, and conditions C1–C4
from TASK-041 `sec.md`.

---

## 1. Scanner results (Tier 1 stack)

| Scanner | Scope | Auto-fail condition | Result |
|---|---|---|---|
| **semgrep** 1.166.0 | changed Rust files (`settings/mod.rs`, `settings/secret_store.rs`, `lib.rs`), `--config=auto --severity=ERROR` | any ERROR finding | **0 findings** ✅ |
| **gitleaks** 8.30.1 | task-044 commit (`1160f04^..1160f04`) + changed source dir | any detected secret | **0 in commit/source** ✅ (3 FPs — see §1.1) |
| **OSV-scanner** 2.3.8 | `src-tauri/Cargo.lock` (492 pkgs) + `package-lock.json` (106 pkgs) | CVE ≥ 7.0 CVSS | **no CVSS ≥ 7.0** ✅ (see §1.2) |
| **Trivy** 0.71.1 | fs vuln scan, `Cargo.lock` + `package-lock.json`, HIGH/CRITICAL | HIGH or CRITICAL | **0 HIGH/CRITICAL** ✅ |

### 1.1 gitleaks false positives (not blocking)

The 3 hits are all in **gitignored build artifacts**, not source and not in the task-044 diff:

- `src-tauri/target/debug/deps/libmuda-*.rmeta` (×2)
- `src-tauri/target/release/deps/libmuda-*.rmeta` (×1)

All match the `generic-api-key` heuristic against the literal keybinding string
`` `shift+alt+KeyQ` `` compiled into the `muda` menu crate — not credential material.
The task-044 commit scan and the source files are clean.

### 1.2 OSV advisories (advisory, pre-existing, not blocking)

17 advisories across transitive Tauri/GTK deps. **task-044 changed no lockfile**
(`git show 1160f04` touches no `Cargo.toml`/`Cargo.lock`/`package*.json`), so none are
introduced by this change. Highest CVSS-scored finding:

- `glib` 0.18.5 — RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g — **CVSS 6.9** (< 7.0 threshold; fix in 0.20.0)

The remaining 16 (atk/gdk/gtk/gtk-sys/gtk3-macros/gdkx11/gdkwayland-sys,
`proc-macro-error`, `unic-*`) are **unmaintained-crate** RUSTSEC advisories carrying no
CVSS vector — they do not meet the `CVE ≥ 7.0 (CVSS)` auto-fail criterion. These are the
same GTK3/transitive set already triaged under TASK-043 (dependency advisory bump) and
remain tracked there for separate remediation, gated on the upstream Tauri GTK4 migration.

**No auto-fail condition hit on any scanner.**

---

## 2. Manual secure-code review — C1–C4 + SW-5 conditions

Reviewed `src-tauri/src/settings/mod.rs`, `settings/secret_store.rs`, `lib.rs` (diff
`1160f04`) and the SEC-relevant tests in `settings/tests.rs`.

| # | Condition | Finding | Verdict |
|---|---|---|---|
| C1 | Atomic credential pair across settings + Keychain | `set_langfuse_secret_repo` captures `prior_public` via the **strict** read (a real read failure aborts before any write); writes public→`settings` first (cheap, reliable, prompt-free), then secret→Keychain. On Keychain `set` failure it restores the settings row to its prior value (reinstate or `clear_setting`) and returns the coarse `e.0`. Fragile store written last ⇒ the only rollback ever needed is a local SQLite rewrite/delete. Pair ends both-new or both-prior, never one-store. | ✅ |
| C2 | Secret stays Keychain / presence-only | Secret only ever flows to `secrets.set(SECRET_KEY_ACCOUNT, …)`. `LangfuseSettings` carries presence flags (`has_public_key`/`has_secret_key`) — no secret field. The only `write_setting` calls target `KEY_PUBLIC_KEY`. T5 scans **every** settings row for the secret value and asserts absence + no credential-bearing key name. | ✅ |
| C3 | Strict DB read — no env fallback on a real read failure | `resolve_credentials` reads public via `read_setting_strict` (`.optional()?` ⇒ `Ok(None)` only for a genuinely absent row, `Err` for a real failure) with `.map_err(…)?` short-circuiting **before** the env branch; secret side `secrets.get(…).map_err(\|e\| e.0)?` likewise. A broken store can never be downgraded to env, so no mixed-source pair. | ✅ |
| C4 | Atomic clear — Keychain first, abort before settings on failure | `clear_langfuse_secret_repo` deletes the Keychain secret first and returns on its error before touching `settings`, preserving the prior consistent pair; then `clear_setting(KEY_PUBLIC_KEY)`. | ✅ |

| SW-5 condition | Finding | Verdict |
|---|---|---|
| No public/secret leaks in errors | All `SecretStoreError` messages are coarse, hardcoded strings; underlying driver errors discarded via `map_err(\|_\| …)`. The public-read error in the resolver is a fixed string. Public key is non-secret regardless. | ✅ |
| No stale env/settings public paired with a legacy Keychain secret | Rollback always restores a consistent pair; legacy Keychain public-key item is best-effort `secrets.delete(PUBLIC_KEY_ACCOUNT)` on both set (post-success) and clear, so it is never read again and never re-pairs. `PUBLIC_KEY_ACCOUNT` is now write/delete-only (never read). | ✅ |
| Clear wipes both stores | C4 — secret + settings row both removed (+ legacy item). | ✅ |
| No new deps / egress / capabilities | `git show 1160f04` touches no `Cargo.toml`/lockfile/`tauri.conf.json`/`capabilities/*`/entitlements. No network primitives added (sole `http` match is the "HTTP Basic-Auth username" doc comment). Only SQLite + Keychain I/O. | ✅ |

---

## 3. Concurrence with SW-4 design escalation (DEC-026 credential-pair integrity)

SW-4 Code Review ran in parallel and returned **ESCALATE** (`review.md`) on two cross-store
correctness gaps. One overlaps the Security remit (DEC-026 credential-pair integrity), so I
record my security position here rather than opening a competing escalation:

- **Per-field env fallback can assemble a mixed-source pair** (`mod.rs` `resolve_credentials`,
  ~L260–290). When exactly one store is **genuinely absent**, that field independently falls
  back to its env var, so `settings` public absent + Keychain secret present + env public set
  ⇒ env-public paired with a stale Keychain secret. This is the documented existing-install
  upgrade state (no auto-migration; user re-saves once).
  - **Security severity: LOW, not a security auto-fail.** No secret disclosure: SEC-009/C2
    hold (the secret never enters plaintext and is never echoed), and the assembled pair only
    ever reaches the user's own configured `base_url` (no third-party egress, no cross-tenant
    leak). C3 still holds — a genuine *read failure* (vs absence) is never downgraded to env.
  - **It is, however, a real credential-integrity weakening of the DEC-026 pair invariant**,
    and the spec sanctions it only because the no-mixed-pair guarantee is written per-field
    (failure-scoped) rather than pair-level. That is a **design decision for the Architect**:
    whether the env fallback should be pair-level (both-from-env or neither).
- **Compensation/atomicity on SQLite failure** (set rollback `let _ = …`; clear deletes the
  Keychain secret before the settings row with no recovery) is a reliability/integrity gap,
  not a secret-handling defect. No security impact beyond the same low-severity one-store →
  mixed-pair window above.

I **concur with routing to the Architect** for the env-fallback semantics + two-store failure
contract. **This change must not release until that decision lands**, regardless of the
security gate result below.

---

## 4. Verdict

**PASS (security gate)** — no scanner/SAST/secrets auto-fail condition is hit; SEC-009 and
C2/C3 (secret non-echo, secret-never-in-plaintext, no env downgrade on a real read failure)
are verified in code and tests; no secret disclosure, no new deps/egress/capabilities. The
trust boundary (secret in Keychain, non-secret public key in local plaintext settings) was
approved in `arch-review.md` and is correctly realized. Advisory items (OSV transitive
GTK/unic advisories, max CVSS 6.9) are pre-existing, not introduced here, tracked under
TASK-043.

The DEC-026 credential-pair-integrity concern in §3 is **low security severity (no
disclosure)** and is **already escalated by SW-4 to the Architect** — I do not raise a
separate FAIL-DESIGN, but I gate release on that decision.

**Routing**: the change is **blocked at the Architect** via SW-4's ESCALATE (and my §3
concurrence). It does **not** proceed to SW-6 until the Architect resolves env-fallback
semantics + the two-store failure contract, then SW-2 reworks and SW-3/SW-4/SW-5 re-run as
needed. SW-6 must also add the RELEASE.md one-time re-save note (existing installs, no
auto-migration).
