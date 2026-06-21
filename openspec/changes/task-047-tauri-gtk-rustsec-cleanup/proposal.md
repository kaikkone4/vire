# TASK-047 — Tauri/GTK RustSec dependency-chain cleanup (TASK-043 Stream B)

## Why

TASK-043 split the carried dependency advisories into two chains. Stream A (npm `vite`/`esbuild`) shipped
in PR #30. Stream B — the Tauri → wry/tao/muda **Linux GTK3** Rust chain — was deferred and is this task.
The advisories keep re-surfacing on every SW-5 gate as "recommend a separate task" but nobody owns them.

This task **decides the disposition of Stream B and lands the enforcement** so the signal stops riding
along. It is a **dependency/security-posture change only** — no feature, UI, schema, IPC, or capability
change.

## Decision (verified — see `design.md` for sources)

**(c) Deferred with documented risk acceptance, enforced by a target-scoped `cargo-deny` advisory gate.**

The two alternatives are not actionable today and were ruled out against official sources:

- **(a) compatible dependency bump — rejected.** No semver-compatible bump clears the chain. The gtk3-rs
  bindings (`glib`/`gtk`/`gdk`/`atk`/`gtk3-macros` 0.18.x) are **end-of-life upstream** — RustSec marks
  them "no longer maintained" with **no patched version**. `glib`'s soundness fix (RUSTSEC-2024-0429) lands
  only in `glib ≥ 0.20`, which belongs to the **gtk4** ecosystem the gtk3 backend cannot use. Tauri 2.x
  still targets **GTK3 + webkit2gtk-4.1** on Linux.
- **(b) Tauri major-migration spike — rejected (premature).** There is no Tauri release that drops GTK3
  and no published GTK4 Linux-backend roadmap to migrate *to*. A spike would research an upstream
  capability that does not yet exist, for crates that are **not compiled into the shipped macOS artifact**.

This **corrects the TASK-043 arch-review assumption** that "bumping Tauri pulls gtk-rs ≥ 0.20" — verified
false. See `design.md` §"Correction to TASK-043".

## What changes

- Add a **`cargo-deny` configuration** (`src-tauri/deny.toml`) whose advisory graph is **scoped to the
  shipped Apple targets** (`aarch64-apple-darwin`, `x86_64-apple-darwin`). On those targets the gtk3-rs
  chain is **not in the dependency graph** (proven by `cargo tree`), so the advisories do not fire — the
  gate reflects exactly what ships.
- Add a **documented risk-acceptance record** (in `design.md`) enumerating the deferred Linux-only
  advisory IDs, the reachability rationale, and a **tripwire**: adding a Linux build target re-surfaces
  `glib` RUSTSEC-2024-0429 and the unmaintained cluster, forcing re-evaluation rather than burying them.
- Add a **`cargo deny check advisories` gate** (CI step + documented local command) so the posture is
  enforced and auditable going forward.

## Scope

In scope: `src-tauri/deny.toml`, the advisory gate wiring/docs, and this OpenSpec change. **No** edit to
`Cargo.toml`, `Cargo.lock`, source, schema, IPC, capabilities, or `tauri.conf.json`. Zero shipped-crate
delta.

Out of scope: any Tauri/wry/tao version bump; any gtk-rs upgrade; adding a Linux build target; npm/Stream A
(done in TASK-043).

## Owner

SW Architect (SW-1) owns this proposal/design/spec + the decision. Implementation owner: **SW-backend +
devops** (add `deny.toml`, wire the CI gate, run the scan to confirm clean). If a future task adds a Linux
target or bumps Tauri, **SW-3 QA** re-enters per the tripwire.

## Impact

- Affected code: **none shipped** — config + docs only. `git diff main -- src-tauri/Cargo.lock` MUST be empty.
- Tier: L2 security-posture change (no runtime reachability on the shipped target).
- Risk: **low.** The change cannot regress the macOS `.app`; it only adds an honest, scoped gate.
- Recommendation envelope: **pass** — single coherent task, no split, no BA escalation (macOS-only is an
  established project decision: DEC-026 / `keyring` apple-native).
