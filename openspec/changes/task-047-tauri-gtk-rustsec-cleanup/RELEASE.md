# RELEASE gate artifact — TASK-047 / v0.7.1

- **Role**: Release Manager (SW-6) · **Date**: 2026-06-21
- **Branch**: `feat/task-047-tauri-gtk-rustsec-cleanup` · **PR**: #35
- **Version**: v0.7.1 (patch)
- **Tag**: `task-047/v0.7.1`
- **Tier**: L2

---

## Gate status

| Gate | Status |
|---|---|
| SW-3 QA recheck | **PASS** (`qa.md`) |
| SW-4 Code Review recheck | **PASS** (`review.md`) |
| SW-5 Security recheck | **PASS** (`sec.md`) |
| SW-6 DOCS | **PASS** (`handoff.md` §SW-6 DOCS) |
| CI advisory gate | **pending first PR run** (no status checks recorded; gate fires on PR/push/dispatch) |

All prior SW-3/SW-4/SW-5/DOCS blockers closed. CI gate not yet recorded in `statusCheckRollup`
(PR triggered after branch push; advisory gate will run on the PR event).

---

## 1. Deployment size

**patch** — config/docs only. Zero shipped-crate delta: `git diff main -- src-tauri/Cargo.toml
src-tauri/Cargo.lock` is empty; no source (`src-tauri/src/`, `src/`), IPC, Tauri capability,
`tauri.conf.json`, or UI change. The macOS `.app` binary built from this branch is bit-equivalent
to v0.7.0.

---

## 2. Rollback strategy

**automated** — delete `src-tauri/deny.toml` and
`.github/workflows/dependency-advisories.yml`. No data migration, no schema change, no runtime
artifact delta. Reverting removes the CI advisory gate; the shipped app binary is unaffected.

---

## 3. Component compatibility matrix

| Dependency / component | Min version | Pinned / locked | Notes |
|---|---|---|---|
| `cargo-deny` (CI gate) | 0.18.0 | **0.19.9** | `unmaintained = "all"` requires cargo-deny ≥ 0.18 (advisories v2 schema); pinned via `tool: cargo-deny@0.19.9` in CI workflow |
| Rust toolchain | stable | stable (cargo 1.95.0 tested) | No constraint change from v0.7.0 |
| Tauri runtime | 2.11.2 | 2.11.2 | Zero `Cargo.lock` delta — all 17 advisory crate versions unchanged |
| npm / frontend deps | unchanged | unchanged | No `package.json` / `package-lock.json` delta |
| macOS target | Ventura 13+ | — | Same as v0.7.0 |
| Upstream advisory DB | RustSec db as of 2026-06-21 | — | 5 accepted `unic-*` IDs (RUSTSEC-2025-0075/0080/0081/0098/0100); trigger review on next RustSec DB update touching these IDs |

**No upstream/downstream service version constraints changed.** The compatibility surface is
identical to v0.7.0 — this release adds only a CI/developer tooling gate with no effect on the
deployed macOS application.

---

## Changed paths

| Path | Change |
|---|---|
| `src-tauri/deny.toml` | Added (cargo-deny config; Apple-target-scoped advisory gate; ACCEPTED/DEFERRED blocks) |
| `.github/workflows/dependency-advisories.yml` | Added (CI gate; pinned cargo-deny 0.19.9; ubuntu runner; PR/push/dispatch) |
| `RELEASE.md` | Updated (v0.7.1 entry added; compatibility matrix) |
| `README.md` | Updated (version bump v0.6.3→v0.7.1; `## Dependency advisory gate` section added) |
| `openspec/changes/task-047-tauri-gtk-rustsec-cleanup/*.md` | Added (gate artifacts: arch-review, design, handoff, ops-review, proposal, qa, review, sec, tasks) |

---

## Advisory posture (17 RustSec advisories)

### Group A — 12 Linux-only (DEFERRED, target-scoped out)

Not in the shipped macOS graph (proven by `cargo tree`). Not in `ignore`. Re-surface when a Linux
triple is added to `[graph].targets` (tripwire). See `ops-review.md` §4 and `sec.md` for full inventory.

### Group B — 5 Apple-present (ACCEPTED)

`unic-*` cluster (RUSTSEC-2025-0075/0080/0081/0098/0100): unmaintained, unscored, no upstream fix.
Accepted in `[advisories].ignore` with per-ID rationale. See `deny.toml` ACCEPTED block + `sec.md` §SEC-001 closure.

**Review trigger:** if a new RustSec advisory for any accepted ID is published, the per-ID entry in
`deny.toml` must be re-evaluated. The review trigger is documented in `deny.toml` header.

---

## Local gate command

```sh
cargo install cargo-deny --version 0.19.9 --locked
cd src-tauri && cargo deny check advisories
```

---

## Notes

- **Version correction**: branch was cut from main at v0.6.2 (task-044). task-045 (v0.6.3) and
  task-046 (v0.7.0) merged to main subsequently. This release is correctly versioned v0.7.1.
  Root `RELEASE.md` heading and `README.md` version updated as part of SW-6 gate.
- **CI advisory gate**: no `statusCheckRollup` recorded yet (PR was draft). Gate runs on the
  PR push event. Expected to pass (real `cargo-deny 0.19.9` evidence in `ops-review.md` §3).
- **Signed tag**: SSH signing key present (`/Users/kaikkonen/.ssh/id_ed25519.pub`, `gpg.format=ssh`,
  `tag.gpgsign=true`). Tag creation attempted after commit.
