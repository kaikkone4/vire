# Security Review — TASK-053 attach v0.8.1 DMG asset + release smoke checklist

**Tier:** L2 · **Scope:** release distribution/docs + public DMG asset on GitHub Release `v0.8.1` (README.md, RELEASE.md, change dir only) · **Verdict:** PASS

## Scanner results summary (Tier 1)

| Scanner | Version | Surface | Result |
|---|---|---|---|
| **gitleaks** | 8.30.1 | branch commit range `main..HEAD` (the docs commit, ~21 KB) | **Clean** — 1 commit scanned, no leaks found |
| **semgrep** | 1.167.0 | changed docs `README.md`, `RELEASE.md` (`--severity ERROR`) | **Clean** — no ERROR-severity findings |
| **OSV-scanner** | 2.4.0 | `package-lock.json` (106 pkgs) + `src-tauri/Cargo.lock` (529 pkgs) | 19 advisories in the **inherited v0.8.1 baseline** (no lockfile touched by this task); **no auto-fail on the shipped artifact** — see triage below |
| **Trivy** | 0.71.2 | fs secret+misconfig on changed docs; container-image scan | **Clean** — no secrets/misconfig. **No container image is built or shipped by this repo** (macOS Tauri desktop app; no Dockerfile) → image-vuln auto-fail condition **N/A** |

### OSV triage (auto-fail gate = CVE ≥ 7.0 CVSS)

TASK-053 touches **no** dependency/lockfile/manifest (`git diff` on all `package*.json`, `Cargo.*`, Dockerfile, `*.yml` → none). The 19 advisories are the frozen v0.8.1 dependency baseline carried from TASK-052, not a delta of this change. Two carry a CVSS score; both were checked for reachability in the shipped macOS `aarch64` binary:

- **RUSTSEC-2026-0185 — `quinn-proto` 0.11.14 — CVSS 7.5 (High).** **Not compiled into the shipped binary.** Pulled only via `reqwest`'s optional `http3`/quinn feature; the app declares `reqwest = { default-features = false, features = ["blocking","json","rustls-tls"] }` (http3 off). `cargo tree -i quinn-proto --target aarch64-apple-darwin --edges normal` → *nothing to print* (absent from the activated host graph). Lockfile-only, unreachable → **advisory, not auto-fail.**
- **RUSTSEC-2024-0429 — `glib` 0.18.5 — CVSS 6.9.** Below the 7.0 threshold, and part of the Linux GTK stack (not in the macOS-host activated graph; macOS uses WKWebView/wry). → **advisory.**
- **GTK-binding advisories** (`atk`, `gdk`, `gdkx11`, `gdkwayland-sys`, `gtk`, `gtk-sys`, `gtk3-macros`, `proc-macro-error`, `unic-*` — RUSTSEC-2024/2025) are **unmaintained/informational (no CVSS)**, Linux-only bindings not compiled on macOS, previously triaged under TASK-047 (tauri-gtk-rustsec-cleanup). → **advisory.**

**No CVSS ≥ 7.0 finding is reachable in the artifact TASK-053 distributes → no auto-fail condition hit.**

## Manual review findings

- **Secrets** — none introduced. gitleaks + Trivy secret scan clean; the docs contain only a public download URL and a published sha256 checksum (non-sensitive). The DMG embeds no secrets — Langfuse keys are read from the user's local `.env` at runtime, not baked into the bundle.
- **Auth / authorization** — no auth code touched (excluded-path diff for `src-tauri`, `tauri.conf.json`, `.github` is empty). No trust-boundary, capability, or IPC change; no auth-bypass surface.
- **Updater / signing / notarization claims — accurate & safe.** RELEASE.md states "still **no** auto-updater, signing/notarization, or release-feed automation"; README/RELEASE both correctly describe the build as **not code-signed or notarized**. No false claim that signing/notarization was performed. The smoke-checklist "Check for updates" note correctly scopes it to reading `tag_name` only (asset-independent).
- **CI / release-feed** — no `.github/` or workflow change; no automation added.
- **Checksum documentation accurate** — RELEASE.md records `sha256 e77d15cf2066a24ee344ea5ab65787c1551400799b766e8261e1e02e1e82e27f`, size `5,787,708 B`, embedded `CFBundleShortVersionString 0.8.1`, matching QA's verified three-way evidence (local build ↔ RELEASE.md ↔ live GitHub asset digest).
- **Unsigned Gatekeeper guidance is safe.** Both README step 3 and RELEASE step 3 instruct **right-click → Open** (or *System Settings → Privacy & Security → Open Anyway*) and explicitly say **"Do not disable Gatekeeper or strip quarantine."** No `xattr -d com.apple.quarantine`, no `spctl --master-disable`, no `sudo` quarantine-bypass anywhere in the changed docs. (The lone `spctl`/quarantine mention, in `arch-review.md`, is a guard requiring guidance *not* bypass Gatekeeper.)
- **Public asset exposure is expected and approved.** Publishing the v0.8.1 DMG on the public GitHub Release is the declared purpose of this change (proposal/spec). The asset is the already-built, QA-verified release binary; no over-exposure of internal artifacts.

## Escalations

None design-level. **Advisory (non-blocking) follow-up:** the inherited dependency baseline carries `quinn-proto` 0.11.14 (7.5, unreachable/optional) and `glib` 0.18.5 (6.9, Linux-only). Recommend a future dependency-bump task (analogous to TASK-047) to lift `quinn-proto` → 0.11.15 and refresh the GTK/`glib` stack — **not** in TASK-053's scope (a bump would change `Cargo.lock` + require rebuild/re-release, violating this task's no-source/no-lockfile boundary).

## Checks run

- `gitleaks detect --log-opts="main..HEAD"` — clean.
- `semgrep --severity ERROR README.md RELEASE.md` — clean.
- `osv-scanner scan source --lockfile=package-lock.json --lockfile=src-tauri/Cargo.lock` — 19 baseline advisories triaged (see above).
- `cargo tree -i quinn-proto/glib --target aarch64-apple-darwin --edges normal` — both absent from the shipped-binary activated graph.
- `trivy fs --scanners secret,misconfig README.md` / `RELEASE.md` — clean; no Dockerfile/image in repo.
- `git diff --name-only main...HEAD` incl. dep/lock/container/CI grep — no source/manifest/CI surface touched.
- Manual grep of changed docs for quarantine/Gatekeeper-bypass patterns and false signing claims — none present; safe phrasing confirmed.

## Verdict

**PASS.** No auto-fail condition hit per the L2 triage rubric (no secret, no reachable CVE ≥ 7.0, no HIGH/CRITICAL image finding — no image exists, no semgrep ERROR). Docs are accurate and the Gatekeeper guidance is safe. Wait for SW-4 (code review — already PASS) then route to SW-6 (release manager) for the outstanding human-Mac UAT (smoke steps 2–5).
