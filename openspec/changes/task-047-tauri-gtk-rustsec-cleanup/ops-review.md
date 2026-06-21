# Ops review — TASK-047 Tauri/GTK RustSec cleanup (SW-2 DevOps)

- **Role**: DevOps (SW-2) · **Date**: 2026-06-21 · **Branch**: `feat/task-047-tauri-gtk-rustsec-cleanup`
- **Decision (fixed by SW-1)**: (c) deferred w/ documented risk acceptance, enforced by a **target-scoped
  `cargo-deny` advisory gate**. See `design.md`. No `Cargo.*`/source/capability/`tauri.conf.json` edit.
- **Tier**: L2 security-posture. CI gate added; no runtime/observability change (no shipped code path).

## 1. What was implemented

| Artifact | Path | Purpose |
|---|---|---|
| cargo-deny config | `src-tauri/deny.toml` | Apple-target-scoped advisory gate + auditable ACCEPTED/DEFERRED blocks |
| CI gate | `.github/workflows/dependency-advisories.yml` | runs pinned `cargo deny check advisories` (cargo-deny **0.19.9**) on PR/push/dispatch |
| Contributor docs | `RELEASE.md` (v0.6.3 entry) | local one-liner + posture summary |

- **Mechanism = `[graph].targets`, not a blanket ID ignore list** (per `design.md` §5 / spec). The 12
  Linux-only gtk3-rs/glib/proc-macro-error IDs stay **OUT** of `ignore` so the tripwire is real. The 5
  Apple-present `unic-*` IDs (genuinely in the shipped graph) are accepted via a **documented, scoped**
  `ignore`. `unmaintained = "all"` keeps the unmaintained cluster armed when a Linux triple is later added.
- **Local one-liner** (documented in `RELEASE.md`): `cd src-tauri && cargo deny check advisories`.

## 2. Tooling note — pinned scanner; project-local verification

