# Code Review — TASK-053 attach v0.8.1 DMG asset + release smoke checklist

**Verdict:** PASS

## Blocking issues

None.

## Suggestions

None.

## Escalations to SW Architect

None.

## Review notes

- Scope stayed release distribution/docs only. `git diff --name-status origin/main...HEAD` lists only `README.md`, `RELEASE.md`, and `openspec/changes/task-053-release-dmg-asset-smoke/*`; the excluded-path diff for `src-tauri`, `tauri.conf.json`, and `.github` is empty.
- README install guidance is clear and appropriately scoped: prebuilt install path appears before source build instructions, names `Vire_0.8.1_aarch64.dmg`, states Apple Silicon only, instructs dragging only `Vire.app`, documents unsigned Gatekeeper first launch without bypassing Gatekeeper, and says no dev server is required at runtime (`README.md:63`, `README.md:68`, `README.md:71`, `README.md:74`, `README.md:79`). The build artifact example is bumped to `Vire_0.8.1_aarch64.dmg` (`README.md:101`).
- RELEASE records the shipped artifact with name, size, sha256, embedded version, and download URL (`RELEASE.md:23`, `RELEASE.md:27`, `RELEASE.md:28`, `RELEASE.md:29`, `RELEASE.md:30`, `RELEASE.md:31`). The smoke checklist separates the verified asset/download step from human physical-Mac UAT steps (`RELEASE.md:35`, `RELEASE.md:37`, `RELEASE.md:38`, `RELEASE.md:39`, `RELEASE.md:40`, `RELEASE.md:41`, `RELEASE.md:43`).
- Checksum handling is consistent across docs and QA evidence: QA records the same `e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f` sha256 and 5,787,708-byte size for local build, RELEASE.md, and the live GitHub asset digest (`openspec/changes/task-053-release-dmg-asset-smoke/qa.md:11`, `RELEASE.md:28`, `RELEASE.md:29`).
- QA already verified the live release asset/download and PR file list (`openspec/changes/task-053-release-dmg-asset-smoke/qa.md:9`, `openspec/changes/task-053-release-dmg-asset-smoke/qa.md:18`, `openspec/changes/task-053-release-dmg-asset-smoke/qa.md:24`, `openspec/changes/task-053-release-dmg-asset-smoke/qa.md:26`). Reviewer-side live recheck was blocked by sandbox DNS/network (`gh release view`: could not connect to `api.github.com`; `curl`: could not resolve `github.com`), so the review relies on QA's recorded live evidence for the mutable GitHub asset state.

## Checks run

- `git diff --name-status origin/main...HEAD`
- `git diff --name-status origin/main...HEAD -- src-tauri tauri.conf.json .github`
- `git diff --check origin/main...HEAD`
- `sed` / `nl` review of required README, RELEASE, QA, tasks, and handoff sections
- `test -s` for required files; `tail -c1 | od -An -t x1` for trailing newline checks
- Attempted live release recheck: `gh release view v0.8.1 --repo kaikkone4/vire --json assets,tagName,isDraft,isPrerelease` and `curl --max-time 15 -sS -o /dev/null -w '%{http_code} %{url_effective}\n' -L <download-url>`; both failed due sandbox network/DNS, not product behavior.
