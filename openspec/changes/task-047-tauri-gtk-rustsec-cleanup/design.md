# Design — TASK-047 Tauri/GTK RustSec cleanup

- **Reviewer**: SW Architect (SW-1) · **Date**: 2026-06-21
- **Type**: dependency/security-posture; config + docs only, **no shipped-crate delta**
- **Decision**: **(c) Deferred with documented risk acceptance**, enforced by a target-scoped `cargo-deny`
  advisory gate.

## 1. Verified ground truth (working tree, 2026-06-21)

| Fact | Value | Source |
|---|---|---|
| Tauri | declared `2.2`, resolved **2.11.2** (Tauri **v2**, not v1) | `src-tauri/Cargo.toml`, `Cargo.lock` |
| wry / tao | 0.55.1 / 0.35.3 | `Cargo.lock` |
| gtk3-rs chain | `glib` 0.18.5, `gtk`/`gdk`/`atk` 0.18.2, `gtk3-macros` 0.18.2, `proc-macro-error` 1.0.4 | `Cargo.lock` |
| Host / target | `aarch64-apple-darwin`; bundle `targets: "all"` (= all **macOS** bundle types, not cross-OS) | `rustc -vV`, `tauri.conf.json` |
| Updater Rust deps | **none** — `tauri-plugin-updater` absent from `Cargo.lock` | `Cargo.lock` |
| Existing audit gate | **none** in `.github/` | repo scan |

> **Premise correction.** The task brief says "Tauri v1". Verified false — this is **Tauri v2** (`tauri =
> "2.2"` → 2.11.2). The disposition is unchanged, but all reasoning below is v2-specific.

## 2. Reachability — empirically proven, not assumed

`cargo tree -i glib --target aarch64-apple-darwin` → **"nothing to print"**: the gtk3-rs chain is **not in
the macOS dependency graph**. The same query on `x86_64-unknown-linux-gnu` resolves
`glib ← atk ← gtk ← {muda, tao} ← tauri`. The chain is `[target.'cfg(...)']`-gated to Linux/BSD by
`muda`/`tao`/`wry` upstream and is **never compiled into the shipped macOS `.app`**. Runtime reachability
on the shipped target = **zero**. (Reproducible; this is the load-bearing fact for option (c).)

## 3. Advisories — verified against RustSec

