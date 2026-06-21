<!-- handoff.md — compact per-task state. KEEP <= 2 KB. -->

# Handoff — TASK-047 Tauri/GTK RustSec cleanup

- **Branch/PR**: `feat/task-047-tauri-gtk-rustsec-cleanup` / #35.
- **Phase**: SW-1 PASS → SW-3 PASS → SW-4 PASS (recheck) → **SW-5 PASS (recheck)** — both gates green.
- **Decision**: deferred risk acceptance via Apple-target-scoped `cargo-deny`; no
  product/source/Cargo/UI/IPC/capability/config delta.

## SW-4 recheck — PASS

- Pin `cargo-deny@0.19.9` (workflow = `deny.toml`/`tasks.md`/`ops-review.md`). Real scan
  evidence in `ops-review.md` §3 (Apple exit 0; Apple+Linux `ignore=[]` → 5 `unic-*` + 11
  Linux unmaintained; `glib` 2024-0429 via osv). `ignore` = exactly the 5 `unic-*`;
  gtk3-rs/`glib`/`proc-macro-error` out of `ignore`, target-scoped. Wording = 17 (12
  deferred + 5 accepted). OpenSpec strict / YAML / `git diff --check` PASS; Cargo/product
  diff empty. cargo-deny not rerun (not installed); accepted real 0.19.9 evidence + QA.

## SW-5 recheck — PASS

- SEC-001 closed: 5 Apple-present `unic-*` in scoped `ignore` (per-ID rationale + ACCEPTED
  block); gtk3-rs/`glib`/`proc-macro-error` (2024-*) NOT in `ignore`, target-scoped
  (tripwire intact); cargo-deny pinned 0.19.9.
- L2 clean: gitleaks 0; semgrep ERROR 0; trivy HIGH/CRITICAL 0; osv 17 advisories, max
  `glib` 6.9 (<7.0) — no CVE≥7. Scope `Cargo.{toml,lock}`/`src/`/conf/capabilities = 0 B.
- Gate exits 0 (osv cross-check: only the 5 ignored `unic-*` fire on Apple). See `sec.md`.

## SW-6 DOCS — PASS (2026-06-21)

- Gap: README lacked advisory gate section; version showed v0.6.2.
- Fixed: `README.md` version bump → v0.6.3; added `## Dependency advisory gate` section
  (cargo-deny 0.19.9, Apple-target scope, 17-advisory posture table, tripwire warning,
  local/CI command, reference to deny.toml + RELEASE.md for full inventory).
- No code change; config/docs only. RELEASE.md + deny.toml content verified correct.
- Changed paths: `README.md`.

## Route

SW-4 + SW-5 + SW-6 DOCS PASS → **SW-6 Release Manager**; release once CI advisory gate is green.
