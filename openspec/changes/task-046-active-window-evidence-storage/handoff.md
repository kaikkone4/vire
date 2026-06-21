<!-- handoff.md — compact per-task state. KEEP <= 2 KB. -->

# Handoff — TASK-046 active-window evidence storage

- **Branch / PR**: `feat/task-046-active-window-evidence-storage` / #34
- **Head**: `19b95ff`; implementation fix: `59747b4`
- **Phase**: SW-3 + SW-4 + SW-5 all PASS at `59747b4`; **SW-6 COMPLETE** (docs gate passed)
- **Next**: release manager — branch ready to merge

## SW-5 verdict

PASS (`sec.md`). Tier-L2 scanners clean of auto-fail: semgrep 0 ERROR · gitleaks 1 triaged FP (ADV-1
synthetic fixture) · OSV max CVSS 6.9 < 7.0 · Trivy 0 HIGH/CRITICAL. Zero deps added; lockfiles
byte-identical. SEC-001 invariants enforced + test-covered. Prior **ADV-3** (`capture_health` vocab on
raw/evidence) **closed** by §8.1; **ADV-2** (`detail` ≤200 B) stays closed. 1 advisory left: **ADV-1** —
scope a `.gitleaks.toml` allowlist for the historical synthetic-fixture commit `3aa716e` (non-blocking).
No design/code escalation.

## SW-4 verdict

PASS (`review.md`). No blockers/escalations. Confirmed: `capture_health` vocab on raw/evidence;
fail-closed title gate (`Some("")`→`(NULL,"empty")`; non-null title iff stored+`captured`); serialized
env-only + DB-over-env config tests; `Some("")` bundle→null sentinel; prior invariants intact;
storage-only scope (no IPC/renderer/capture/network/dep/CSP). Non-blocking: add `Some("")` bundle
regression; resolve 45 staged dead-code warns when seam is wired.

## Checks (at `59747b4`)

- `cargo test` ✓ 220 lib + 5 adversarial · `clippy --lib` ✓ 48 warn (45 staged, 3 pre-existing)
- `cargo fmt --check` ✓ · `openspec validate task-046 --strict` ✓
- SW-5 Tier-1 scanners re-run ✓ (semgrep/gitleaks/OSV/Trivy — see `sec.md`)

## SW-6 verdict

COMPLETE (release manager pass). All three L2 declarations present in `RELEASE.md`:
1. **Deployment size**: minor
2. **Rollback strategy**: automated
3. **Component compatibility matrix**: added at `0de0e62` (rusqlite/tauri/chrono/serde/keyring/@tauri-apps/api/macOS/SQLite DB — min/max per dep)

PR #34 promoted from draft → **ready-for-review** (confirmed via `gh pr ready 34`).

Signed tag `task-046/v0.7.0`: **blocked-non-critical** — SSH signing key
`/Users/kaikkonen/.ssh/id_ed25519.pub` not found; no unsigned fallback per L2+ policy.

Changed files (docs pass + release manager pass):
- `README.md` — version v0.6.3→v0.7.0; env vars table; privacy section; release compat section.
- `RELEASE.md` — v0.7.0 entry + component compatibility matrix.
- `handoff.md` — this update.

## Gate files

- `review.md`: SW-4 PASS
- `qa.md`: SW-3 PASS
- `sec.md`: SW-5 result/current security state
- `handoff.md`: SW-6 COMPLETE
