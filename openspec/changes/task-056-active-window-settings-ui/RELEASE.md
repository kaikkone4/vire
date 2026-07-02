# Release — TASK-056 · active-window capture Settings panel + truthful status/privacy copy

- **Task:** TASK-056
- **Tier:** L2 (SEC-001/007/012; new IPC surface + transparency copy)
- **Branch:** `feat/task-056-active-window-settings-ui`
- **PR:** #43 — https://github.com/kaikkone4/vire/pull/43
- **Tag (intended):** `task-056/v0.8.1` — **not created this gate**, see §Tag status below
- **Head commit:** `97ee6eb`
- **Base:** `main` @ `6eaffc8` (branch diff clean vs current `main`; `origin/main` has since advanced via
  unrelated task-053/054 merges — PR #43 reports `mergeable: MERGEABLE` against current base)
- **App version carried:** `0.8.1` (unchanged — this task ships no version-file bump; see Declaration 1)
- **Gates passed:** SW-3 QA **PASS** (`qa.md`), SW-4 Code Review **PASS** (`review.md`), SW-5 Security
  **PASS** (`sec.md`), Documentation gate **PASS** (`docs.md`) → this gate (SW-6)

---

## Summary

Adds an in-app **Settings → Active-window capture** panel and truthful status copy for the
zero-permission active-app/idle capture loop shipped (backend-only, no UI) in TASK-048/v0.8.0. Two new
Tauri IPC commands (`get_active_window_capture_settings`, `set_active_window_capture_settings`) expose
the five existing `active_window_*` settings keys plus a read-only capture-status/health projection; a
new frontend module (`src/active-window-settings-ui.ts`) renders the panel, a Captured-vs-Never-captured
privacy table, and drives the sidebar/Today banner from the real setting instead of static copy. No
schema change, no new dependency, no `tauri.conf.json`/capability/CSP change, no native
Accessibility/window-title/screen-capture code touched. Capture remains **OFF by default**; `title_mode`
stays fixed at `"redacted"`, read-only, never accepted as input.

This session also committed doc fixes left over from the Documentation gate (`README.md`,
`docs/active-window-capture.md`, `tasks.md` checkbox correction) and the SW-3/4/5 gate artifacts
(`qa.md`, `review.md`, `sec.md`, `docs.md`) that were written but not yet committed — see commit
`97ee6eb`. PR #43 diff re-verified clean: exactly the 16 original TASK-056 paths plus these 4 doc/gate
files (20 total), no drift from other in-flight tasks' work sitting untracked in the workspace.

---

## Gate results

| Gate | Verdict | Evidence |
|------|---------|----------|
| SW-3 QA | **PASS** | `qa.md` — full scenario matrix (`specs/active-window-settings/spec.md`) covered; backend 259/259 + `active_window` 66/66, `cargo fmt` clean, `npm run build` clean, frontend 146/148 (2 pre-existing unrelated failures outside diff), `openspec validate --strict` passes |
| SW-4 Code Review | **PASS** | `review.md` — no blocking issues; suggestions only (stale tasks.md checkboxes — since fixed; one comment wording nit) |
| SW-5 Security | **PASS** | `sec.md` — gitleaks 0 leaks; semgrep 112 rules / 0 findings; Trivy 0 secrets/misconfig; OSV 19 pre-existing advisories, none introduced (zero lockfile diff) |
| Documentation | **PASS** | `docs.md` — README.md/docs/active-window-capture.md verified accurate against shipped code, 3 stale spots fixed |

---

## Declaration 1 — Deployment size: **MINOR**

New, backward-compatible, additive user-facing capability: a Settings UI surface and two new IPC
commands for a capture feature that previously had no in-app controls (env-var only). No breaking
change to any existing IPC contract, settings key, or on-disk schema; no data migration. By SemVer this
is a minor-tier change (new feature, no break). Consistent with how the original capture loop itself
(TASK-048) was classified minor (`0.7.x → 0.8.0`) when it first shipped.

**No version-file bump lands in this PR** — `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and
`package.json` are untouched (confirmed in `review.md` §Branch/Scope Notes and by `git diff
main...HEAD`), following this project's established pattern of separating feature work from the
dedicated version-bump-and-publish task (precedent: TASK-048 merged source, TASK-049/052 bumped and
published the version). The app continues shipping as `0.8.1` until a future version-bump task rolls
this and any other pending merges into a new published release.

---

## Declaration 2 — Rollback strategy: **partial-automated**

Source-only change, no schema/migration/dependency/capability delta, so revert is deterministic but
requires a PR review cycle on `main` (hence partial-automated, not fully automated):

```bash
git revert -m 1 <merge-commit-sha>   # after PR #43 merges
```

Open the revert as a new PR, get it reviewed, merge. Verify: `get_active_window_capture_settings` /
`set_active_window_capture_settings` no longer registered in `generate_handler!`
(`src-tauri/src/lib.rs`), Settings panel no longer renders, and `cargo test --lib active_window` /
`node --import tsx --test tests/activeWindowSettingsUi.test.mjs` both return to their pre-TASK-056
baseline counts. Because capture defaults OFF and no settings-table row shape changed, no data
cleanup or backfill is needed on rollback — existing `active_window_capture_enabled` etc. rows are
simply no longer reachable from the UI (still readable via the env var toggle, unaffected).

**L2 note:** per role policy, this rollback path should be exercised in a staging/local rebuild before
any future production (packaged/notarized) release that bundles this change — not yet done in this
gate (no packaged build exists yet for `0.8.1`+TASK-056; see Declaration 1).

---

## Declaration 3 — Component compatibility matrix

No dependency, toolchain, or platform-minimum change from the `0.8.1` baseline (`task-052`). This
change is IPC/UI-only against existing infrastructure.

### Runtime and toolchain (unchanged from `0.8.1` baseline)

| Component | Min version | Tested on | Notes |
|-----------|-------------|-----------|-------|
| Vire | 0.8.1 (this task's commits land on top, unreleased as a new number) | 0.8.1 | see Declaration 1 |
| macOS | 10.13 (High Sierra) | — (no physical-Mac smoke run this session; carried forward, see Notes) | Tauri 2.x platform minimum; macOS-only binary |
| Rust toolchain | 1.56 (edition 2021) | per CI/session toolchain | edition 2021 minimum |
| Cargo | ≥1.56 | matches Rust toolchain | unchanged |

### Tauri framework (Cargo) — unchanged, zero lockfile diff

| Crate | Spec version | Notes |
|-------|-------------|-------|
| `tauri` | `2.2` | no feature flags; no new IPC plugin |
| `tauri-build` | `2.0` | build-only |
| `tauri-plugin-dialog` | `2.2` | unchanged, unrelated to this diff |
| `tauri-plugin-opener` | `2` | unchanged, unrelated to this diff |

### Frontend / tooling — unchanged, zero lockfile diff

| Package | Spec version | Notes |
|---------|-------------|-------|
| `@tauri-apps/api` | `^2.2.0` | Tauri JS bridge — new commands invoked through existing `invoke()` |
| `@tauri-apps/plugin-dialog` | `^2.2.0` | unchanged |
| `@tauri-apps/cli` | `^2.2.0` | dev CLI, build-time only |
| TypeScript | `^5.7.2` | strict mode |
| Vite | `^6.4.3` | bundler |
| tsx | `^4.22.0` | test runner (`tests/activeWindowSettingsUi.test.mjs`) |

### Internal IPC contract (new in this change — min version)

| Command | Introduced | Notes |
|---------|-----------|-------|
| `get_active_window_capture_settings` | this change | frontend/backend shipped as one bundled artifact — no independent versioning/skew possible |
| `set_active_window_capture_settings` | this change | validates before persisting; rejects out-of-bounds rather than clamping |

### Upstream/downstream services

None. No network, schema, Langfuse, or Keychain surface touched by this change.

**Carry-forward advisory (not blocking, pre-existing baseline):** OSV reports `quinn-proto`
RUSTSEC-2026-0185 (CVSS 7.5, transitive via `reqwest`, unreached by app source) and the Linux
GTK/glib cluster (not compiled on macOS) — both pre-existing on `main`, zero lockfile diff introduced
by TASK-056. Belongs to the `task-047-tauri-gtk-rustsec-cleanup` dependency-hygiene lineage.

---

## Tag status: **not created this gate — passphrase required, deferred to Janne**

Attempted the required signed, task-scoped tag:

```bash
git tag -s task-056/v0.8.1 -m "release(task-056): v0.8.1 minor — active-window capture Settings panel + truthful status/privacy copy" HEAD
```

Failed: `error: Enter passphrase for "/Users/kaikkonen/.ssh/id_ed25519" ... incorrect passphrase
supplied to decrypt private key` / `error: unable to sign the tag`. Git is configured for SSH signing
(`gpg.format = ssh`, `user.signingkey = ~/.ssh/id_ed25519.pub`, `commit.gpgsign = true`); the key is
passphrase-protected and this non-interactive session has no way to supply it. Per this role's working
rule, an **unsigned tag is not created as a fallback** — confirmed no dangling/partial tag object was
left behind (`git tag -l "task-056*"` → empty).

This is **non-blocking**, consistent with precedent: `task-053/v0.8.1` and `task-054/v0.8.1` hit the
identical passphrase blocker and were deferred the same way (see those tasks' `RELEASE.md` §Tag
status); `task-045/v0.6.3` is the only tag in this repo with signing *attempted* historically, and
`git tag -v task-045/v0.6.3` confirms its signature also did not apply ("no signature found").

**Follow-up for Janne:** run the command above locally (with the SSH key passphrase available) against
commit `97ee6eb`, then `git push origin task-056/v0.8.1`.

---

## Changed paths (this gate's commit `97ee6eb`)

```
README.md                                                          (doc fix, committed this gate)
docs/active-window-capture.md                                      (doc fix, committed this gate)
openspec/changes/task-056-active-window-settings-ui/docs.md        (new, Documentation gate artifact)
openspec/changes/task-056-active-window-settings-ui/qa.md          (new, SW-3 gate artifact)
openspec/changes/task-056-active-window-settings-ui/review.md      (new, SW-4 gate artifact)
openspec/changes/task-056-active-window-settings-ui/sec.md         (new, SW-5 gate artifact)
openspec/changes/task-056-active-window-settings-ui/handoff.md     (updated)
openspec/changes/task-056-active-window-settings-ui/tasks.md       (A/B checkboxes corrected)
```

Full PR #43 diff vs `main` (20 files, unchanged from prior gates plus the above):
`docs/active-window-capture.md`, `README.md`, `src-tauri/src/active_window/{mod.rs,model.rs,
settings_api.rs,store.rs,tests.rs}`, `src-tauri/src/lib.rs`, `src/active-window-settings-ui.ts`,
`src/main.ts`, `tests/activeWindowSettingsUi.test.mjs`, and the `task-056-active-window-settings-ui`
change directory (`arch-review.md`, `design.md`, `docs.md`, `handoff.md`, `proposal.md`, `qa.md`,
`review.md`, `sec.md`, `specs/active-window-settings/spec.md`, `tasks.md`).

---

## Gate checklist

- [x] SW-3 QA PASS
- [x] SW-4 Code Review PASS
- [x] SW-5 Security PASS
- [x] Documentation gate PASS
- [x] RELEASE.md written with all three required declarations
- [ ] Tag `task-056/v0.8.1` — **deferred**, see §Tag status (non-blocking)
- [x] PR #43 promoted from draft to ready-for-review (this gate)

## Non-blocking follow-ups

1. Signed `task-056/v0.8.1` tag — create locally when Janne has the SSH key passphrase available
   (command above), then push.
2. Physical-Mac smoke (`tasks.md` §Smoke; `README.md` steps 26–30) — still not run in any session to
   date (no physical Mac available); carried forward as human/UAT, does not block this gate.
3. `quinn-proto` RUSTSEC-2026-0185 and the GTK/glib advisory cluster — pre-existing, address in the
   dedicated dependency-hygiene task (`task-047` lineage), independent of TASK-056.
4. A future version-bump-and-publish task (pattern: TASK-049/052) should roll this change (and any
   other pending merges) into the next published `0.9.0` GitHub Release once ready.
