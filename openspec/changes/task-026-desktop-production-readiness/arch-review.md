# Architecture Review — TASK-026 (desktop production readiness)

**Reviewer:** SW Architect (SW-1)
**Change:** `openspec/changes/task-026-desktop-production-readiness/`
**Tier:** L2
**Verdict:** **PASS** — one cohesive "production readiness" change with three sequenced, routable
workstreams. No architectural split required. One design decision recorded (**DEC-026**, secret
storage), one security control recorded (**SEC-009**, secure secret storage), both routed to BA for
the canonical logs.

---

## 1. Scope vs. architecture plan

TASK-026 is production-readiness, not new product behavior. It aligns with and operationalizes the
architecture plan rather than changing it:

- **`03_architecture_plan.md:147`** — "credentials … provided by local configuration/**secret
  storage** and must never be printed, exported, committed, logged…". TASK-026 realizes the **secret
  storage** half (macOS Keychain) for the in-app flow. **Honored.**
- **`03_architecture_plan.md:169`** — "Setup should show the active base URL, environment(s), last
  import, local stack health, and credential/trace-health status **without displaying secrets**." The
  new settings panel shows exactly these and never reads a secret back. **Honored.**
- **DEC-020 / DEC-022** — local Docker Langfuse on loopback is the default; Cloud is an explicit
  override. Defaults are unchanged; `validate_target` (SEC-002) still gates `local` to loopback, now
  for settings-sourced values too. **Preserved.**
- **DEC-025** — `VIRE_*` env config is **retained** as a clearly-marked dev fallback, not removed. The
  TASK-025 `.env.example` stays valid for developers. **Consistent.**
- **TASK-019 `design.md` §3** already specified credentials "from **Keychain** or chmod-600 `.env`."
  DEC-026 is the realization of that anticipated path, not a new direction.

No data-model component boundary (`§3`: capture / observer / importer / store / classifier / review /
exporter) is crossed. The importer's **evidence** logic is untouched; only its **config source**
changes (env-only → settings-first, env-fallback).

## 2. Contract verified against the live code (authoritative source)

| Fact relied on | Verified at |
| --- | --- |
| Importer loads config from env only | `langfuse/config.rs` `from_env`/`public_from_env`; `langfuse/mod.rs:39,65` |
| Redacting `Secret` / `Credentials` types already exist | `langfuse/config.rs:43–76` |
| Loopback allowlist invariant (SEC-002) | `langfuse/config.rs::validate_target` / `build_url` |
| SQLite `settings` table already present | `lib.rs:53` (`CREATE TABLE … settings(key,value)`) |
| Bounded-command pattern for hang safety | `lib.rs:217` `run_bounded` (reuse for Test connection) |
| Settings view + source panel exist (UI seam) | `src/main.ts:41,43` `renderSettings`/`sourcePanel` |
| `bundle.icon` empty; only a 159-byte placeholder | `tauri.conf.json:16`; `src-tauri/icons/icon.png` |
| Prod build path exists (no dev server) | `tauri.conf.json` `beforeBuildCommand`/`frontendDist`; README:59 |
| DB path / idempotent init (rollback basis) | `lib.rs:249` `app_data_dir/vire.sqlite`; `init_db` `IF NOT EXISTS` |

The implementer must reuse the **existing** `Secret`/`Credentials` types and `run_bounded` rather than
introduce parallel un-redacted/unbounded paths — this is the single biggest correctness lever on
Workstream A.

## 3. DEC / SEC numbering — correction to the dispatch instruction

The dispatch said "use DEC-023 … and SEC-009." **DEC-023, DEC-024, and DEC-025 are already allocated**
(proposed) by TASK-023 / TASK-024 / TASK-025 respectively (grep of `openspec/changes/`). Using DEC-023
would collide. The next free decision number is **DEC-026**, which also aligns with the task number.
I have allocated **DEC-026** for the secret-storage decision and flagged this so Pi-Assistant/Janne can
confirm. **SEC-009 is genuinely free** (the committed security register
`artifacts/ba/11_security_review.md` ends at SEC-008; SEC-009 is referenced nowhere) and is used as
directed for the secure-secret-storage control.

## 4. Key architectural decisions / constraints for implementers

- **DEC-026 (secret storage):** secret key → macOS Keychain (`keyring` crate); non-secret settings →
  SQLite `settings`; settings-first resolution, env as marked dev fallback. Rejected alternatives:
  encrypt-in-SQLite (key next to ciphertext is not secure) and Stronghold (needs a vault password;
  heavier than a single-user Mac prototype needs). See `proposal.md` §ADR and `design.md` §3.
- **SEC-009 (secure secret storage control):** the secret key is never stored in SQLite / `settings` /
  logs / evidence / exports, and is **never rendered back** into the form (presence flag only). Test
  connection returns a coarse verdict with no secret, no raw body. SEC-002 (loopback) and SEC-003
  (credential non-leak) are preserved and extended to settings-sourced values.