| Advisory | Crate (ours) | Class | Affected → Patched | Reachable on macOS? | Source |
|---|---|---|---|---|---|
| RUSTSEC-2024-0429 | `glib` 0.18.5 | **Unsound** (`VariantStrIter` NULL-ptr UB) | `>=0.15.0,<0.20.0` → **`>=0.20.0`** | No | [advisory](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) |
| RUSTSEC-2024-0370 | `proc-macro-error` 1.0.4 | **Unmaintained** | no patch (use `manyhow`/`proc-macro2-diagnostics`) | No | [advisory](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) |
| RUSTSEC-2024-0411…0423 cluster | `atk` `atk-sys` `gdk` `gdk-sys` `gdkwayland-sys` `gdkx11(-sys)` `gtk` `gtk-sys` `gtk3-macros` | **Unmaintained** — "gtk-rs GTK3 bindings no longer maintained" | **no patch** (gtk3-rs EOL → gtk4-rs) | No | [RUSTSEC-2024-0415](https://rustsec.org/advisories/RUSTSEC-2024-0415.html); Tauri tracking [#11924](https://github.com/tauri-apps/tauri/issues/11924)–[#11929](https://github.com/tauri-apps/tauri/issues/11929) (auto-filed, **closed**) |

`glib` carries the **only** scored item (prior gates recorded CVSS ≈ 6.9, < 7.0 auto-fail bar; the RustSec
page itself lists no CVSS). Everything else is **unmaintained-class** (warnings, unscored), not exploitable
CVEs. The full Linux-gated gtk-rs family in our lock (28 crates incl. `cairo-rs`, `pango`, `soup3`,
`webkit2gtk`, `gio`, `gobject-sys`) is enumerated by the implementer from a real scan — see §6.

## 4. Why (a) and (b) are not actionable

- **(a) compatible bump.** Tauri 2.x's Linux backend is **GTK3 + webkit2gtk-4.1** ([Tauri
  prerequisites](https://v2.tauri.app/start/prerequisites/); [alpha-3 migration
  note](https://v2.tauri.app/blog/tauri-2-0-0-alpha-3/)). gtk3-rs is EOL — the unmaintained advisories
  have **no patched version**, so no bump within the gtk3 line clears them. `glib ≥ 0.20` exists only for
  the **gtk4** ecosystem, which the gtk3 webkit backend does not consume. A Tauri patch bump (2.11.2 → a
  newer 2.x) would churn the lock **without clearing a single Stream-B advisory**.
- **(b) major-migration spike.** There is no Tauri release that drops GTK3 and no official GTK4 Linux-backend
  roadmap. A spike has nothing concrete to evaluate, for crates that don't ship on macOS. Premature.

### Correction to TASK-043

`task-043/arch-review.md` Stream B "Option 1" assumed a Tauri bump could "pull gtk-rs ≥ 0.20." **Verified
false** (this §). gtk3-rs is end-of-life; the only true fix is an upstream migration of the Tauri Linux
backend off GTK3, which does not exist today. Option 1 is therefore **not** a near-term path; the resolution
collapses to a documented, target-scoped acceptance (TASK-043's "Option 2"), refined below.

## 5. Mechanism — target-scoping, not a blanket ignore list

The TASK-043 guardrail requires the suppression to be scoped so that **adding a Linux target re-surfaces
`glib` 6.9 rather than burying it**. An advisory-ID `ignore = [...]` list does **not** satisfy that intent —
an ignored ID stays suppressed on every target, including a future Linux build. The mechanism that makes the
tripwire real is **`[graph].targets` scoped to the shipped Apple targets**: on those targets the gtk3 crates
are absent, so nothing fires; the day a Linux triple is added to that list (which a Linux build *requires*),
the advisories re-appear and must be re-addressed.

So: **enforce via `[graph].targets` (tripwire-correct); document the deferred IDs in this design + a
deny.toml comment block (auditable).** This honors the guardrail's *intent* over its literal "ignore-list"
wording, and is the architecturally sound choice. If CI ever invokes `cargo deny` without target scoping,
fall back to an ID-scoped `ignore` with per-ID comments — second choice, because it buries the tripwire.

### Reference `deny.toml` (implementer finalizes syntax against installed cargo-deny version)

```toml
# src-tauri/deny.toml — advisory posture for vire (macOS-only target; see TASK-047 design.md).
# Enforcement is target-scoped: gtk3-rs/glib are Linux-backend crates and are NOT in the macOS
# dependency graph, so they do not fire here. Adding a Linux triple below re-surfaces glib
# RUSTSEC-2024-0429 + the gtk3-rs unmaintained cluster — that is the intended tripwire. Do NOT
# add a blanket [advisories].ignore for those IDs; that would bury them on a future Linux build.
[graph]
targets = [
    { triple = "aarch64-apple-darwin" },
    { triple = "x86_64-apple-darwin" },
]

[advisories]
version = 2
yanked = "deny"
# ignore = []  # intentionally empty — Apple-target graph is clean by construction.
```

## 6. Implementation outline (full checklist in `tasks.md`)

1. Add `src-tauri/deny.toml` as above; finalize TOML against the installed `cargo-deny`.
2. Run `cargo deny check advisories --manifest-path src-tauri/Cargo.toml` → must be **clean**.
3. As evidence, run one **full-target** advisory scan (`cargo deny check advisories` with default/all
   targets, or `cargo audit`) and paste the resulting Linux-only advisory IDs into a "Deferred advisories"
   comment in `deny.toml` (and `sec.md`) — generated from the scan, **not** hand-copied, so IDs are exact.
4. Wire a CI step running the scoped check; document the local one-liner in `RELEASE.md`/contributor docs.
5. Confirm `git diff main -- src-tauri/Cargo.lock src-tauri/Cargo.toml` is **empty**.

## 7. NFR / boundaries

- **No NFR impact** — no runtime code path, startup, or artifact changes. macOS `.app` byte-identical.
- **No BA escalation.** macOS-only is an established decision (DEC-026; `keyring` apple-native). No
  `feedback_to_ba[]`.
- **Review trigger (tripwire).** Re-open this posture when **(i)** a Linux build target is added, or
  **(ii)** Tauri's Linux backend migrates off GTK3 — at which point `glib ≥ 0.20` becomes genuinely
  in-scope and a real bump + packaged-app QA (SW-3) is required.

## 8. Tooling note

`cargo-deny`/`cargo-audit` are **not installed** in this environment, so the live scan is specified for the
implementer rather than run here. The Sonatype guide MCP was unavailable (interactive auth required in this
headless run) — version facts above come from RustSec, Tauri official docs, and local `cargo tree`/`Cargo.lock`.
