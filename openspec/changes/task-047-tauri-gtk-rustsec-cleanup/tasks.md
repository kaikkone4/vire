# Tasks — TASK-047 Tauri/GTK RustSec cleanup

Owner: **SW-backend + devops**. Decision is fixed (option **(c)**, target-scoped `cargo-deny` gate — see
`design.md`). No `Cargo.toml`/`Cargo.lock`/source/capability edits. Zero shipped-crate delta.

## Implementation

- [x] T1 Add `src-tauri/deny.toml` with `[graph].targets` scoped to `aarch64-apple-darwin` and
  `x86_64-apple-darwin` (template in `design.md` §5). Finalize the TOML schema against the **pinned**
  `cargo-deny` version (`[advisories] version = 2`).
  → **DONE.** `version=2`, `yanked="deny"`, `unmaintained="all"`. `ignore` holds the **5 Apple-present**
  `unic-*` IDs (RUSTSEC-2025-0075/0080/0081/0098/0100) only — documented, scoped. The 12 Linux-only
  gtk3-rs/glib/proc-macro-error IDs stay OUT of `ignore` (target-scoped). Config validated by the real
  pinned binary (`cargo-deny 0.19.9`).
- [x] T2 Install/pin `cargo-deny` for the gate (devops). Record the version used.
  → **DONE & PINNED. Version = `cargo-deny 0.19.9`.** CI installs it pinned via
  `taiki-e/install-action@v2` with `tool: cargo-deny@0.19.9` (binary version pinned, not just the
  installer action). Recorded in `deny.toml` header, this line, and `ops-review.md` §2. Verified locally
  with a project-local temp install (`cargo install cargo-deny --version 0.19.9 --root <tmp> --locked`,
  **not** a system install, per policy). Note: `unmaintained = "all"` requires cargo-deny ≥ 0.18 — 0.16.4
  rejects it (key removed in advisories v1→v2 migration, PR #611).
- [x] T3 Run `cargo deny check advisories` (scoped) → **clean** (exit 0).
  → **DONE — real binary run.** `cargo-deny 0.19.9 check advisories` against the Apple-scoped
  `deny.toml` exits **0** (`advisories ok`). Pre-fix run (ignore=[]) errored on exactly the 5 `unic-*`
  (empirically confirms SW-5 SEC-001); after adding them to the documented `ignore`, the scoped gate is
  green. Evidence in `ops-review.md` §3/§5.
- [x] T4 Run one **full-target** advisory scan; capture the Linux-only advisory IDs.
  → **DONE — real binary run.** `cargo-deny 0.19.9` over Apple+`x86_64-unknown-linux-gnu` with `ignore=[]`
  errors on **16** advisories, all `unmaintained`: 5 Apple-present `unic-*` + 11 Linux-only
  (`proc-macro-error` RUSTSEC-2024-0370 + gtk3-rs RUSTSEC-2024-0411–0420). `glib` RUSTSEC-2024-0429 is
  *unsound*-class — not enforced by cargo-deny v2; surfaced separately by `osv-scanner` (CVSS 6.9). RustSec
  inventory total = **17** (12 Linux-only deferred + 5 Apple-present accepted), cross-checked via
  `osv-scanner`. In `deny.toml` ACCEPTED/DEFERRED blocks + `ops-review.md` §4.
- [x] T5 Wire a CI step (`.github/workflows/`) running the **scoped** `cargo deny check advisories`; document
  the local one-liner for contributors (RELEASE.md / contributing notes).
  → **DONE.** `.github/workflows/dependency-advisories.yml`; local one-liner in `RELEASE.md` v0.6.3.

## Verification gate

- [x] V1 Scoped `cargo deny check advisories` exits 0 on the Apple targets.
  → **PASS — verified with the real pinned binary** (`cargo-deny 0.19.9`): `advisories ok`, exit 0. CI
  re-runs the same pinned version on every PR/push. The Apple graph is *not* empty of advisories — the 5
  `unic-*` are present and are accepted via the documented `ignore`; gtk3-rs/glib stay target-scoped out.
- [x] V2 `git diff main -- src-tauri/Cargo.lock src-tauri/Cargo.toml` is **empty** (config/docs only).
  → **PASS** — verified 0 bytes.
- [x] V3 Tripwire proven: adding `x86_64-unknown-linux-gnu` to the evaluated targets makes the scan
  **re-report** the gtk3-rs/proc-macro-error cluster.
  → **PASS — real binary.** A throwaway full-target config (Apple + `x86_64-unknown-linux-gnu`, `ignore=[]`,
  `cargo-deny 0.19.9`) errors on the 11 unmaintained Linux IDs (`proc-macro-error` RUSTSEC-2024-0370 +
  gtk3-rs RUSTSEC-2024-0411–0420) on top of the 5 Apple `unic-*`. `glib` RUSTSEC-2024-0429 is *unsound*
  (not enforced by cargo-deny v2) — caught by `osv-scanner`. Also corroborated by `cargo tree`:
  `--target x86_64-unknown-linux-gnu` resolves `glib←atk←gtk←{muda,tao}←tauri`; `--target
  aarch64-apple-darwin` → "nothing to print". The temp config lives outside the repo; shipped `deny.toml`
  scoping is unchanged.
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
