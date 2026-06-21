# SW-4 code review — TASK-043 Stream A / PR #30

## Verdict: PASS

No blocking craft, convention, complexity, or dead-code issues.

## Blocking issues

None.

## Suggestions

None.

## Review results

- `package.json:1` changes only the Vite devDependency floor from `^6.0.7` to `^6.4.3`; runtime dependencies and all other scripts/dependencies are unchanged.
- `package-lock.json:2` restores the repository lockfile name to `"code"`.
- `package-lock.json:15`, `package-lock.json:1839`, and `package-lock.json:1840` consistently resolve Vite to dev-only `6.4.3`; no Vite 8 major is present.
- `package-lock.json:1783` and `package-lock.json:1784` consistently resolve the `tsx`-transitive, dev-only esbuild to `0.28.1`; all 26 matching platform packages also resolve to `0.28.1`.
- Semantic lockfile comparison changes only the root Vite range, Vite, `tsx`'s esbuild, and its platform packages. `npm ci --ignore-scripts` and `npm ls vite esbuild --all` succeeded.
- Diff scope is limited to `package.json`, `package-lock.json`, and TASK-043 OpenSpec artifacts. There are no source, runtime dependency, capability, Tauri config, Cargo manifest, or Cargo lock changes.
- `npm run build` passed with Vite 6.4.3. The local frontend run reached 101/105; four listener tests were blocked by sandbox `listen EPERM`. SW-3's recorded gate remains PASS (103/105 with two pre-existing environment-dependent failures).
- Live `npm audit` and GitHub PR metadata checks could not run because external DNS/API access is unavailable. SW-3 recorded `npm audit` at 0 vulnerabilities.
- PR #20 is correctly superseded: its Vite 8 major upgrade is outside this minimal Vite 6 patch scope. The branch commit messages and OpenSpec rationale are clear and complete.

## Escalations to SW Architect

None.

## Changed paths reviewed

- `package.json`
- `package-lock.json`
- `openspec/changes/task-043-dependency-advisory-bump/*`