**Pinned scanner version: `cargo-deny 0.19.9`** (recorded in `deny.toml` header, the CI workflow
`tool: cargo-deny@0.19.9`, and `tasks.md` T2). `unmaintained = "all"` requires cargo-deny **≥ 0.18**;
0.16.4 rejects the key (removed in the advisories v1→v2 migration, cargo-deny PR #611).

The scans below were run with the **real pinned binary**, installed **project-local** (not a system
install, per policy): `cargo install cargo-deny --version 0.19.9 --root <tmp> --locked`. CI runs the same
pinned version via `taiki-e/install-action@v2` — the enforced, repeatable scan going forward.
`osv-scanner` (already on PATH) was used as an independent cross-check against the same RustSec data.

## 3. Evidence — real scanner runs + reachability proof

**Real `cargo deny check advisories` (cargo-deny 0.19.9):**

- **Scoped (Apple targets, shipped `deny.toml`)** → `advisories ok`, **exit 0**. Pre-fix (with `ignore=[]`)
  it errored on exactly the 5 `unic-*` advisories — empirically confirming SW-5 SEC-001 — after which the
  documented `ignore` for those 5 makes the gate green.
- **Full-target (Apple + `x86_64-unknown-linux-gnu`, `ignore=[]`)** → **FAILED**, 16 `unmaintained` errors:
  the 5 Apple `unic-*` + 11 Linux-only (`proc-macro-error` RUSTSEC-2024-0370 + gtk3-rs
  RUSTSEC-2024-0411–0420). This is the tripwire demonstrated by the real binary.

**`cargo tree` reachability** against `src-tauri/Cargo.lock` (cargo 1.95.0):

- `cargo tree -i glib --target aarch64-apple-darwin` → **"nothing to print"** — gtk3-rs/glib **absent** from
  the macOS graph.
- `cargo tree -i glib --target x86_64-unknown-linux-gnu` → `glib ← atk ← gtk ← {muda, tao} ← tauri ← vire`.
- `cargo tree -i unic-ucd-ident --target aarch64-apple-darwin` → `unic-ucd-ident → urlpattern → tauri-utils
  → {tauri-build (build-dep), tauri} → vire` — the 5 `unic-*` **ARE** in the shipped macOS graph.

→ The gtk3-rs chain is `cfg`-gated to Linux/BSD upstream; macOS runtime reachability = **zero**. The
`unic-*` cluster is gtk-unrelated and genuinely Apple-present (accepted, not deferred).

## 4. Advisory inventory — 17 total = 12 Linux-only deferred + 5 Apple-present accepted

**Correction to the prior draft ("Apple-target graph clean by construction / 12 IDs total").** That was
**false**: the shipped Apple graph is *not* advisory-free. A real `cargo deny check advisories` (cargo-deny
0.19.9) and an independent `osv-scanner` run over `src-tauri/Cargo.lock` both surface **17** advisories
against our locked crates, split into two groups:

### Group A — 12 Linux-only (DEFERRED, target-scoped OUT of the Apple graph)

Absent from the `aarch64`/`x86_64-apple-darwin` graph (cargo tree proof §3), so they do not fire on the
shipped target and are **not** in `ignore`. The full Apple+Linux scan re-reports the **11 unmaintained**
ones; `glib` RUSTSEC-2024-0429 is *unsound*-class and not enforced by cargo-deny v2 (caught by osv-scanner).

| Advisory | Crate (locked) | Class | cargo-deny v2 enforces? | Upstream fix |
|---|---|---|---|---|
| RUSTSEC-2024-0429 | `glib` 0.18.5 | unsound (CVSS 6.9) | **no** — informational; osv-scanner only | `glib ≥ 0.20.0` (gtk4 ecosystem; gtk3 backend cannot consume it) |
| RUSTSEC-2024-0370 | `proc-macro-error` 1.0.4 | unmaintained | yes | none (`manyhow` / `proc-macro2-diagnostics`) |
| RUSTSEC-2024-0411 | `gdkwayland-sys` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0412 | `gdk` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0413 | `atk` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0414 | `gdkx11-sys` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0415 | `gtk` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0416 | `atk-sys` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0417 | `gdkx11` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0418 | `gdk-sys` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0419 | `gtk3-macros` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |
| RUSTSEC-2024-0420 | `gtk-sys` 0.18.2 | unmaintained | yes | none — gtk3-rs EOL → gtk4-rs |

### Group B — 5 Apple-present (ACCEPTED via documented `ignore`)

In the shipped macOS graph via `urlpattern → tauri-utils → {tauri-build (build-dep), tauri} → vire`
(cargo tree §3). With `ignore=[]` these are exactly the 5 advisories that error on the Apple-scoped gate
(SW-5 SEC-001); accepting them is what makes the scoped check exit 0. All from the unmaintained
`open-i18n/rust-unic` project (RUSTSEC announcement rustsec/advisory-db#2414), unscored, no safe upgrade.

| Advisory | Crate (locked) | Class | Upstream fix |
|---|---|---|---|
| RUSTSEC-2025-0081 | `unic-char-property` 0.9.0 | unmaintained | none — rust-unic EOL |
| RUSTSEC-2025-0075 | `unic-char-range` 0.9.0 | unmaintained | none — rust-unic EOL |
| RUSTSEC-2025-0080 | `unic-common` 0.9.0 | unmaintained | none — rust-unic EOL |
| RUSTSEC-2025-0100 | `unic-ucd-ident` 0.9.0 | unmaintained | none — rust-unic EOL |
| RUSTSEC-2025-0098 | `unic-ucd-version` 0.9.0 | unmaintained | none — rust-unic EOL |

**Note on `design.md` §3:** the design referenced the gtk3 cluster approximately as "RUSTSEC-2024-0411…0423"
and did not enumerate the `unic-*` cluster. The exact gtk3-rs range is **…0411–0420** (10 IDs). The core
gtk-rs bindings shared with gtk4 (`cairo-rs`, `pango`, `gdk-pixbuf`, `gio`, `gobject-sys`, `soup3`,
`webkit2gtk`, `javascriptcore-rs` and their `-sys`) are in the Linux graph but carry **no** RustSec advisory.

Both groups are mirrored in `deny.toml` (Group B → `ignore` + ACCEPTED block; Group A → DEFERRED block).
SW-5 records the same split + risk acceptance in `sec.md`.

## 5. Verification status

| Gate | Status | Note |
|---|---|---|
| V1 scoped check exits 0 on Apple targets | **PASS (real binary)** | `cargo-deny 0.19.9 check advisories` → `advisories ok`, exit 0 (§3). 5 `unic-*` accepted via documented `ignore`; gtk3-rs/glib target-scoped out. CI re-runs the pinned version. |
| V2 `git diff main -- src-tauri/Cargo.{toml,lock}` empty | **PASS** | verified empty (config/docs only) |
| V3 tripwire re-reports on Linux triple | **PASS (real binary)** | full-target run (Apple+Linux, `ignore=[]`) errors on the 11 Linux unmaintained IDs (§3); `glib` unsound via osv-scanner; corroborated by `cargo tree`. Shipped `deny.toml` scoping unchanged. |
| V4 `cargo test` still green | **N/A by construction** | no source/dep change; deny.toml/workflow are not compiled |
| V5 `sec.md` records deferred IDs | **handoff to SW-5** | inputs in §4 + `deny.toml` (both groups) |

> V1/V3 are now evidenced by **real** `cargo-deny 0.19.9` runs (project-local temp install, not system), and
> re-enforced by the CI gate's pinned-version run on every PR/push. `cargo tree` and `osv-scanner` provide
> independent corroboration of the same dependency-graph signal.

## 6. CI / DevOps notes

- Runner: **ubuntu-latest**. The advisory check is graph-only and **target-scoped** via `deny.toml`, so it
  evaluates the Apple-target graph regardless of runner OS — ubuntu is chosen purely for speed/cost. The
  gate's correctness comes from `[graph].targets`, not the runner. Documented in the workflow header.
- `permissions: contents: read` (least privilege). Triggers: `pull_request`, `push` to `main`,
  `workflow_dispatch`. No secrets used.
- Actions pinned to major versions: `actions/checkout@v4`, `dtolnay/rust-toolchain@stable`,
  `taiki-e/install-action@v2`. The **cargo-deny binary is pinned to `0.19.9`** (`tool: cargo-deny@0.19.9`)
  so the advisory schema/behavior is reproducible — not just the installer action. A future hardening task
  may pin actions to commit SHAs.
- Scope held: advisories-only. `[bans]`/`[licenses]` left permissive (out of TASK-047 scope) so the
  advisory gate runs standalone without a license inventory.

## 7. Handoffs

- → **SW-5 (security)**: write `sec.md` from §4 (deferred IDs + reachability proof + risk acceptance +
  review trigger). All inputs are here and in `deny.toml`.
- → **SW-3 (QA)**: re-enter per the tripwire if a Linux build target is added or Tauri's Linux backend
  migrates off GTK3 (then `glib ≥ 0.20` becomes in-scope → real bump + packaged-app QA).
- → **SW-6 (release)**: `RELEASE.md` v0.6.3 entry drafted; finalize version number at release time.
