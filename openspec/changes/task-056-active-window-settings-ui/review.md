# Code Review — TASK-056 active-window capture Settings + privacy/status UI

## Verdict: PASS

No blocking craft/convention issues found. The PR scope is limited to the expected TASK-056 docs, backend
settings/status IPC, frontend Settings/sidebar/banner builders, and tests. I found no title-mode input,
no native Accessibility/window-title capture change, no schema/dependency/CSP change, and no stale
contradictory product copy in the changed runtime UI.

## Blocking Issues

None.

## Suggestions

- `openspec/changes/task-056-active-window-settings-ui/tasks.md:8` and
  `openspec/changes/task-056-active-window-settings-ui/tasks.md:27` still leave backend workstreams A/B
  unchecked even though the backend IPC/status implementation and tests are present. Update the checklist
  before release so the change artifact does not contradict the shipped code.
- `src/active-window-settings-ui.ts:35` says the backend "rejects" non-allowlisted keys, but the Rust DTO
  uses serde's default unknown-field behavior, which drops/ignores extra fields. The implementation is fine
  for `title_mode` non-persistence; the comment should say "ignores/drops" to match reality.

## Changed Paths / Artifacts Reviewed

- `docs/active-window-capture.md`
- `openspec/changes/task-056-active-window-settings-ui/{arch-review.md,design.md,handoff.md,proposal.md,qa.md,specs/active-window-settings/spec.md,tasks.md}`
- `src-tauri/src/active_window/{mod.rs,model.rs,settings_api.rs,store.rs,tests.rs}`
- `src-tauri/src/lib.rs`
- `src/active-window-settings-ui.ts`
- `src/main.ts`
- `tests/activeWindowSettingsUi.test.mjs`

## Checks

- `cargo fmt --check --manifest-path src-tauri/Cargo.toml` — pass
- `cargo test --lib active_window --manifest-path src-tauri/Cargo.toml` — 66 passed
- `npm run build` — pass
- `node --import tsx --test tests/activeWindowSettingsUi.test.mjs` — 31 passed
- `openspec validate task-056-active-window-settings-ui --strict` — pass
- `npm run test:frontend` — 144/148 passed; 4 failures are in `tests/pi-observe.security.test.mjs`
  from sandbox-denied `127.0.0.1` listen attempts, outside the TASK-056 diff.

## Branch / Scope Notes

- `git diff main...HEAD --name-only` contains the expected 16 TASK-056 paths.
- No diff in `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `package.json`, `package-lock.json`,
  `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, or
  `src-tauri/src/active_window/capture.rs`.
- Working tree has pre-existing untracked OpenSpec artifacts from other tasks; no tracked non-review file
  modifications were produced by this review.
