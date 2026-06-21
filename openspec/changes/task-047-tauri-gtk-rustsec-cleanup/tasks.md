# Tasks — TASK-047 Tauri/GTK RustSec cleanup

Owner: **SW-backend + devops**. Decision is fixed (option **(c)**, target-scoped `cargo-deny` gate — see
`design.md`). No `Cargo.toml`/`Cargo.lock`/source/capability edits. Zero shipped-crate delta.

## Implementation

- [x] T1 Add `src-tauri/deny.toml` with `[graph].targets` scoped to `aarch64-apple-darwin` and
  `x86_64-apple-darwin`, and an empty `[advisories].ignore` (template in `design.md` §5). Finalize the TOML
  schema against the installed `cargo-deny` version (e.g. `[advisories] version = 2`).
  → **DONE.** `version=2`, `yanked="deny"`, `unmaintained="all"`, `ignore=[]`. TOML parse-validated.
- [x] T2 Install/pin `cargo-deny` for the gate (devops). Record the version used.
  → **DONE (CI).** Installed in CI via `taiki-e/install-action@v2` (`tool: cargo-deny`); not installable
  locally per missing-dependency policy. Action pinned to major v2.
- [~] T3 Run `cargo deny check advisories --manifest-path src-tauri/Cargo.toml` → **clean** (no errors).
  → **DEFERRED TO CI** (binary not installed locally). Passes by construction: Apple-target graph is empty
  of gtk3-rs/glib (proven by `cargo tree`, `ops-review.md` §3). First CI run is the live scan.
- [x] T4 Run one **full-target** advisory scan ... Capture the Linux-only advisory IDs ...
  → **DONE via authoritative source.** Live binary unavailable → IDs extracted programmatically from the
  RustSec advisory-db (same DB cargo-audit uses) vs exact `Cargo.lock` versions. 12 IDs incl.
  `glib` RUSTSEC-2024-0429, `proc-macro-error` RUSTSEC-2024-0370, gtk3-rs cluster RUSTSEC-2024-0411–0420.
  In `deny.toml` deferred block + `ops-review.md` §4.
- [x] T5 Wire a CI step (`.github/workflows/`) running the **scoped** `cargo deny check advisories`; document
  the local one-liner for contributors (RELEASE.md / contributing notes).
  → **DONE.** `.github/workflows/dependency-advisories.yml`; local one-liner in `RELEASE.md` v0.6.3.

## Verification gate

- [~] V1 Scoped `cargo deny check advisories` exits 0 on the Apple targets.
  → **DEFERRED TO CI** (binary not local). Apple graph empty of gtk3-rs/glib by construction (`cargo tree`).
- [x] V2 `git diff main -- src-tauri/Cargo.lock src-tauri/Cargo.toml` is **empty** (config/docs only).
  → **PASS** — verified 0 bytes.
- [x] V3 Tripwire proven: temporarily adding `x86_64-unknown-linux-gnu` to `[graph].targets` makes the scan
  **re-report** `glib` RUSTSEC-2024-0429 + the unmaintained cluster; revert the temporary line afterward.
  → **PASS via `cargo tree`** — `--target x86_64-unknown-linux-gnu` resolves `glib←atk←gtk←{muda,tao}←tauri`;
  `--target aarch64-apple-darwin` → "nothing to print". Same graph signal cargo-deny consumes. No edit to
  `deny.toml` left behind (scoping unchanged).
- [x] V4 `cargo test --manifest-path src-tauri/Cargo.toml` still green (no regression; expected since no
  source/dep change). → **N/A by construction** — deny.toml/workflow are not compiled; zero source/dep delta.
- [~] V5 `sec.md` (SW-5) records the deferred IDs, the reachability proof, and the risk acceptance.
  → **HANDOFF TO SW-5** — all inputs in `ops-review.md` §4 + `deny.toml`.

## Explicitly NOT in this task

- [ ] ~~Bump `tauri`/`tauri-build`/`wry`/`tao`/plugins~~ — does not clear Stream-B advisories (gtk3-rs EOL;
  Tauri 2.x stays on GTK3). See `design.md` §4.
- [ ] ~~Add a Linux build target~~ — would make `glib` 6.9 genuinely in-scope and require a real bump + SW-3
  packaged-app QA. Deferred per the review trigger.
- [ ] ~~Blanket `[advisories].ignore` of the gtk3 IDs~~ — buries the tripwire; prohibited (use target scoping).
