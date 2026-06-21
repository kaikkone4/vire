# Architecture review — TASK-043 dependency advisory follow-up (SW-1)

- **Change**: `openspec/changes/task-043-dependency-advisory-bump/`
- **Type**: triage / design only — **no implementation**, no dependency files edited.
- **Reviewer**: SW Architect · **Date**: 2026-06-20
- **Verdict**: **SPLIT-REQUIRED** — two independent dependency chains with different risk, owner, test
  strategy, and cadence. Do not hand both to one developer as a single "bump" task.

## Source of truth for the advisories

Advisory IDs/CVSS below are taken verbatim from the recorded OSV-scanner runs in two prior gates, not
re-derived here:
- `openspec/changes/task-032-ai-time-suggestions/sec.md` — §"OSV findings — triage" + §Advisories.
- `openspec/changes/task-026-desktop-production-readiness/sec.md` — §4 "Advisory findings".

Current observed state (working tree, 2026-06-20): `node_modules/vite` = **6.4.2**; declared
`devDependencies.vite` = **^6.0.7** (`package.json`); `Cargo.lock` has `tauri` **2.11.2**, `glib`
**0.18.5**, `gtk`/`gdk` **0.18.2**. The shipped artifact is the Tauri `.app` (Rust binary + static
`dist/`); `tauri.conf.json` `frontendDist: "../dist"`, dev server `127.0.0.1:1420` (loopback).

---

## Stream A — npm dev-deps (`vite`, `esbuild`)

| Property | Finding |
|---|---|
| **Advisories** | `vite` GHSA-fx2h-pf6j-xcff (CVSS 8.2), GHSA-v6wh-96g9-6wx3 (CVSS 5.5); `esbuild` (CVSS 2.5–8.1 across the two recorded scans). All **dev-server**-class (file-serve / CORS). |
| **Classification** | `vite` + `tsx` + `@tauri-apps/cli` are `devDependencies`; runtime `dependencies` are only `@tauri-apps/api`, `@tauri-apps/plugin-dialog`. `esbuild` is transitive via `vite`. |
| **Reachability in shipped artifact** | **None.** The packaged `.app` runs the Rust binary serving static `dist/`; there is **no dev server** at runtime. The vite/esbuild dev-server attack surface is absent in the deliverable (consistent with both prior triages). |
| **Minimal fix** | `npm update vite esbuild` → `vite` **6.4.3** (clears both vite GHSAs), which pulls the fixed `esbuild` (**≥ 0.28.1**). **6.4.3 is inside the already-declared `^6.0.7` range** → a **lockfile + node_modules refresh**, no `package.json` edit required. Optionally raise the declared floor to `^6.4.3` to pin the fix. |
| **Risk** | **Low.** Semver-compatible patch within vite 6.x; dev/build tooling only; not shipped. The only failure mode is a build/test regression, caught by the gate below. |
| **Owner** | SW-2 (frontend) or devops. |
| **Tests / gate** | `npm run build` (tsc + `vite build`) must succeed and emit a working `dist/`; `npm run test:frontend` green. **No packaged-app rebuild needed.** Re-run OSV/Trivy on `package-lock.json` to confirm 0 ≥7.0. |
| **Cadence** | **Ship now / next release.** Independent of Stream B. |

## Stream B — Tauri/GTK Rust chain (`glib` + gtk-rs/unic transitive)

