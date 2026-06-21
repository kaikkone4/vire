# Architecture review — TASK-047 Tauri/GTK RustSec cleanup (SW-1)

- **Change**: `openspec/changes/task-047-tauri-gtk-rustsec-cleanup/`
- **Reviewer**: SW Architect · **Date**: 2026-06-21
- **Verdict**: **PASS** — single coherent task, no split, no BA escalation. Decision **(c) deferred with
  documented risk acceptance**, enforced by a target-scoped `cargo-deny` advisory gate. Implementation can
  proceed; route to **SW-backend + devops**.

## Decision

Of the four envelopes — (a) compatible bump, (b) Tauri major-migration spike, (c) deferred risk acceptance,
split-required — the answer is **(c)**, on verified evidence:

1. **Zero reachability on the shipped target (proven).** `cargo tree -i glib --target aarch64-apple-darwin`
   = nothing. The gtk3-rs chain (`glib`/`gtk`/`gdk`/`atk`/`gtk3-macros`/`proc-macro-error`) is Linux/BSD-gated
   via `muda`/`tao`/`wry` and is never compiled into the macOS `.app`.
2. **No upstream fix exists.** RustSec marks the gtk3-rs bindings "no longer maintained" with **no patched
   version** (EOL); `glib` RUSTSEC-2024-0429 is fixed only in `glib ≥ 0.20`, which is gtk4-ecosystem and
   unavailable to the gtk3 backend. Tauri 2.x still ships GTK3 + webkit2gtk-4.1 on Linux. → option (a) cannot
   clear anything.
3. **No migration target.** No Tauri release drops GTK3; no GTK4 Linux-backend roadmap. → option (b) is
   premature for non-shipped code.

Full sources and the deny.toml design are in `design.md`.

## Correction to prior architecture (TASK-043 Stream B, Option 1)

`task-043/arch-review.md` assumed a Tauri bump could "pull gtk-rs ≥ 0.20." **Verified false.** gtk3-rs is
end-of-life; the only genuine remediation is an upstream Tauri Linux-backend migration off GTK3, which does
not exist today. The resolution therefore collapses to a **target-scoped, documented acceptance**.

## Component-boundary / NFR check

- **No boundary crossed.** Config + docs only; the shipped macOS `.app` is byte-identical. No frontend↔native
  surface change, no schema/IPC/capability/CSP delta.
- **No NFR regression.** No runtime code path or startup behavior touched.
- **Guardrail honored — refined.** TASK-043 required the suppression to *re-surface* `glib` 6.9 if Linux is
  added. An ID-scoped `ignore` list buries it; **`[graph].targets` scoping** is the mechanism that makes the
  tripwire real. Enforcement = target scoping; auditability = a documented, scan-generated list of deferred
  IDs. See `design.md` §5.

## Escalation

- **None to BA.** macOS-only is an established project decision (DEC-026; `keyring` apple-native). No
  `feedback_to_ba[]` entry warranted.
- **One forward review trigger (tripwire):** re-open the posture when a Linux build target is added **or**
  Tauri's Linux backend migrates off GTK3 — then `glib ≥ 0.20` is genuinely in-scope and a real bump +
  SW-3 packaged-app QA is required.

## Handoff

To **SW-backend + devops** — add `src-tauri/deny.toml` (target-scoped), wire the `cargo deny check
advisories` gate, run the scan to confirm clean and to generate the deferred-ID list, prove the tripwire
(V3), keep `Cargo.lock` delta empty. Then SW-3 QA / SW-5 Security per normal gate flow.