- **Terminology:** "enable/disable tracing" → `langfuse_enabled`, the **import** integration switch.
  Vire is a trace **consumer**, not an emitter; UI copy must not imply OTEL emission (`design.md` §2).
- **No new webview surface:** Keychain + HTTP stay in the Rust core; CSP and `capabilities/default.json`
  are expected unchanged. If a capability/CSP change appears necessary, **stop and flag** — it would be
  an architecture-level change, not an implementation detail.
- **Icon:** temporary mark is a placeholder; brand owns the final asset. `artifacts/brand/` is
  read-only to SW — do **not** write there; document the replacement path in `code/`.
- **Packaging/rollback:** additive `settings` rows + idempotent `init_db` + app-scoped Keychain =
  forward/backward DB compatibility; document it for SW-6's `RELEASE.md`.

## 5. One change vs. split

**One change, three sequenced workstreams — no split required.** The three concerns share a single
release purpose ("make Vire a real desktop app") and the L2 thresholds (secure secret storage,
documentation, release compat/rollback) span all three. None crosses a data-model component boundary
(`§1`). The dispatch explicitly asked for a single OpenSpec change.

`tasks.md` segments the work so Pi-Assistant can route each workstream to the right role —
**A** (backend Rust/Tauri + frontend), **B** (devops/assets), **C** (devops/release) — and sequences
**A first** because it is the security-bearing core that the SW-4/SW-5 gates must cover. If scheduling
ever demands it, Workstream A could be carved into its own task without architectural harm (B and C are
near-zero-coupled build slices) — but that is an optional delivery choice, **not** an architectural
requirement, so the verdict is PASS rather than split-required.

## 6. ADR — DEC-026 (proposed)

See `proposal.md` §ADR / `design.md` §3. Summary: in-app Langfuse config; secret key in macOS Keychain
(`keyring`), non-secret settings in SQLite `settings`; settings-first with env as a marked dev
fallback; DEC-020 defaults and SEC-002 loopback allowlist unchanged. Status: proposed; canonical log
entry routed to BA.

## 7. feedback_to_ba[]

```
feedback_to_ba:
  - id: FB-026-1
    type: decision-log-entry
    target: artifacts/ba/07_decision_log.md
    summary: >
      Record DEC-026 — Vire's in-app Langfuse configuration is editable in Settings and persisted
      locally: the secret key (and public key) in the macOS Keychain via the keyring crate, and
      non-secret settings (base_url, source, environments, langfuse_enabled) in the SQLite settings
      table. The importer resolves config settings-first, with process env retained only as a marked
      dev fallback. DEC-020 loopback defaults and the SEC-002 loopback URL allowlist are unchanged;
      secrets are never stored in SQLite/logs/evidence/exports nor rendered back to the form.
    reason: >
      SW write-scope is limited to code/; ADRs must land in the canonical BA decision log. DEC-026 is
      proposed in this change but needs a BA-flow entry to be authoritative.
    blocking: false

  - id: FB-026-2
    type: security-register-entry
    target: artifacts/ba/11_security_review.md
    summary: >
      Record SEC-009 — secure secret storage for the in-app Langfuse credential: the Langfuse secret
      key is stored in macOS Keychain only, never in SQLite/the settings table/logs/evidence/exports,
      and is never returned by any read command or rendered back into the settings form (presence flag
      only). Test connection returns a coarse, secret-free verdict and is time-bounded. Extends SEC-002
      (loopback boundary) and SEC-003 (credential non-leak) to settings-sourced values.
    reason: >
      L2 secure-secret-storage threshold. The control is realized in code/ but the canonical security
      register entry is out of SW write-scope.
    blocking: false

  - id: FB-026-3
    type: numbering-correction
    target: pi-assistant / ba-architect
    summary: >
      The dispatch named DEC-023 for this task, but DEC-023/024/025 are already allocated (proposed)
      by TASK-023/024/025. Allocated DEC-026 instead (next free; aligns with the task number). SEC-009
      was free and used as directed. Please confirm DEC-026 when recording the canonical log entry.
    reason: >
      Avoid a decision-number collision in the canonical log.
    blocking: false
```

## Verdict

**PASS.** Proceed with the single change as scoped in `proposal.md` / `design.md` / `tasks.md`,
honoring DEC-026 (Keychain secret storage), SEC-009 (no plaintext secret, no read-back, bounded
secret-free Test connection), the settings-first / env-dev-fallback precedence, the unchanged loopback
allowlist, and the documented icon-replacement and release-compat/rollback posture. Route DEC-026 and
SEC-009 to BA for the canonical logs (non-blocking), and confirm the DEC-026 (not DEC-023) number.
Implement and review Workstream A (security-bearing) first, then B and C.
