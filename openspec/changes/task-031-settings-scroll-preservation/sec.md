# SEC — TASK-031 settings-scroll-preservation (SW-5)

- **Change dir**: `openspec/changes/task-031-settings-scroll-preservation/`
- **Branch / PR**: `feat/task-031-settings-scroll-preservation` — draft PR #26
- **Reviewed range**: `main...HEAD` (1 commit, `4a8ea20`)
- **Tier**: project = L2; this change = L1-equivalent (frontend-only; no backend/IPC/schema/egress/dependency changes)
- **Date**: 2026-06-20
- **Verdict**: **PASS** (with one required out-of-scope follow-up — see Escalations)

## Scope reviewed

Frontend-only change, 5 source/test files:

| File | Nature | Security read |
|------|--------|---------------|
| `src/scroll.ts` (new) | Pure fn `nextScrollTop(sameView, prevScroll)` → number | No DOM, no I/O, no input — returns `prevScroll` or `0`. Inert. |
| `src/main.ts` | `shell()` captures outgoing `<main>.scrollTop` and restores it post-render | `scrollTop` is **numeric** read/write; **no new string flows into innerHTML**. The innerHTML template and its `esc()` escaping are unchanged from baseline. |
| `src/env-mapping-ui.ts` | Help copy: `Create project for …` → `Create &amp; map` | Static string literal; ampersand correctly HTML-entity-escaped (`&amp;`). No user input. |
| `tests/scroll.test.mjs` (new) | Unit test for the pure helper | n/a |
| `tests/envMappingUi.test.mjs` | Asserts new help copy + `doesNotMatch` old copy | n/a |

## Requested threat checks

- **XSS / DOM injection** — None introduced. `shell()` adds only `scrollTop` (number) read/write; the templated HTML and existing `esc()` calls are byte-for-byte unchanged. The env-mapping copy change is a static literal with `&` properly escaped to `&amp;`. ✅
- **Secret exposure** — gitleaks on branch commits (`main..HEAD`): **no leaks**. No secrets, tokens, or credentials touched. Existing test `mapping surfaces never leak a secret-shaped token` still guards `env-mapping-ui`. ✅
- **IPC misuse** — No `invoke`/`call` sites added or changed; the `call<T>()` wrapper is untouched. ✅
- **New capabilities / egress / dependencies** — None. Only a **local** import added (`./scroll`). `package.json`, `package-lock.json`, and Tauri capability/conf are untouched by this branch. ✅
- **Auth / permission regression** — No auth, permission, or trust-boundary code in scope. ✅

## Scanner results (Tier 1 stack)

| Scanner | Scope | Result | Auto-fail? |
|---------|-------|--------|------------|
| **semgrep** (`--config auto --severity ERROR`) | 5 changed files | **0 findings** (109 rules) | No |
| **gitleaks** | branch commits `main..HEAD` | **no leaks found** | No |
| **gitleaks** | full working tree (`--no-git`) | 3 hits — see note ① | No (out of scope) |
| **OSV-scanner** | `package-lock.json` (106 pkgs) | 3 vulns, all **dev deps** — see note ② | See triage |
| **Trivy** (`vuln,secret,misconfig`, HIGH/CRITICAL) | tracked source; `package-lock.json` + `src-tauri/Cargo.lock` | **0 / 0 / 0** (dev deps suppressed → confirms no runtime exposure) | No |

**① gitleaks working-tree (3× `generic-api-key`)** — all in `src-tauri/target/{debug,release}/deps/libmuda-*.rmeta`: Rust **build artifacts**, **git-ignored** (`git check-ignore` confirms) and untracked, **not in this branch's diff**, false-positive pattern matches in compiled metadata. Cannot reach version control. Non-blocking; not introduced by TASK-031.

**② OSV-scanner findings** — all `devDependencies`, **pre-existing**, lockfile **not modified by this change**:

| Advisory | Pkg | Ver | CVSS | Fixed in | Scope |
|----------|-----|-----|------|----------|-------|
| GHSA-fx2h-pf6j-xcff | vite (dev) | 6.4.2 | **8.2 High** | 6.4.3 | vite dev server only |
| GHSA-v6wh-96g9-6wx3 | vite (dev) | 6.4.2 | 5.5 Med | 6.4.3 | vite dev server only |
| GHSA-g7r4-m6w7-qqqr | esbuild (dev) | 0.28.0 | 2.5 Low | 0.28.1 | esbuild dev server |

## Triage — why PASS despite a CVSS 8.2 OSV hit

The OSV ≥7.0 auto-fail rubric targets vulnerabilities a change *introduces* or *ships*. This finding meets none of those:

1. **Pre-existing** — `package.json`/`package-lock.json` are untouched by TASK-031 (frontend scroll fix). The vuln predates this branch.
2. **Dev-only, not shipped** — `vite`/`esbuild` are `devDependencies`; runtime deps are only `@tauri-apps/api` + `@tauri-apps/plugin-dialog`. The advisories are vite/esbuild **dev-server** issues; a Tauri desktop app ships a static `vite build` bundle, so the dev server is not in the end-user artifact. Real-world exposure ≈ nil (limited to a dev running the dev server on an exposed network).
3. **Corroborated clean at runtime** — Trivy (dev deps suppressed) reports **0 HIGH/CRITICAL** for `package-lock.json` and `src-tauri/Cargo.lock`.
4. **Fix is out of scope** — bumping vite to 6.4.3 inside a scroll-preservation PR would be scope creep (forbidden by hard rules). It belongs in a dedicated dependency-remediation task.

The change under review is clean across all five requested threat checks and all four scanners. Therefore: **PASS this gate**, and surface the dev-dep vite/esbuild bump as a **required separate follow-up** (escalated below, not dropped).

## Escalations

- **[REQUIRED follow-up, non-blocking for TASK-031]** Project dependency hygiene: bump `vite` 6.4.2 → ≥6.4.3 (clears GHSA-fx2h-pf6j-xcff 8.2 + GHSA-v6wh-96g9-6wx3 5.5) and `esbuild` 0.28.0 → ≥0.28.1 (GHSA-g7r4-m6w7-qqqr 2.5). Open as its own task — do **not** fold into this PR. Not a design-level (BA Architect) issue.

## Verdict

**SEC STATUS: pass** — TASK-031 introduces no security defects. Pre-existing dev-dependency advisories and git-ignored build-artifact false positives are documented and escalated, and do not block this change's gate.
