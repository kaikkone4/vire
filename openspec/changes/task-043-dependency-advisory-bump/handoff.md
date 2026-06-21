<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-043 Stream A

- **Change / branch / PR**: `task-043-dependency-advisory-bump` / `chore/task-043-vite-esbuild-advisory-bump` / PR #30.
- **Scope**: npm devDependency advisory bump only. Stream B is TASK-044 and remains untouched.
- **Current gate**: **SW-6 Release PASS (2026-06-21)** — RELEASE.md written; PR #30 promoted draft→ready.
- **Prior gates**: SW-3 QA PASS, SW-4 Code Review PASS, SW-5 Security PASS.
- **Blockers**: SSH private key absent — tag `task-043/v0.6.1` is a dry-run; Janne must sign+push after restoring key. PR #20 must be closed manually.

## Verified state

- Vite floor/resolution: `^6.4.3` / `6.4.3`; no Vite 8.
- `tsx`-transitive esbuild: `0.28.1`, including platform packages.
- Lockfile name restored to `"code"` by `953191c`.
- `npm audit` = 0 vulnerabilities; `npm run build` OK; 103/105 tests (two pre-existing env failures).
- SW-4: no blocking findings; diff scope confirmed (package files + OpenSpec only).
- SW-5: full L2 scanner stack — OSV/npm-audit/Trivy 0, gitleaks 0, semgrep 0 ERROR. Baseline CVSS 8.2 HIGH cleared.
- SW-6: RELEASE.md — all three declarations present (patch / automated / compat matrix). PR #30 ready.

## Pending actions (Janne)

1. **Sign + push tag** (after restoring `~/.ssh/id_ed25519`):
   `git tag -s task-043/v0.6.1 -m "release(task-043): v0.6.1 patch — npm dev-dep advisory bump" 7d845b9 && git push origin task-043/v0.6.1`
2. **Merge PR #30** → main.
3. **Close PR #20** as superseded (do not merge — vite 8 major is out of scope).

## References

- `RELEASE.md` — SW-6 release document (all three declarations).
- `sec.md` — SW-5 security verdict and scanner evidence.
- `review.md` — SW-4 verdict and checks.
- `qa.md` — SW-3 evidence.
- `ops-review.md` — advisory and PR #20 rationale.
