# Security Review — TASK-032 AI time-entry suggestions (SW-5)

- **Change**: `openspec/changes/task-032-ai-time-suggestions/` · Branch `feat/task-032-ai-time-suggestions` · PR #27
- **Scope reviewed**: full A+B+C — suggestion engine/table (A), accept/dismiss IPC + AI-origin entry + origin-separated reporting/CSV (B), Suggestions review UI (C), and the SW-4 fix-loop atomic regeneration.
- **Tier**: L2 (secrets + CVE≥7 + Trivy + semgrep ERROR).
- **HEAD**: `fd5cf12` · **Reviewed**: 2026-06-20.

## Verdict: **PASS**

The change is clean across every L2 auto-fail dimension and every requested manual check. One pre-existing
high-CVSS **dev-only** dependency advisory (vite) is surfaced below as advisory — it is not introduced by
TASK-032 (zero dependency delta vs `main`), not shipped in the desktop binary, and is therefore not a
TASK-032 auto-fail. A separate dependency-bump task is recommended.

## Scanner results (Tier 1 / L2)

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** | full history (165 commits) + working tree | **no leaks** | No |
| **semgrep** | `p/rust` + `p/typescript` + `p/security-audit`, ERROR only — 115 rules / 38 files | **0 findings, 0 errors** | No |
| **Trivy** `fs` | `vuln,secret,misconfig`, HIGH/CRITICAL — `package-lock.json`, `Cargo.lock` | **0 findings** (no secrets, no misconfig) | No |
| **OSV-scanner** | `Cargo.lock` (492 pkgs) + `package-lock.json` (106 pkgs) | 1 npm finding ≥7.0 — see below | Triaged → advisory |

### OSV findings — triage

- **vite 6.4.2 → 6.4.3 — GHSA-fx2h-pf6j-xcff — CVSS 8.2 (npm, dev).** `vite` is a **devDependency**
  (`package.json` `devDependencies`); production `dependencies` are only `@tauri-apps/api` and
  `@tauri-apps/plugin-dialog`. The shipped Tauri binary serves the static built bundle
  (`tauri.conf.json` `frontendDist: "../dist"`); the vite dev server (`devUrl 127.0.0.1:1420`) is
  loopback-only and is **not** part of the distributed artifact. **Not introduced by TASK-032** — the
  `package.json`/`package-lock.json` diff vs `main` is empty (zero dependency delta). Trivy reports the
  same lockfile as 0 HIGH/CRITICAL. → **Advisory, not a TASK-032 auto-fail.**
- **esbuild 0.28.0 → 0.28.1 — GHSA-g7r4-m6w7-qqqr — CVSS 2.5 (npm, dev).** Transitive via vite; dev-only,
  pre-existing. Advisory.
- **vite GHSA-v6wh-96g9-6wx3 — CVSS 5.5 (npm, dev).** Same package/fix as above (6.4.3). Advisory.
- **glib 0.18.5 → 0.20.0 — RUSTSEC-2024-0429 — CVSS 6.9 (crates.io).** `< 7.0`, transitive via the
  Tauri/GTK stack; not reachable from TASK-032 code. Advisory.
- **15 unscored RUSTSEC advisories** (atk, gdk*, gtk*, proc-macro-error, unic-*) — all transitive
  Tauri/GTK/proc-macro deps, "unmaintained"-class with no CVSS, pre-existing. Advisory.

None of the advisories are introduced by, or in scope of, TASK-032. **Recommended follow-up (separate
task):** `npm update vite esbuild` (→ 6.4.3 / 0.28.1, dev-only lockfile bump) and a Tauri-version bump to
clear the GTK-stack RUSTSEC set.

## Manual review (requested checks)

