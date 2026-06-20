# SW-5 Security Review — TASK-030 (create-project-button-fix)

- **Change:** `openspec/changes/task-030-create-project-button-fix/`
- **Branch / PR:** `feat/task-030-create-project-button-fix` / PR #25
- **Gate:** SW-5 Security review (parallel with SW-4 Code Review — SW-4 already PASS)
- **Tier:** L2 project; this task introduces no new backend / egress / dependencies, so the live attack surface is L1-equivalent (frontend-only UI delta).
- **Verdict:** **PASS**

## Scope reviewed (TASK-030 delta only)

Reviewed the delta `9de66e5..2c86111` (single impl commit `2c86111`), **excluding** TASK-029 branch noise (langfuse importer / runtime_observer / settings — ~37k lines from the unmerged recovery PR #24 history). Source/test files in delta:

- `src/env-mapping-ui.ts` — `mappingRow()`: replaced the `Create project for <env>` button with an inline `<input data-create-name>` (pre-filled, `maxlength="120"`) + `Create & map` button.
- `src/main.ts` — `bindEnvMapping()`: the `[data-create-map]` click handler now reads the inline input value instead of calling native `prompt()` (which returns `null` silently in the macOS WKWebView).
- `tests/envMappingUi.test.mjs` — markup assertions for the new input.

Nature of change: replace a native `window.prompt()` dialog with an in-app text input. No backend (`src-tauri/`) change, no IPC command added, no config change.

## Scanner results (Tier 1)

| Scanner | Scope | Result | Auto-fail hit? |
|---------|-------|--------|----------------|
| **gitleaks** | commit range `9de66e5..2c86111` | 1 commit / ~24.6 KB scanned, **no leaks** | No |
| **semgrep** (`--config=auto`, ERROR) | `env-mapping-ui.ts`, `main.ts`, `html.ts` | 109 rules, 3 targets, **0 findings, 0 errors** | No |
| **OSV-scanner** | `package-lock.json`, `src-tauri/Cargo.lock` | 20 known vulns (see triage below) | No — none introduced by delta |
| **Trivy** | container image | **N/A** — Tauri desktop app, no container/image artifact | N/A |

## Manual review — targeted checks

- **XSS / DOM injection — CLOSED.** The environment name is the only attacker-influenceable value (it originates from discovered Langfuse traces — a trust boundary). In `mappingRow()` it is escaped once (`const e = esc(env.environment)`) and then interpolated into the double-quoted attributes `data-create-name="${e}"`, `value="${e}"`, and `aria-label="…${e}"`. `escapeHtml` (`src/html.ts`) escapes `& < > ' "` — including `"`→`&quot;` and `'`→`&#39;` — so the name cannot break out of an attribute to inject new attributes or event handlers. Builder remains pure HTML (no `innerHTML` of unescaped data).
- **Selector injection — CLOSED.** `main.ts` resolves the input via `` `[data-create-name="${CSS.escape(env)}"]` `` — `CSS.escape` neutralizes any CSS-meta characters in the env name. (Same pattern already used for `data-map-select`.)
- **IPC misuse — none.** Handler calls only pre-existing commands `create_project`, `set_env_mapping` (and `clear_env_mapping`, unchanged). No new `invoke`/`call` target. The typed project name flows to `create_project` as a JSON IPC arg (`{input:{name:trimmed,notes:null}}`) — serialized, not interpolated into a shell/SQL/HTML sink; backend validation is unchanged. Empty input is rejected client-side (`alert` + refocus). Stored names are re-escaped at render (`projectOptionsHtml`, `mappingRow`), so no stored-XSS path is opened.
- **Secret exposure — none.** No credential, trace payload, or secret touched. Consistent with the file's SEC-010 invariant (surfaces carry only environment names and project references). gitleaks confirms no secret in the delta.
- **New capabilities / egress / dependencies — none.** No `tauri.conf.json`, no `capabilities/*`, no `package.json` / `Cargo.toml` / lockfile in the delta. No `fetch`/network call added — all actions are local Tauri IPC. Net egress surface unchanged.
- **Auth / permission regression — none.** Single-user local desktop app; no auth/permission/trust-boundary touched. The create-and-map action is still explicit (no auto-create/auto-map — DEC-006 preserved). Replacing `prompt()` with an in-app input narrows, not widens, the surface (removes reliance on a native dialog).

## OSV triage — inherited, NOT introduced by TASK-030

The TASK-030 delta changes **zero dependencies**, so every OSV finding is pre-existing repo state (carried in via TASK-029 / base), not a regression of this change. Per the gate scope ("TASK-030 delta only"), these are **advisory** and do not auto-fail this gate. Notable:

- **`vite` 6.4.2 → fix 6.4.3** — GHSA-fx2h-pf6j-xcff, **CVSS 8.2 (HIGH)**. This is the only finding meeting the CVE≥7 auto-fail threshold. It is a **dev-only** dependency (Vite build/dev-server tooling) — not bundled into the shipped Tauri binary, so no production runtime exposure. Also GHSA-v6wh-96g9-6wx3 (5.5).
- **`esbuild` 0.28.0 → fix 0.28.1** — GHSA-g7r4-m6w7-qqqr (2.5), dev-only.
- **`glib` 0.18.5** — RUSTSEC-2024-0429 / GHSA-wrw7-89jp-8q8g, CVSS 6.9 (below 7.0 threshold).
- GTK3 / atk / gdk RUSTSEC-2024-04xx stack and unic-* RUSTSEC-2025-00xx — unscored, transitive via the GTK/Tauri toolchain.

**Recommendation (non-blocking for TASK-030):** address the repo-level dependency posture — bump `vite` ≥ 6.4.3 and `esbuild` ≥ 0.28.1 — in a dedicated dependency-bump task or at the next release pass (SW-6 / Release Manager awareness). This is repo debt independent of the create-project-button fix and should be tracked at branch/main level once recovery PR #24 is synced, not gated on this frontend UI change.

## Blockers

None (no auto-fail condition hit by the TASK-030 delta).

## Verdict

**SEC STATUS: pass** — TASK-030 delta is clean: XSS/attribute-injection closed via quote-safe escaping, no new IPC/egress/capability/dependency, no secrets, no semgrep ERROR. The single CVE≥7 OSV finding (`vite` 8.2) is an inherited dev-only dependency advisory, not introduced by this change; documented for repo-level remediation. No design-level escalation. Proceed — wait for SW-4 (already PASS) then route to SW-6 Release Manager.
