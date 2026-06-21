<!-- handoff.md — compact per-task state. KEEP <= 2 KB. Reference files; never paste content. -->

# Handoff — TASK-047 Tauri/GTK RustSec cleanup (TASK-043 Stream B)

- **Change**: `openspec/changes/task-047-tauri-gtk-rustsec-cleanup/`. Branch:
  `feat/task-047-tauri-gtk-rustsec-cleanup`. Draft PR open (link in SW-2 report).
- **Phase**: SW-1 arch (PASS) → **SW-2 DevOps COMPLETE**. Next: **SW-5 sec.md**, then SW-3 QA.
- **Decision**: (c) deferred w/ risk acceptance via **target-scoped `cargo-deny`** gate. Zero
  shipped-crate delta; no `Cargo.*`/source/capability/`tauri.conf.json` edit.

## SW-2 delivered

- `src-tauri/deny.toml` — `[graph].targets`=Apple only; `ignore=[]` (no blanket suppress);
  `unmaintained="all"`; auditable 12-ID deferred block.
- `.github/workflows/dependency-advisories.yml` — `cargo deny check advisories` (ubuntu, graph-only
  scoped). Local: `cd src-tauri && cargo deny check advisories`.
- `RELEASE.md` v0.6.3 (SW-6 finalizes version) · `ops-review.md` (full evidence + 12-ID table).

## Gate results

- **V2** empty Cargo diff: PASS (0 bytes vs main). **V3** tripwire: PASS via `cargo tree` (Linux triple
  resurfaces `glib`+cluster; macOS="nothing to print"). **V4**: N/A (no compiled change).
- **V1** scoped check exits 0 + **V5** sec.md: deferred to CI / SW-5.

## Tooling note

`cargo-deny`/`cargo-audit` NOT installed; policy=no install → real scan runs in CI. Deferred IDs
extracted programmatically from RustSec advisory-db vs exact `Cargo.lock` versions (not hand-copied).

## Deferred IDs — see `ops-review.md` §4 / `deny.toml`

`glib` 0.18.5→RUSTSEC-2024-0429 (unsound, fix ≥0.20); `proc-macro-error` 1.0.4→-0370; gtk3-rs unmaint
cluster -**0411–0420** (10 crates). NB design's "…0423" approximate; exact=…0420.

## Next

- SW-5: write `sec.md` from `ops-review.md` §4. SW-3: re-enter only per tripwire (Linux target / Tauri
  off GTK3).

## Refs

`design.md` · `tasks.md` · `ops-review.md` · `specs/dependency-security/spec.md` · `src-tauri/deny.toml`.
