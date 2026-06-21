# TASK-043 — Dependency advisory follow-up: triage & split (vite/esbuild + Tauri/GTK RUSTSEC)

## Why

Every recent SW-5 security gate has carried the **same set of non-blocking dependency advisories** as
"recommend a separate dependency-bump task" — see `task-032/sec.md` §OSV/Advisories and
`task-026/sec.md` §4. They were correctly triaged as non-auto-failing (dev-only and/or not reachable in
the shipped `.app`, zero dependency delta per feature change), but they keep re-surfacing because nobody
owns clearing them. This task does the **triage and design only** to decide *what* to bump, *who* owns
it, *what tests* prove it, and — the central question — *whether the npm dev-deps and the Tauri/GTK Rust
chain are one task or two*. **No implementation here.**

## What changes (this task = SW-1 triage)

This change produces a design/triage record, not code. It:

- Catalogs the carried advisories into **two independent dependency chains** with different risk, owner,
  test strategy, and cadence (see `arch-review.md`).
- **Recommends a split** into two execution tasks so the trivial, shippable npm fix is not held hostage
  to the riskier native-framework work.
- Defines the minimal, in-scope bump and the verification gate for each stream in `tasks.md`.

## The two chains (summary — full triage in `arch-review.md`)

- **Stream A — npm dev-deps (`vite`, `esbuild`).** Dev/build tooling only; **not** in the distributed
  `.app` (Rust binary + static `dist/`); dev server is loopback-only. Fix is a **lockfile refresh**
  inside the already-declared `vite ^6.0.7` range (→ 6.4.3, which pulls fixed `esbuild`). **Low risk.**
- **Stream B — Tauri/GTK Rust chain (`glib` RUSTSEC-2024-0429 + ~15 unscored gtk-rs/unic advisories).**
  Transitive via the Tauri → wry → tao **Linux** GUI backend (gtk-rs 0.18). On the macOS-only target
  these crates are **not compiled into the shipped binary** — zero runtime reachability today. Fix is a
  framework-level concern (bump Tauri to pull gtk-rs ≥ 0.20) **or** a documented, target-scoped audit
  suppression. **Higher blast radius; deliberate.**

## Scope

In scope (this task):
- Triage + risk/owner/test design for both chains; the split recommendation and proposed task names.

Out of scope (explicitly — handed to the split tasks):
- Running `npm update`, editing `package.json`/`package-lock.json`, bumping Tauri/`Cargo.toml`/`Cargo.lock`,
  or adding `deny.toml`/`cargo audit` config. **No dependency files are edited in TASK-043.**
- Any source, schema, IPC, or capability change.

## Owner

SW Architect (SW-1) owns this triage + `arch-review.md`. Implementation owners are assigned per stream in
`arch-review.md` and `tasks.md` (Stream A → SW-2/devops; Stream B → SW-backend + devops, with SW-3 QA on
the packaged app if the framework is bumped).

## Impact

- Affected code: **none** in this task (design only).
- Tier: L1 (triage doc). The downstream streams are L1 (A) and L2 (B) respectively.
- Risk: none here. The recommendation lowers future-gate noise and prevents a risky framework bump from
  blocking the safe npm patch.
- Recommendation envelope: **split-required** — see `handoff.md` for the proposed task names and routing.
