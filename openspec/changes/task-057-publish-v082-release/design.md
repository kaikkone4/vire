# Design — TASK-057 publish v0.8.2 release

## Goal

Publish a discoverable `v0.8.2` GitHub Release (with a real installable macOS aarch64 DMG built from
`main` incl. TASK-056) so an existing `0.8.1` build reports **UpdateAvailable**, without touching any
update-check logic and without building an in-app auto-updater.

## Current state (verified locally)

- `main` metadata triple = `0.8.1` (`src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `Cargo.lock`
  `vire` entry all read `0.8.1`).
- TASK-056 merged to `main` at `df79423` (PR #43) — but the **local `origin/main` ref is stale**
  (its recorded head is the TASK-054 merge `71719b9`, PR #42). **Precondition:** SW-2/SW-6 MUST
  `git fetch origin` and branch/build off the *actual* current `main` that contains `df79423`, or the
  DMG will omit TASK-056. This is a required step, not a blocker.
- Latest Release = `v0.8.1`, full, with asset `Vire_0.8.1_aarch64.dmg`
  (`sha256 e77d15cf…`, 5,787,708 B) and TASK-054 unsigned-launch notes.
- `update_check/mod.rs`: current version = `env!("CARGO_PKG_VERSION")`; `compare_release` strips a
  leading `v` from `tag_name`, parses both with `semver`, returns `UpdateAvailable` iff
  `latest > current`, else `UpToDate` (equal or lower → up-to-date, never downgrade). **Unchanged.**

## Version decision — 0.8.1 → 0.8.2 (patch)

Bump instructed by the task. Note (non-blocking): TASK-056 added a user-facing feature (Settings UI),
which by strict SemVer would be minor (→0.9.0), and TASK-056's own RELEASE.md suggested folding into a
future `0.9.0`. A **patch** bump is nonetheless a deliberate, valid pre-1.0 choice here: the release's
*purpose* is to make the update check detect *a* newer version, and any increment `> 0.8.1` satisfies
`latest > current`. Pre-1.0 (`0.x`) SemVer does not reserve minor for features. The design proceeds
with `0.8.2` as directed; if the product prefers `0.9.0` semantics, only the three version strings and
the tag/notes change — no design impact. Flagged in the handoff for Janne's awareness.

## Version triple + docs (Part A)

Change exactly these, keep them in agreement:

| File | From | To |
|------|------|----|
| `src-tauri/Cargo.toml` `version` | `0.8.1` | `0.8.2` |
| `src-tauri/tauri.conf.json` `version` | `0.8.1` | `0.8.2` |
| `src-tauri/Cargo.lock` `vire` entry | `0.8.1` | `0.8.2` (via `cargo build`, commit only that line) |
| `RELEASE.md` | — | prepend `## v0.8.2` entry (TASK-056 + this publication) |
| `README.md` | `Current version: v0.8.0`; `Vire_0.8.0_aarch64.dmg` example | `v0.8.2`; `Vire_0.8.2_aarch64.dmg` |

No change under `src-tauri/src/update_check/`. No new dependency; the only lockfile line touched is
the `vire` package version.

## DMG build + publish (Part B, after merge)

1. `git fetch origin && git checkout main` (must include `df79423`); confirm
   `git show HEAD:src-tauri/Cargo.toml` reads `0.8.2`.
2. `npm run tauri:build` → `src-tauri/target/release/bundle/dmg/Vire_0.8.2_aarch64.dmg` (aarch64;
   x86_64 not built — matches the `v0.8.1` single-arch precedent).
3. `gh release create v0.8.2 --repo kaikkone4/vire --target <merged-sha> --title "Vire v0.8.2"
   --notes-file <notes> src-tauri/target/release/bundle/dmg/Vire_0.8.2_aarch64.dmg` — **full
   release**, no `--draft`/`--prerelease` (either would keep `/releases/latest` on `v0.8.1`). Tag is
   created server-side over the authenticated `gh` session → avoids the local SSH-key passphrase
   blocker that deferred every prior signed task-tag.
4. Notes MUST contain: asset name, "unsigned/unnotarized", the exact
   `xattr -dr com.apple.quarantine /Applications/Vire.app` command, official-source trust caveat, and
   the TASK-028 Developer-ID/notarization long-term pointer. MUST NOT contain `sudo`,
   `spctl --master-disable`, or any global-Gatekeeper-disable guidance.

## xattr — why Plan B stays user-run (answers Janne's question)

Do **not** embed a quarantine bypass in the DMG/installer. Architecturally impossible-and-wrong:

- **The quarantine attribute is not in the artifact.** `com.apple.quarantine` is an extended
  attribute stamped by Gatekeeper onto the file **on the user's Mac at download time**, keyed to the
  downloading app. It does not exist inside the DMG we upload, so there is nothing in the artifact to
  pre-clear. `xattr -dr …` must run on the user's filesystem, after download, against the installed
  `/Applications/Vire.app`.
- **A postinstall "bypass" is circular and unsafe.** A plain drag-install DMG runs no install script;
  and any script that ran `xattr`/`spctl` would itself be blocked by the same Gatekeeper it tries to
  defeat, and would teach users to globally weaken Gatekeeper — a security anti-pattern we reject.
- **The only artifact-side removal of the user step is Developer ID + notarization.** A validly
  signed, Apple-notarized, ticket-stapled app launches without any user xattr action. That requires a
  paid Apple Developer ID certificate and a notarization pipeline — tracked as **TASK-028** (BA
  decision FB-054-1 still open). Until that exists, Plan B remains the documented, user-run, per-app
  remedy — exactly the posture Janne validated on a physical Mac in TASK-054.

## Verification (proportional)

- Part A: `cargo build` OK + `Cargo.lock` `vire = 0.8.2`; version triple all `0.8.2`;
  `cargo test update_check` green (logic unchanged); `npm run test:frontend` for
  `updateCheckUi.test.mjs` green; `npm run build` + `cargo fmt --all -- --check` clean;
  `openspec validate task-057-publish-v082-release --strict` passes.
- Part B live: `gh release view v0.8.2 --json tagName,isDraft,isPrerelease,assets` →
  `v0.8.2`, `isDraft=false`, `isPrerelease=false`, DMG asset present; `/releases/latest` → `200`
  with `tag_name=v0.8.2`; **0.8.1 build → UpdateAvailable (0.8.1→0.8.2, real url)**; **0.8.2 build →
  UpToDate**; releases page `200`. Physical-Mac DMG smoke (mount, drag, xattr, launch) is standing
  human UAT (carried from TASK-053/054), not a merge blocker.
