# Security review — TASK-041 (macOS Keychain repeated-prompt stability)

**Gate:** SW-5 security-agent · **Tier:** L2 · **Scope:** security review of the SW-2
research/design recommendation **F1 + F2a (F3 dropped)** — Apple Developer ID
signing/notarization, keyring v3 ACL behaviour, and moving the Langfuse **public** key out of
the Keychain into the `settings` table. **No code implemented in this task** (docs/research
only). This review (a) grants/denies the SEC sign-off on public-key secrecy that gates
TASK-044, (b) judges whether F2a is acceptable, and (c) records the conditions carried to the
downstream implementing tasks.

**Verdict: PASS.** The recommendation is security-sound; the SEC sign-off the handoff asked
for is **GRANTED** (the Langfuse public key is **not** secrecy-sensitive). No auto-fail
scanner condition hit. F2a and F1 are approved **with conditions for the implementing tasks**
(TASK-044 and the TASK-042 signing foundation respectively) — these are verified at those
tasks' own SW-5 gates, not blockers on this research gate.

---

## Tier 1 scanner results (L2: secrets + CVE≥7 + Trivy + semgrep ERROR)

| Scanner | Scope run | Auto-fail condition | Result |
|---|---|---|---|
| **gitleaks** 8.30.1 | full repo + git history (182 commits) | any detected secret | **CLEAN** — `no leaks found` |
| **semgrep** 1.166.0 | `p/rust` + `p/secrets`, ERROR severity, on `settings/`, `langfuse/`, `lib.rs` (12 files) | any ERROR finding | **CLEAN** — 0 findings, 0 errors |
| **OSV-scanner** 2.3.8 | `src-tauri/Cargo.lock` (492 pkgs) + `package-lock.json` (106 pkgs) | CVE ≥ 7.0 CVSS | **PASS** — highest scored = glib 0.18.5 **CVSS 6.9** (RUSTSEC-2024-0429, Medium) < 7.0; no ≥7.0 finding |
| **Trivy** 0.71.1 | `fs` scan (vuln+secret+misconfig), HIGH/CRITICAL | HIGH or CRITICAL | **CLEAN** — 0 HIGH/CRITICAL, 0 secrets, 0 misconfig. No container image exists (Tauri desktop app) ⇒ image scan N/A |

**No auto-fail conditions hit per the triage rubric.**

### OSV advisory detail (informational — not blocking)
The 17 OSV advisories are the **pre-existing repo baseline** (the subject of TASK-043), not
introduced here: **TASK-041 adds zero code and zero dependencies.**
- **glib 0.18.5 — RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g, CVSS 6.9 (Medium).** Below the
  7.0 auto-fail line. Fixed in glib 0.20.0; transitively pinned by the GTK 0.18 stack. Advisory.
- **gdk / gtk / atk / gdkx11 / gdkwayland (RUSTSEC-2024-041x) — Unknown CVSS.** Unmaintained /
  unsound advisories for the **GTK3 Linux bindings**. Vire is **Mac-first**
  (`src-tauri/Cargo.toml:29`, `apple-native` only); these crates are not on the macOS code
  path. Advisory.
- **unic-\* (RUSTSEC-2025-007x/008x/0100), proc-macro-error (RUSTSEC-2024-0370) — Unknown
  CVSS.** Unmaintained-crate notices, transitive. Advisory.

All advisory; tracked under TASK-043 (dependency advisory bump). None gates TASK-041.

---

## Manual security review of the recommendation

### Decision 1 — Is the Langfuse **public key** non-secret? → **YES (sign-off GRANTED)**

This is the prerequisite the handoff flagged as blocking TASK-044/F2a. Confirmed against the
code:

- **Role in auth.** The public key is the **username** of HTTP Basic Auth, paired with the
  secret key as the password: `req.basic_auth(&creds.public_key, Some(creds.secret_key.expose()))`
  (`src-tauri/src/langfuse/api.rs:81`). This is the canonical Langfuse credential model
  (`pk-lf-*` public project identifier + `sk-lf-*` secret). The public key **cannot
  authenticate alone** — every protected request requires the secret key. Disclosure of the
  public key in isolation grants **no** API access and breaks **no** confidentiality property.
