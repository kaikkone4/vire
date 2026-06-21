# Handoff — TASK-044 keychain-public-key-to-settings (F2a)

- **Change dir**: openspec/changes/task-044-keychain-public-key-to-settings/
- **Branch / PR**: feat/task-044-keychain-public-key-to-settings · PR (draft) — see `gh pr`
- **Phase / gate**: SW-2 BACKEND **complete** — ready for SW-3 QA
- **Tier**: L2

## Last gate result
SW-1 ARCH **PASS** (2026-06-21). SW-2 implementation done this pass: public key relocated from
Keychain → SQLite `settings`; secret stays Keychain-only. C1–C4 realized in code + tests.

## Active blockers
- None. Branch was cut from the task-043 SW-6 tip (task-043 not yet on `main`); the PR targets
  `main`, so the diff cleans up to TASK-044-only once task-043 merges.

## Exact next action
sw-qa-engineer (SW-3): integration/QA against `specs/langfuse-credential-storage/spec.md`.
**Manual T6 prompt-count**: fresh macOS launch + open Settings + Test connection ⇒ Keychain
prompts **once** (secret only). Expect the **one-time re-save** for pre-F2a installs (no
auto-migration — see Notes).

## Changed files (this gate)
- `src-tauri/src/settings/mod.rs` — `KEY_PUBLIC_KEY`; `read_setting_strict`/`clear_setting`;
  two-store atomic `set`/`clear` (C1/C4); `resolve_credentials(conn,…)` strict public read (no
  env downgrade on real failure); `get_langfuse_settings_repo` public presence from settings (no
  Keychain read). Legacy Keychain public best-effort deleted on set+clear.
- `src-tauri/src/lib.rs` — `set_langfuse_secret`/`clear_langfuse_secret` gain `State` + `db_conn`.
  JS arg shape unchanged (`{publicKey, secretKey}`) ⇒ no `src/main.ts` edit; no `generate_handler!`
  change.
- `src-tauri/src/settings/tests.rs` — call-site updates + T1/T2/T3/T4/T5 (5 new/rewritten).
- `openspec/changes/task-044-…/{tasks.md updated, handoff.md}`.

## Checks
- `cargo test` 169 pass (27 settings, 5 new). `cargo fmt --check` clean. `npm run build` green.
- `cargo clippy -D warnings`: **my files clean**; pre-existing pedantic-lint failures (clippy
  1.95.0, no toolchain pin) in untouched files — `langfuse/importer.rs`, `langfuse/tests.rs`,
  `lib.rs:1155/1713`. Out of scope; flagged for separate cleanup.

## Notes carried forward
- Public key = Basic-Auth username, **non-secret** (SEC sign-off GRANTED). Secret Keychain-only,
  presence-flag-only (SEC-009/C2). Pair atomic across both stores (DEC-026).
- **Existing installs re-save once** (M-c; no auto-migration — it would re-trigger the prompt).
  **SW-6 must add the RELEASE.md one-time-re-save note** (RELEASE.md is a SW-6 artifact here).
- **ID collision:** TASK-043 Stream B (Tauri/GTK advisory) renumbers later (recorded, not actioned).