1. **No secrets / traces / prompts exposed — PASS.** The `Suggestion` surface
   (`suggestions/mod.rs`) carries only env name, project ref, local times, aggregate token/cost sums,
   counts, and health/confidence/reason labels. `session_id` is read into `EvidenceRow` but consumed
   **only** for a distinct-session count (`engine.rs:350`) — never persisted or serialized; `trace_id`
   is not projected (kept solely in `ORDER BY`); no payload/prompt/metadata/credential is touched. The
   accept-path provenance note is `"AI-suggested ({source}): {reason}"`, both secret-free and capped at
   `MAX_ENTRY_NOTE_LEN`. Negative test `surfaces_carry_no_secrets` (`tests.rs:622`) seeds a fake
   `session-Bearer-sk-leak` / `t1` and asserts none of `session-`/`Bearer`/`sk-`/`leak`/`t1` appear in
   the serialized output. gitleaks + semgrep concur.
2. **No XSS / DOM injection — PASS.** `suggestions-ui.ts` escapes every rendered value via
   `escapeHtml` (escapes `& < > ' "`); the only un-wrapped interpolations are the `Health · Confidence`
   cell (each half individually `esc()`-ed) and action markup keyed by the backend UUID `id` (escaped).
   `main.ts` `esc()`-es all dynamic values and uses `CSS.escape()` for attribute-selector building. No
   `innerHTML`-with-raw, `insertAdjacentHTML`, `eval`, or `new Function` in the new frontend.
3. **No auth / permission regression — PASS.** Vire is a local-only desktop app with no auth layer
   (no accounts/cloud — Manual Mode). No auth/permission code is touched. The three new IPC commands
   (`list_/accept_/dismiss_time_entry_suggestion`) read/write local SQLite only and are registered in the
   existing `invoke_handler`. No Tauri capability/allowlist or `tauri.conf.json` change.
4. **No new egress / capabilities / dependencies — PASS.** The engine is pure over SQLite — no
   network code in `suggestions/` (DEC-001/017 honored). `Cargo.lock`, `Cargo.toml`,
   `package.json`, `package-lock.json` are unchanged vs `main` (zero dependency delta). No capabilities
   or config surface added.
5. **Transaction safety / security — PASS.** All SQL is parameterized (`params![]` / `?n`); no user
   input is concatenated into SQL (the reporting split uses a fixed `'ai_suggested'` literal in a
   `CASE`). `accept_suggestion_repo` runs in one `conn.transaction()`: status-gate → active-project
   check → `parse_duration` → insert entry → guarded `UPDATE … WHERE id=? AND status='pending'`
   (TOCTOU-safe), rolling back on any error. `generate` wraps delete + guarded inserts + read in one
   `unchecked_transaction` (atomic replace-set — SW-4 fix B1), so a failed Refresh preserves the prior
   pending set. `dismiss` is a guarded `WHERE status='pending'` update; an accepted suggestion cannot be
   dismissed.
6. **Local-only boundary — PASS.** Engine reads SQLite only; no Langfuse egress; Vire stays the
   authoritative store with read-only association of evidence at read time (DEC-001).
7. **CSV injection (defense-in-depth) — PASS.** `export_csv_repo`'s new `origin` column is run through
   the existing `csv_escape` → `csv_formula_neutralized` (neutralizes leading `= + - @` and tab/CR/LF,
   plus quote-escaping). `origin` is a fixed `manual`/`ai_suggested` value, so no injection vector.

## Blockers

- **None** (no auto-fail condition hit).

## Advisories (non-blocking)

- **A1 (recommend separate task, before next release):** bump dev deps `vite`→6.4.3 / `esbuild`→0.28.1
  to clear GHSA-fx2h-pf6j-xcff (CVSS 8.2, dev-only). Pre-existing, not shipped, out of TASK-032 scope.
- **A2:** Tauri/GTK-stack RUSTSEC advisories (glib 6.9 + 15 unscored) — transitive, pre-existing;
  resolve via a Tauri version bump in a dependency-maintenance task.
- **A3 (carried from SW-4, non-blocking):** add an explicit cross-midnight evidence/acceptance policy
  test.

## Routing

PASS → SW-4 already PASS (recheck `fd5cf12`) → proceed to **SW-6 Release Manager**.
