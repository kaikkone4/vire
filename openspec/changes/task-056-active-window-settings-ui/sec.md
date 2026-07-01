# Security review — TASK-056 active-window capture Settings + privacy/status UI (SW-5)

- **Branch / PR**: feat/task-056-active-window-settings-ui · draft PR #43 (open, base `main`)
- **Tier**: L2 — Secrets + CVE≥7 + Trivy + semgrep ERROR
- **Controls in scope**: SEC-001 (field-allowlist), SEC-007 (zero-grant baseline), SEC-012 (no-secrets)
- **Scope reviewed**: 8 source files — new IPC for capture settings/status (`get_/set_active_window_capture_settings`) + Settings/privacy/status UI. `git diff main...HEAD` = exactly the 16 TASK-056 files; native capture path (`capture.rs`/`config.rs`) untouched.

## Verdict: PASS

No auto-fail condition attributable to this change. All Tier-L2 scanners clean on the changed
surface; the two new IPC commands read/write only the five pre-existing capture `settings` keys with
parameterized SQL, validate before persisting, and expose no prohibited field. The one dependency
CVE≥7 that OSV reports is **pre-existing baseline** (lockfiles byte-identical to `main`; this change
adds zero dependencies) and is not introduced or reached by TASK-056 — recorded below as a
carry-forward advisory, not a blocker for this change.

## Scanner results (Tier L2)

| Scanner | Scope | Result |
| --- | --- | --- |
| **gitleaks** 8.30.1 | commit range `main..HEAD` (4 commits, ~106 KB) | **no leaks** |
| **semgrep** 1.167.0 | 6 changed source files (rust + typescript + xss + command-injection rulesets, 112 rules, ERROR severity) | **0 findings (0 blocking)** |
| **OSV-scanner** 2.4.0 | `src-tauri/Cargo.lock` (529 pkgs) + `package-lock.json` (106 pkgs) | 19 pre-existing advisories (1 HIGH, 1 Medium) — **none introduced by TASK-056** (see Advisories) |
| **Trivy** 0.71.2 | `secret` + `misconfig` over `src-tauri/src`, `src` | **0 secrets, 0 misconfig**. Container-image role N/A — no container image / Dockerfile in this Tauri desktop app |

## Manual review — SEC controls verified

**SEC-001 field-allowlist — no prohibited captured fields.**
- New IPC input DTO (`CaptureSettingsInput`) is a serde struct with exactly five keys; serde drops any
  non-allowlisted key, so `title_mode` (or any prohibited field) cannot be deserialized (`settings_api.rs`).
- Status projection (`CaptureStatusView`/`HealthMarker`, `store::capture_status_snapshot`) reads only
  allowlisted columns via `MAX`/`COUNT`/`state`/`start_ts`/`detail` — counts, timestamps, and bounded
  coarse state/detail codes. No title, path, URL, command, keystroke, screenshot, or clipboard value
  is selected or projected (structurally absent upstream).
- Privacy table (`privacyTable`) truthfully lists Captured = bundle id + display name, coarse idle
  state, timestamp; Never captured = window titles, Accessibility tree, screen pixels, keystrokes,
  mouse/clipboard, URLs/file paths/terminal command bodies, prompts/responses, secrets.

**SEC-007 zero-grant baseline — no new permission surface.**
- No new IPC beyond the two settings commands; both operate over the existing `settings` table and
  `active_window_*` tables only. No new native/TCC/Accessibility/window-title/screenshot/URL/keystroke
  code (repo grep over the diff: no `kAX`/`AXUIElement`/`CGWindow`/`CGEvent`/event-tap/screen-record).
- No `title_mode` input control anywhere in the UI; fixed `title_mode = "redacted"` is informational,
  never accepted, never written (backend guardrail test `settings_api_set_never_writes_title_mode`).
- Capture stays **OFF by default**; only an explicit user toggle enables it. Non-macOS disables all controls.
- No `tauri.conf.json` / capabilities / entitlements / CSP change; no new dependency; no network,
  release, or migration change (confirmed empty in `git diff main...HEAD --name-only`).

**SEC-012 no-secrets.**
- gitleaks + Trivy secret scan clean; no credential handling in this change. Secret material is
  structurally absent from the capture surface. Langfuse secret-key handling (Keychain, never echoed)
  is unchanged and out of scope for this diff.

## Auth / injection / validation / transparency

- **Auth bypass**: N/A — local single-user Tauri desktop app, no auth layer, no server endpoints,
  no rate-limit surface. The IPC commands expose no data beyond the local capture config/status.
- **SQL injection**: all writes/reads use `rusqlite` parameter binding (`params![...]`, `?1`/`?2`);
  the upsert is a fixed `ON CONFLICT` statement with bound key/value — no string interpolation.
- **Input validation prevents unsafe values**: `settings_api::validate` enforces safe bounds
  (sample 1–3600 s, idle-candidate 1–86400 s, away > candidate and ≤86400 s, retention 1–3650 days),
  **rejects** rather than silently clamping, and runs **before** the transaction opens — a rejected
  write touches no row and the prior config stays in effect. Frontend `validateCaptureInput` mirrors
  the same bounds (incl. explicit NaN/non-integer rejection) as pre-flight UX; the backend re-validates
  authoritatively.
- **XSS**: every dynamic value rendered by `active-window-settings-ui.ts` (status timestamps, coarse
  health state/detail codes, `title_mode`, numeric field values) passes through `escapeHtml`; even the
  structurally-bounded codes are escaped defensively. main.ts wiring surfaces backend errors verbatim
  via `run()`/`alertError` and uses `CSS.escape` for selectors.
- **Truthful privacy copy**: the false Settings copy ("does not collect … active windows, idle state")
  is removed; banner/panel/sidebar are all driven by the real `get_active_window_capture_settings`
  view (off / on / macOS-only / unavailable), so no view asserts a capture denial while capture is on.

## Advisories (carry-forward — NOT blocking this change)

OSV reports 19 pre-existing dependency advisories in the baseline lockfiles. TASK-056 changes no
lockfile (`Cargo.lock`/`Cargo.toml`/`package-lock.json`/`package.json` all byte-identical to `main`),
so **none are introduced by this change**; they belong to dependency-hygiene work (the
`task-047-tauri-gtk-rustsec-cleanup` lineage), not this Settings-UI gate.

- **RUSTSEC-2026-0185 `quinn-proto` 0.11.14 — CVSS 7.5 HIGH** (fix 0.11.15). Transitive via
  `reqwest → quinn` (QUIC/HTTP3 path). Not referenced anywhere in app source; the app's only network
  is Langfuse HTTP + the GitHub update check — the QUIC path is not exercised. Pre-existing on `main`.
- **RUSTSEC-2024-0429 `glib` 0.18.5 — CVSS 6.9** (fix 0.20.0) and the GTK cluster
  (atk/gdk/gtk/gdkx11/… RUSTSEC-2024-04xx) are Linux GTK3 bindings / unmaintained-crate notices,
  not compiled or used on this macOS-only desktop app. Below the CVE≥7 line and pre-existing.

**Recommendation**: address the pre-existing `quinn-proto` 7.5 (and GTK/glib cluster) in the
dedicated dependency-hygiene task, independent of TASK-056.

## Escalations

None. No design-level security issue (trust boundary, missing auth layer). No `feedback_to_ba[]`.

## Gate routing

PASS → proceed. Wait for SW-4 (Code Reviewer); when both SW-4 and SW-5 pass, hand to SW-6
(Release Manager). Physical-Mac smoke (`tasks.md` §Smoke) remains the mandated human/UAT route and
does not block this gate.