- **Same sensitivity class as existing plaintext settings.** `base_url`, `source`,
  `environments`, `langfuse_enabled` already live in the plaintext SQLite `settings` table
  (`settings/mod.rs:28-31`) and are documented non-secret. The public key is a
  project/installation identifier of the same class.
- **The current redaction is defense-in-depth, not a secrecy requirement.**
  `Credentials::Debug` redacts even the public key (`config.rs:70-72`, "Never print the real
  key material, not even the public key"). That is conservative data-minimisation, **not**
  evidence the value is secrecy-sensitive — no security control in the codebase depends on the
  public key staying hidden. SEC-009's non-echo guarantee is explicitly scoped to the **secret
  key** (`mod.rs:338` "SEC-009 guards the secret key only").

**Sign-off:** nothing in the codebase relies on the public key's secrecy; moving it out of the
Keychain weakens no auth or confidentiality property. **GRANTED.**

### Decision 2 — Is TASK-044 / F2a acceptable? → **YES, with conditions**

F2a moves `langfuse_public_key` from the Keychain to the `settings` table (drops one Keychain
item/ACL ⇒ fresh-launch prompts 2→1). Acceptable: storing a non-secret identifier in the
plaintext local settings store is the same posture already used for `base_url` et al. The
**secret key stays in the Keychain**. Conditions for the TASK-044 implementation (verified at
TASK-044's SW-5 gate, not here):

- **C1 — credential-pair integrity across two stores (load-bearing).** The current code
  maintains an atomic-pair invariant so the resolver never combines a public key from one
  source with a **stale secret** from another (`mod.rs:330-359` documents this exact failure
  mode: a half-written pair lets `resolve_credentials` pair an env-fallback public key with a
  stale Keychain secret). After F2a the public key lives in **SQLite** while the secret stays
  in the **Keychain** — two stores. TASK-044 must preserve atomic **set** and atomic **clear**
  across both stores, and the env-fallback ordering in `resolve_credentials`
  (`mod.rs:233-244`) must not silently pair a settings/env public key with a stale Keychain
  secret (or vice-versa). This is the one substantive risk in F2a.
- **C2 — keep the SEC-009 boundary.** `get_langfuse_settings_repo` returns presence flags only
  (`mod.rs:262-272`). Returning the public-key **value** in the renderer view is acceptable
  (it is non-secret), but the **secret key must remain presence-flag-only**. Do not generalise
  "settings now carries the public key" into "settings carries credentials" — keep the two on
  separate paths.
- **C3 — no new public-key leak surface (advisory).** Once the public key sits in plaintext
  settings, ensure it is not newly written into shared/exported artifacts (evidence exports,
  logs, support bundles). The value is non-secret, but the same data-minimisation instinct
  behind the current Debug redaction argues against broadcasting a project identifier. Advisory.
- **C4 — clear path covers both stores.** `clear_langfuse_secret_repo` (`mod.rs:364-367`)
  currently deletes both Keychain entries. After F2a it must delete the settings-table
  public-key row **and** the Keychain secret, leaving neither store populated. (Part of C1.)

### Decision 3 — F1 (Apple Developer ID signing + notarization) → **endorsed, with conditions**

F1 is a net security **improvement**: a stable Developer ID signature + notarization + hardened
runtime yields a Gatekeeper-clean, tamper-evident artifact and a stable designated requirement
so the Keychain ACL persists. No objection. Conditions for the TASK-042 signing-foundation
implementation (verified at that task's gate):

- **C5 — signing-secret handling (hard).** The Apple signing assets (`.p12` / App Store
  Connect `.p8`, `APPLE_CERTIFICATE_PASSWORD`, app-specific password) are high-value secrets.
  They must live **only** in the CI secret store, never committed, never logged — same posture
  class as SEC-009 (ops-review §124 already states this; endorsed as a hard condition).
  gitleaks must stay green after CI wiring lands.
- **C6 — minimal entitlements / hardened runtime (advisory→condition).** Keep
  `hardenedRuntime` at its default `true`. If an entitlements file is added, keep it minimal;
  avoid broad escapes (`disable-library-validation`, `allow-unsigned-executable-memory`)
  absent a proven need. Note F3 (`keychain-access-groups`) is correctly **ruled out**, so no
  keychain-access-group entitlement should be introduced.

### Decision 4 — Keychain ACL behaviour (F3 dropped) → **no security concern**

The research correctly establishes keyring v3 `apple-native` uses the **file-based login
keychain** with a per-item ACL bound to the designated requirement, **no `kSecAttrAccessGroup`**
(ops-review Finding 2). Security implication: a stable Developer ID identity makes "Always
Allow" persist across future builds signed with the same Team ID. This is the **standard,
expected** macOS trust model for any signed app and does not weaken the secret's protection
(the secret stays behind the Keychain ACL + the user's scoped grant). Dropping F3 removes an
unnecessary entitlement — a positive for least-privilege. No concern.

---

## Escalations (design-level)

**None.** The recommendation does not move a trust boundary incorrectly, remove an auth layer,
or contradict the BA-flow architecture plan — the DEC-026 credential-pair model and SEC-009
non-echo guarantee are preserved (and reinforced by C1/C2). No `feedback_to_ba[]`; no
escalation to ba-architect.

---

## Security blockers / conditions summary

| ID | Applies to | Type | Statement |
|----|-----------|------|-----------|
| **SEC sign-off** | TASK-044 / F2a | **GRANTED** | Langfuse public key is non-secret (Basic-Auth username; no security property depends on its secrecy). Unblocks TASK-044. |
| **C1** | TASK-044 | condition (hard) | Preserve atomic credential-pair set/clear across SQLite (public) + Keychain (secret); no mixed-source pair via env fallback (`mod.rs:233-244`, `330-359`). |
| **C2** | TASK-044 | condition | Secret key stays presence-flag-only (SEC-009); only the public key may surface as a value. |
| **C3** | TASK-044 | advisory | Don't newly write the public key into exports/logs/support bundles. |
| **C4** | TASK-044 | condition | `clear` must wipe both the settings public-key row and the Keychain secret. |
| **C5** | TASK-042 (F1) | condition (hard) | Apple signing secrets only in CI secret store — never committed/logged; gitleaks stays green. |
| **C6** | TASK-042 (F1) | advisory→condition | `hardenedRuntime` default true; minimal entitlements; no `keychain-access-groups` (F3 ruled out). |

C1–C6 are conditions on the **downstream implementation** tasks and are verified at their own
SW-5 gates. They do **not** block this research/design gate.

---

## Checks performed

- 4/4 Tier 1 scanners run; results + auto-fail evaluation in the table above. No auto-fail hit.
- Manual review of the credential code path the recommendation touches:
  `secret_store.rs` (full), `settings/mod.rs:1-70,220-410` (resolver + atomic-pair + clear),
  `langfuse/config.rs:55-76` (Credentials/Secret redaction), `langfuse/api.rs:74-94`
  (Basic-Auth usage of public/secret keys).
- Public-key non-secrecy sign-off decided from code evidence (Basic-Auth username role;
  SEC-009 scoped to secret only).
- F2a / F1 / F3 / ACL-behaviour each assessed; conditions recorded against the implementing task.
- No source/config/CI files modified by this gate (docs-only; `git status` shows changes only
  under `openspec/changes/` task dirs).

## Changed paths (this gate)

```
openspec/changes/task-041-keychain-access-stability/
  sec.md       (this file — SW-5 output)
  handoff.md   (updated: SW-5 gate result)
```

## Gate verdict

**SEC STATUS: pass** — no auto-fail scanner condition hit; public-key non-secrecy sign-off
GRANTED (unblocks TASK-044); F2a and F1 approved with conditions C1–C6 carried to the
implementing tasks; no design-level escalation.

Route (per design): wait for SW-4 (code-reviewer), then `sw-release-manager`. The TASK-044
(F2a) and TASK-042 (F1 signing foundation) implementations each carry the conditions above
into their own SW-5 gate.