| Property | Finding |
|---|---|
| **Advisories** | `glib` 0.18.5 **RUSTSEC-2024-0429** (CVSS **6.9**, < 7.0 auto-fail bar) + ~15 **unscored** "unmaintained"-class RUSTSEC on `atk`, `gdk*`, `gtk*`, `gtk3-macros`, `proc-macro-error`, `unic-*`. |
| **Root cause** | The **gtk-rs 0.18** bindings (`glib`/`gdk`/`gtk`/`atk` 0.18.x) are pulled transitively through the Tauri → wry → tao **Linux** WebKit2GTK GUI backend. |
| **Reachability in shipped artifact** | **None on the current target.** Vire is **macOS-only** (mac-first prototype; mac uses the WebKit/Cocoa backend). The GTK crates are **not compiled into the macOS `.app`**. They become reachable **only if a Linux build target is ever added** — at which point `glib` 6.9 is genuinely in-scope. |
| **Resolution options** | **(1) Framework bump** — raise Tauri (→ wry/tao) to a release whose Linux backend references gtk-rs ≥ 0.20, clearing `glib` 6.9 (the unscored unmaintained-crate set may only partially clear, depending on ecosystem movement). **(2) Documented, target-scoped audit suppression** — add `cargo-deny`/`cargo audit` config (`deny.toml [advisories] ignore`) listing the Linux-backend advisories with a rationale tied to the macOS-only target, so the gate signal stays clean and honest **without** churning the native framework for non-shipped code. |
| **Recommendation** | **Prefer (2) now, (1) later.** The advisories are non-reachable on the shipped target, so a standalone framework bump is disproportionate risk. Add a **target-scoped, justified** suppression now (must be scoped/commented so it does **not** silently hide `glib` 6.9 if Linux is later targeted), and fold the real gtk-rs/glib version bump into the **next routine Tauri upgrade**. |
| **Risk** | Option 1 = **medium-high** (native build churn: rebuild + full smoke of keychain, dialogs, importer, capabilities/CSP on the packaged `.app`). Option 2 = **low** (config + docs; zero artifact delta). |
| **Owner** | SW-backend + devops. If Option 1 is chosen, **SW-3 QA must verify the packaged macOS `.app`** (not just `cargo test`). |
| **Tests / gate** | Option 1: `cargo test` (94+ Rust), full `tauri build`, manual smoke (Keychain set/clear, save-dialog, Langfuse importer, capability/CSP unchanged). Option 2: `cargo audit`/`deny check` clean with the documented ignores; confirm `git diff main -- Cargo.lock` shows **no shipped-crate delta**. |
| **Cadence** | **Deliberate / scheduled.** Never blocks a feature release. |

---

## Split decision — **YES (split-required)**

The two chains differ on every axis that defines a task boundary:

| Axis | Stream A (npm) | Stream B (Tauri/GTK) |
|---|---|---|
| Component boundary | Frontend build tooling | Native Rust framework |
| Change surface | Lockfile refresh | Framework bump **or** audit config |
| Risk | Low | Medium-high (1) / Low (2) |
| Owner | SW-2 / devops | SW-backend + devops |
| Verification | `npm build` + frontend tests | Packaged-app QA (SW-3) / audit-config check |
| Cadence | Ship now | Scheduled |

Coupling them forces the safe, one-line `vite` patch to wait on a framework-upgrade decision — exactly
the stall that has let these advisories ride along for six+ gates. Per the role rule "if a task's scope
would cross component boundaries, stop and split," this crosses the frontend↔native boundary. **Split.**

### Proposed split (names for Pi-Assistant to route)

1. **TASK-043** (this dir, Stream A) — *npm dev-dep advisory bump*: `npm update vite esbuild` →
   vite 6.4.3 / esbuild ≥ 0.28.1, lockfile refresh, `tasks.md` §A gate. L1, SW-2/devops, ship now.
2. **TASK-044** (new) — *Tauri/GTK RUSTSEC resolution*: adopt the documented target-scoped suppression
   now; fold the gtk-rs/glib bump into the next Tauri upgrade. L2, SW-backend + devops, scheduled.

## Non-functional / architecture notes

- **No NFR regression either way.** Stream A touches no runtime code path. Stream B Option 2 changes no
  shipped crate; Option 1's only NFR-relevant risk is native-build/startup behavior, covered by the
  packaged-app smoke gate.
- **No BA escalation.** The macOS-only target is an established project decision; no `feedback_to_ba[]`
  entry is warranted. This stays entirely within SW dependency maintenance.
- **Guardrail for Stream B:** any audit suppression MUST be advisory-ID-scoped and carry a comment that
  it is justified *only* by the macOS-only target — so adding a Linux target later re-surfaces `glib` 6.9
  rather than burying it.
