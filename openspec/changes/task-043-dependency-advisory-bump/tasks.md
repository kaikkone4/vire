# Tasks — TASK-043 dependency advisory bump (split into A + B)

Design + rationale: `arch-review.md`. **This file is the implementer checklist for the split tasks** —
SW-1 (this triage) does NOT execute any of it. Do the minimum; no unrelated dep churn.

## Stream A — npm dev-deps (→ TASK-043, ship now) — Owner: SW-2 / devops

- [ ] A.1 `npm update vite esbuild` so `vite` resolves to **6.4.3** (within the declared `^6.0.7`) and
  the transitive `esbuild` to **≥ 0.28.1**. This is a `package-lock.json` + `node_modules` refresh.
- [ ] A.2 (Optional) Raise the declared floor in `package.json` `devDependencies.vite` to `^6.4.3` to
  pin the fix. Do **not** touch any other dependency.
- [ ] A.3 `npm run build` (tsc + `vite build`) succeeds and emits a working `dist/`.
- [ ] A.4 `npm run test:frontend` green.
- [ ] A.5 Re-run OSV-scanner (and Trivy `fs`) on `package-lock.json`; confirm **0 findings ≥ CVSS 7.0**
  and that GHSA-fx2h-pf6j-xcff / GHSA-v6wh-96g9-6wx3 are gone.
- [ ] A.6 Confirm **no `Cargo.*` change** in the diff (this stream is npm-only).

## Stream B — Tauri/GTK RUSTSEC (→ TASK-044, scheduled) — Owner: SW-backend + devops

Choose ONE resolution per `arch-review.md` (recommended: B-option-2 now).

**Option 2 (recommended, low-risk, no shipped-artifact delta):**
- [ ] B2.1 Add `cargo-deny` (`deny.toml`) or `cargo audit` config with an `[advisories] ignore` list
  containing **only** RUSTSEC-2024-0429 (`glib`) and the unscored gtk-rs/`atk`/`gdk`/`gtk`/`proc-macro-error`/`unic-*`
  Linux-backend advisories.
- [ ] B2.2 Each ignored ID carries a comment: justified **only** by the macOS-only target; adding a Linux
  target must re-surface `glib` 6.9 (do NOT blanket-ignore).
- [ ] B2.3 `cargo audit` / `cargo deny check advisories` runs clean with the documented ignores.
- [ ] B2.4 Confirm `git diff main -- src-tauri/Cargo.lock` shows **no shipped-crate delta** (config-only).

**Option 1 (framework bump — only if/when bundled with a Tauri upgrade):**
- [ ] B1.1 Bump `tauri`/`tauri-build`/plugins (and transitively wry/tao) to a release pulling gtk-rs ≥ 0.20;
  regenerate `Cargo.lock`.
- [ ] B1.2 `cargo test --manifest-path src-tauri/Cargo.toml` (all Rust unit + integration tests) green.
- [ ] B1.3 Full `tauri build` produces a working `.app`.
- [ ] B1.4 **SW-3 manual smoke on the packaged macOS app:** Keychain set/clear, save-dialog, Langfuse
  importer round-trip, capabilities/CSP byte-identical to `main`.
- [ ] B1.5 Re-run OSV-scanner on `Cargo.lock`; confirm RUSTSEC-2024-0429 cleared.

## Notes

- Streams A and B are **independent**; A must not wait on B. See `arch-review.md` "Split decision".
- Nothing in TASK-043 (this triage) edits `package.json`, `package-lock.json`, `Cargo.toml`, `Cargo.lock`,
  or any source/config file.
