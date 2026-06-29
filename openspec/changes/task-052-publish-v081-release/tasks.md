# Tasks — TASK-052 publish v0.8.1 release

Single component (the **Vire desktop app**) plus a release-ops publication. **Two ordered parts
with a hard ownership boundary**: Part A is a normal code change (SW-2 → gates → merge); Part B is
the public GitHub Release, published by the **Release Manager (SW-6) only after Part A merges to
`main`**. Do **not** publish the Release before merge — see `arch-review.md`. No update-check logic
change, no `tauri-plugin-updater`, no installer build/sign/upload, no schema/CSP/capture change.
Generated files (`src-tauri/gen/schemas/capabilities.json`, `src-tauri/target/**`) are
gitignored — do **not** hand-edit or commit them.

## Part A — Land v0.8.1 metadata on `main` (SW-2)

Branch `feat/task-052-publish-v081-release` off `origin/main` (currently at `0.8.0`).

- [ ] `src-tauri/Cargo.toml` — set `version = "0.8.1"`.
- [ ] `src-tauri/tauri.conf.json` — set `"version": "0.8.1"`.
- [ ] `src-tauri/Cargo.lock` — run `cargo build` (or `cargo update -p vire --precise 0.8.1`) so the
      `vire` package `version` entry becomes `0.8.1`; commit the lockfile delta. Do **not** edit it
      by hand or let unrelated dependency lines drift.
- [ ] `RELEASE.md` (repo root) — prepend a `## v0.8.1` entry. State plainly: update-check is now
      usable (TASK-050 check + TASK-051 owner fix + this release publication); version metadata
      reconciled `0.8.0 → 0.8.1`; **no source/schema/CSP/dependency change** beyond the version
      strings; first published GitHub Release. Keep the existing `## v0.8.0` entry intact below.
- [ ] Confirm **no** change under `src-tauri/src/update_check/` — the comparison logic and guards
      are unchanged; this task only moves version strings and the changelog.

## Part A — Verify before merge (proportional to scope)

- [ ] `cargo build --manifest-path src-tauri/Cargo.toml` succeeds; `Cargo.lock` `vire` = `0.8.1`.
- [ ] `cargo test update_check` passes (existing 9 + the TASK-051 owner guards — unchanged).
- [ ] `node --import tsx --test tests/updateCheckUi.test.mjs` passes.
- [ ] `npm run build` passes; `cargo fmt --all -- --check` passes.
- [ ] Version triple agrees: `grep -E '^version' src-tauri/Cargo.toml`,
      `"version"` in `src-tauri/tauri.conf.json`, and the `vire` block in `src-tauri/Cargo.lock`
      all read `0.8.1`.
- [ ] `openspec validate task-052-publish-v081-release --strict` passes.
- [ ] Open PR → gates (SW-3 QA, SW-4 review, SW-5 security) → **merge to `origin/main`**.

## Part B — Publish the GitHub Release (SW-6, AFTER Part A is merged)

- [ ] Resolve the merged `main` commit SHA carrying `0.8.1` (`git rev-parse origin/main` after the
      merge; confirm `git show origin/main:src-tauri/Cargo.toml` reads `0.8.1`).
- [ ] Publish the Release, creating the tag server-side (avoids the local signing-passphrase
      blocker):
      `gh release create v0.8.1 --repo kaikkone4/vire --target <merged-sha> --title "Vire v0.8.1" --notes-file <notes>`.
      It **MUST** be a full release: do **not** pass `--draft` or `--prerelease` — `/releases/latest`
      skips both and would keep returning 404.
- [ ] Asset upload is **out of scope** — publish notes-only (or whatever notes file is prepared).
      Do **not** build/sign/upload installers in this task. Record the deferral in the handoff.

## Part B — Verify the live behaviour

- [ ] `gh release view v0.8.1 --repo kaikkone4/vire --json tagName,isDraft,isPrerelease` →
      `tagName=v0.8.1`, `isDraft=false`, `isPrerelease=false`.
- [ ] `curl -sS -o /dev/null -w "%{http_code}" https://api.github.com/repos/kaikkone4/vire/releases/latest`
      → `200`; the JSON `tag_name` equals `v0.8.1` and `html_url` is the real release page.
- [ ] `https://github.com/kaikkone4/vire/releases` → `200` ("Open GitHub Releases" reaches it).
- [ ] In-app smoke, **0.8.0 build** (the currently distributed binary): "Check for updates" →
      **UpdateAvailable** `{ current: "0.8.0", latest: "0.8.1", release_url: <real> }`.
- [ ] In-app smoke, **0.8.1 build** (rebuilt from merged `main`): "Check for updates" →
      **UpToDate** `{ current: "0.8.1" }` — never "downgrade available".
- [ ] "Open GitHub Releases" opens the real `v0.8.1` page in the OS default browser.
