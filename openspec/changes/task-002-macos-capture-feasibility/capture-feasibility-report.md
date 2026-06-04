# TASK-002 — macOS Capture Feasibility Report

- **Change:** `task-002-macos-capture-feasibility`
- **Branch:** `feat/task-002-macos-capture-feasibility`
- **Tier:** L2 · **Gate context:** APP-005 (SEC-001 primary, SEC-007; SEC-002/003 N/A — no network, no credentials)
- **Author role:** Integration / native-systems implementer (SW-2)
- **Date:** 2026-06-04
- **Target environment:** macOS 15.7.7 (build 24G716), Apple Swift 6.2.4, Command Line
  Tools SDK (macOS 15). Apple Silicon (arm64).
- **Kind:** Feasibility spike / assessment. **Ships nothing**; writes no product evidence
  rows; decides no implementation path. The Tauri+helper-vs-Swift-first decision is handed
  to TASK-003.

> Supporting probe: [`spikes/task-002-macos-capture/`](../../../spikes/task-002-macos-capture/)
> — a non-shipping, redaction-by-default Swift probe. It compiles clean against the
> macOS 15 SDK (`swiftc -typecheck` passes), which is how the API surface below was
> validated without sampling real activity.

---

## 1. Verdict

**Local macOS app/window evidence capture is feasible at the quality Janne's daily
reconstruction needs, at an acceptable and mostly low permission burden.** The primary
path is:

- **NSWorkspace / NSRunningApplication** for the frontmost-app signal — **no TCC
  permission required** (this corrects the UX prototype, see FB-002).
- **AXUIElement** for the focused-window title — requires the **Accessibility** TCC
  permission; degrades explicitly and safely without it.
- **CGEventSource** for the idle/away signal — **no permission required**.
- **Quartz Window Services** as an **optional** fallback only — its title benefit is
  gated behind the **Screen Recording** permission, which is the single intrusive
  permission in this space. **Recommendation: do not adopt Quartz for titles by
  default** (see §5).

Net required permission for full-quality capture is **one** user-facing grant
(Accessibility). The frontmost-app and idle signals work with zero grants, so even the
fully-denied state still yields a useful (degraded) timeline. This retires both
EPIC-001 capture risks ("title capture is inconsistent", "Quartz requires intrusive
permission") with documented, visible degraded states rather than silent gaps.

This report does **not** select Tauri+helper vs Swift-first; it produces the signal for
that decision (§10). No capture MVP is shipped (TASK-005 owns that).

---

## 2. Method and isolation

- **API validation, not live harvesting.** The probe was **compile-validated**
  (`swiftc -typecheck`) against the macOS 15 SDK to confirm every API used is available
  and correctly typed. The probe was **not executed against real activity and committed**:
  running it samples the actual frontmost app and window titles, which would risk
  persisting Janne's private titles. Feasibility is therefore established from (a) Apple's
  documented API semantics for the target macOS version, (b) the compile-validated probe,
  and (c) the manual validation matrix (§9) that Janne / QA run locally on a real session.
- **Probe isolation.** All probe code lives under `spikes/task-002-macos-capture/`,
  outside `src/`, `src-tauri/src/`, and `observability/`, and is not a member of any build
  target. It can be deleted wholesale without touching the app.
- **No real titles persisted.** The probe reduces any title to `<redacted len=N
  bucket=...>` (presence + length only). It writes nothing to disk on its own; ephemeral
  logs are gitignored and carry a documented cleanup step.
- **Legacy boundary preserved.** The legacy manual-tracker surface (`time_entries`,
  manual-entry view, stopwatch CRUD) is treated as **reference-only**. It is not imported,
  migrated, reused, or wiped by this change. That decision is deferred to TASK-003.

---

## 3. Signal-by-signal feasibility

### 3.1 NSWorkspace / NSRunningApplication — frontmost app

**Feasible, no permission required.**

- `NSWorkspace.shared.frontmostApplication` yields `NSRunningApplication` with
  `bundleIdentifier`, `localizedName`, and `processIdentifier`. Sampling at a small
  interval (e.g. 1–5 s) gives the active-app timeline.
- **App switch / launch / terminate:** observable two ways — by polling
  `frontmostApplication`, or event-driven via
  `NSWorkspace.shared.notificationCenter` (`didActivateApplicationNotification`,
  `didLaunchApplicationNotification`, `didTerminateApplicationNotification`). Event-driven
  is preferred for the adapter (lower sampling cost, no missed fast switches); a low-rate
  poll backstops missed notifications.
- **Sleep / wake:** `NSWorkspace` posts `willSleepNotification` / `didWakeNotification`
  (and screensaver/lock notifications). These bound sampling gaps and are recorded as
  `capture_health` rather than appearing as fake activity.
- **Multiple Spaces / full-screen:** `frontmostApplication` tracks the active app across
  Spaces and into native full-screen — the *frontmost-app* signal is unaffected by Spaces.
  (Per-Space *window enumeration* is a Quartz concern, §3.3, not needed for the app signal.)
- **TCC permission:** **none.** Frontmost-app observation via NSWorkspace does not trip
  any TCC prompt on macOS 15. This is the key correction to the UX (FB-002, §8).

**Evidence quality:** high and reliable for "which app was active when". This alone gives
a usable degraded timeline even with every other permission denied.

### 3.2 AXUIElement — focused window / title

**Feasible where the Accessibility permission is granted; degrades explicitly otherwise.**

- Trust gate: `AXIsProcessTrusted()` reports whether the process holds the Accessibility
  grant. `AXIsProcessTrustedWithOptions([kAXTrustedCheckOptionPrompt: true])` can trigger
  the system prompt during onboarding.
- Title read: `AXUIElementCreateApplication(pid)` →
  `kAXFocusedWindowAttribute` → `kAXTitleAttribute`. The title is the visible title-bar
  string only — the same text macOS shows — never window contents.
- **Degraded states recorded as `capture_health`-shaped data (never silent drops):**

  | AX condition | `capture_health` state | Adapter behaviour |
  | --- | --- | --- |
  | Not trusted | `permission_denied` | App-only evidence; row marked "window context unavailable" |
  | Trusted, no focused window | `no_focused_window` | App-only evidence for that sample |
  | Attribute unsupported / app not AX-scriptable | `window_unavailable` | App-only; note source app limitation |
  | AX call error (e.g. `cannotComplete`) | `unavailable` (with AXError code) | App-only; transient, retried next sample |
  | Title present but empty | `title_empty` | App-only; some apps report empty titles |

- **Core-app coverage:** standard AppKit/Catalyst apps (Xcode, Finder, Mail, Slack,
  Linear desktop) expose `kAXTitle` reliably. Known weak spots to validate manually
  (§9): some Electron apps and some browsers expose inconsistent or generic titles; native
  full-screen windows sometimes report an empty title bar. These surface as the explicit
  degraded states above, not as wrong data.

**Evidence quality:** high for native apps; medium/variable for Electron/browser — and the
variability is *visible* as capture-health, which is exactly the BA's degraded-state
requirement.

### 3.3 Quartz Window Services — optional fallback

**Feasible technically, but permission-expensive; recommend against for titles by default.**

- `CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], …)` returns
  per-window dictionaries including geometry, owner pid, and `kCGWindowName`.
- **Screen Recording gate:** on macOS 10.15+ (so on the 15.7.7 target),
  `kCGWindowName` for windows owned by *other* applications is **only populated when the
  app holds Screen Recording permission**. Without it, geometry/owner are available but the
  *window name field is blank* — i.e. Quartz gives **no title benefit over AX** unless the
  user grants Screen Recording. The probe measures this via "windows exposing a name count"
  without ever reading the name value.
- **Permission burden:** Screen Recording is the most intrusive of the relevant grants —
  it implies (to the user, and in System Settings copy) the ability to record the screen,
  which directly contradicts Vire's "no screenshots, no screen pixels" promise. Granting it
  to a time-tracker is a hard sell and a privacy-posture liability.

**Recommendation (burden-justified, per spec):** **do not use Quartz for title capture by
default.** AX already provides the focused-window title at a far lower permission cost.
Quartz's only unique adds — z-order / window geometry / off-focus window enumeration — are
not required for the six-field evidence record. Keep Quartz **out of the default path**;
reconsider only if a concrete future need (not titles) justifies the Screen Recording cost,
and only as an explicit, separately-consented capability.

### 3.4 CGEventSource — idle / away

**Feasible, no permission required.**

- `CGEventSource.secondsSinceLastEventType(.combinedSessionState, eventType:)` gives the
  age of the last input event. The probe takes the **minimum age across input types**
  (`keyDown`, `mouseMoved`, mouse buttons, `scrollWheel`) for robustness, avoiding reliance
  on the `kCGAnyInputEventType` sentinel.
- **No event contents are read** — only the *time since* the last event. No keystrokes, no
  coordinates, nothing that could reconstruct input. This stays firmly inside the
  non-collection list.
- **State mapping (directional thresholds; finalised with Janne in TASK-005):**

  | Seconds since last input | State |
  | --- | --- |
  | `< 60` | `active` |
  | `60 … 300` | `idle_candidate` |
  | `≥ 300` | `away` |

  Thresholds are configurable. `idle_candidate` is deliberately a *candidate* — the review
  UX lets Janne confirm/reject idle blocks rather than auto-deleting time.

**Evidence quality:** high and cheap; combine with sleep/wake notifications (§3.1) so a
locked/asleep machine is `away` by system state, not just by input age.

---

## 4. Degraded states and capture health (enumerated)

The adapter (TASK-005) must persist these as `capture_health` evidence so gaps are
*explained*, never silent:

| Health state | Trigger | Effect on evidence |
| --- | --- | --- |
| `ax_permission_denied` | Accessibility not granted / revoked | App + idle only; titles absent, period marked degraded |
| `screen_recording_absent` | Quartz path attempted without Screen Recording | No Quartz title benefit; default path unaffected (Quartz off by default) |
| `no_focused_window` | AX trusted, app has no focused window | App-only for that sample |
| `window_unavailable` / `title_empty` | App not AX-scriptable / empty title | App-only; source-app limitation noted |
| `sampling_gap` | Polling interval missed / helper paused | Gap bounded by timestamps; not backfilled as activity |
| `system_sleep` / `system_wake` | NSWorkspace sleep/wake notifications | Period attributed `away`/boundary, not fake activity |
| `screen_locked` | Session lock notification | `away` |
| `helper_crash` / `helper_restart` | Capture helper process died/restarted | Gap recorded with reason; capture resumes on restart |
| `no_gui_session` | No `frontmostApplication` (e.g. headless) | Capture inert; nothing fabricated |

---

## 5. Permission burden analysis

| Permission | Needed for | Required? | Degraded behaviour if missing |
| --- | --- | --- | --- |
| **(none)** — NSWorkspace frontmost app | active-app name + switches | n/a (no grant) | always available in a GUI session |
| **(none)** — CGEventSource idle | idle / away state | n/a (no grant) | always available |
| **Accessibility (AX)** | focused-window **title** | **Required** for full quality | App + idle still captured; titles absent, blocks marked degraded → more "Unclassified" |
| **Screen Recording** | Quartz `kCGWindowName` titles | **Optional / not recommended** | Default path doesn't use it; no functional loss |

**Conclusion:** full-quality capture needs exactly **one** user grant (Accessibility).
The zero-grant baseline (app + idle) is still useful. The intrusive grant (Screen
Recording) is **not** on the default path. This is a low, defensible permission burden —
the lowest of the three implementation options benchmarked in §10.

---

## 6. ActivityWatch — reference benchmark only (not adopted)

ActivityWatch is **not installed** in this environment (`aw-server` absent) and was
**not** installed (no-dependency rule); this is a documentation benchmark only.

- **Evidence quality:** ActivityWatch's `aw-watcher-window` captures the same core signals
  Vire needs — active app + window title + (via `aw-watcher-afk`) idle/away — using the
  same macOS surfaces (AX for titles, input age for AFK). This independently corroborates
  that the chosen signal set is the right one.
- **Permission model:** identical pressure point — it relies on Accessibility for titles on
  macOS. No magic lower-permission path exists; the AX grant is intrinsic to title capture.
- **Why not adopt:** it runs its own local server + watcher processes (added dependency and
  packaging complexity), stores in its own schema, and has weak fit for Vire's specific
  needs — pi/Claude Code session boundaries and Langfuse AI-trace reconciliation are out of
  its scope. Adopting it would import a runtime and data model Vire would then have to bend.
- **Disposition:** **reference only.** Use it as a sanity check that the signal set and
  permission model are sound; do not build on it. (Import/extension remains a TASK-003
  option to weigh, not a TASK-002 decision.)

---

## 7. Field allowlist and APP-005 / SEC-001 implications

### 7.1 Positive allowlist (the only fields capture may produce)

| Field | Source | Notes |
| --- | --- | --- |
| `app_name` | NSWorkspace `localizedName` (bundle display name) | allowlisted; e.g. "Xcode" |
| `app_bundle_id` | NSWorkspace `bundleIdentifier` | stable key for matching |
| `window_title` | AXUIElement `kAXTitle` (**AX-permitted only**) | **highest-risk field** — free-form title-bar string |
| `time_range` | sampler timestamps | coarse start/end, local tz |
| `day` | derived | local calendar day |
| `source` | enum | `nsworkspace` · `accessibility` · `quartz` · `runtime` |
| `capture_health` / permission state | §4 | explains degraded periods |
| `idle/away state` | CGEventSource | `active` · `idle_candidate` · `away` |

### 7.2 Explicit non-collection list (never captured, by construction)

screenshots · screen pixels · keystrokes · mouse coordinates/input contents · full browser
contents or page bodies · full URLs / URL paths beyond what a title bar shows · terminal
command bodies · shell history · prompt or response text (pi, Claude Code, any AI tool) ·
environment dumps · secrets · credentials · clipboard.

The chosen APIs make this structural, not just policy: NSWorkspace exposes app identity
only; AX reads the title attribute only (not window contents); CGEventSource exposes event
*age* only (not events); Quartz (if ever used) exposes geometry/owner — and its name field
is gated and recommended off. None of the prohibited data is reachable on the default path.

### 7.3 Mapping to the six-field UX evidence record

`ux/prototype/setup-permissions.html` commits to six fields per evidence block. The
allowlist maps cleanly:

| UX field | Capture source |
| --- | --- |
| `day` | derived from sample timestamp |
| `time_range` | sampler start/end |
| `app_name` | NSWorkspace |
| `window_title` | AXUIElement (AX-permitted; else degraded marker) |
| `source` | enum (`nsworkspace`/`accessibility`/`quartz`/`runtime`/`langfuse`) |
| `review_state` | set by review UX, not by capture |

### 7.4 Implications for TASK-005 (capture adapter)

- The allowlist is a **field-level enforcement contract**, not a guideline. The adapter
  must be unable to emit any field outside §7.1.
- `window_title` is the single highest-risk field (can contain customer names, document
  titles, secrets). Its **degraded / unavailable / redacted handling must be first-class**,
  and **raw titles must never reach logs or the default export**. Test fixtures use
  synthetic titles only (carry the probe's redaction posture into the adapter's tests).
- **SEC-007 transparency:** the permissions screen must reflect the real burden from §5 —
  Accessibility required, Screen Recording not requested (see FB-002 reconciliation).
- **SEC-002 / SEC-003:** N/A for capture — no network client, no credentials. (They are
  importer-side concerns, TASK-007/012.)

---

## 8. FB-002 — UX "App & activity" permission row vs NSWorkspace reality

The UX prototype (`setup-permissions.html`) shows two permission rows: **Accessibility**
and **App & activity** (the latter described as "Lets Vire know which app is frontmost via
NSWorkspace"). The spike confirms the Architect's flag:

- **Finding:** frontmost-app observation via NSWorkspace requires **no TCC grant** on macOS
  15. There is no "App & activity" TCC permission that governs `frontmostApplication`.
- **Impact:** the UX row implies a permission that is not actually requested, which would
  **overstate** the permission burden in onboarding and the SEC-007 permissions page.
- **Recommendation (non-blocking, routed to BA/UX):** treat this report's §5 as the source
  of truth. Reconcile the permissions screen before TASK-005 builds the flow — either drop
  the "App & activity" row or relabel it as an informational "what we read" item (not a
  grantable permission). **Accessibility remains the one real grant**; Screen Recording is
  optional/not-recommended and should not appear as required.

---

## 9. Manual validation matrix

Run on a real interactive login session (the probe samples live state). Expected
behaviour is what QA confirms; none of these steps should persist a real title.

| # | Case | Steps | Expected observable behaviour |
| --- | --- | --- | --- |
| 1 | Frontmost app, no perms | Fresh run, deny all | App name + bundle captured; `ax.trusted=false`; titles `permission_denied`; idle state still computed |
| 2 | App switch | Switch Xcode→Slack→Finder | Frontmost app updates each switch; no missed switch over a 5 s window |
| 3 | App launch / terminate | Launch then quit an app | Launch/terminate observed; terminated app no longer frontmost |
| 4 | AX granted — core apps | Grant Accessibility; focus Xcode, Mail, Finder, Slack | `ax.focusedTitle=granted` with redacted length > 0 for each |
| 5 | AX granted — weak apps | Focus an Electron app / a browser | Title either present or an explicit `no_focused_window`/`window_unavailable`/`title_empty` — never wrong data |
| 6 | AX revoke mid-session | Revoke Accessibility while running | Next sample flips to `permission_denied`; app+idle continue; period marked degraded |
| 7 | Sleep / wake | Sleep the Mac, wake | Sleep/wake recorded as health; gap bounded, not backfilled as activity |
| 8 | Screen lock | Lock screen 1 min | State `away`; no fabricated activity |
| 9 | Multiple Spaces | Move app to a second Space, switch Spaces | Frontmost app tracked correctly across Spaces |
| 10 | Full-screen app | Enter native full-screen | App captured; title present or explicit degraded marker (some full-screen windows report empty title) |
| 11 | Idle thresholds | Idle 0 / 90 / 360 s | States `active` / `idle_candidate` / `away` respectively |
| 12 | Quartz / Screen Recording | Run Quartz sample with SR denied, then granted | `windowsExposingName=0` (other-app names blank) when denied; > 0 when granted — confirms the SR gate and the burden |
| 13 | No GUI session | Run without a login session | Frontmost `<none>`; nothing fabricated |
| 14 | Privacy check | Inspect all probe output | Every title appears only as `<redacted len=N …>`; no real title, secret, command body, or env value present |

---

## 10. Tauri+helper vs Swift-first — signal for TASK-003 (no decision here)

This spike **produces the signal**; TASK-003 makes the call.

| Dimension | Tauri shell + native macOS helper | Swift / AppKit-first | ActivityWatch import/reference |
| --- | --- | --- | --- |
| Capture API access | Full (helper is native; same APIs validated here) | Full (native) | Indirect (its watchers) |
| Permission friction | One AX grant; helper must be the TCC-trusted binary (packaging/codesign care needed so the *helper*, not the webview, holds AX) | One AX grant; simplest trust model (one app binary) | One AX grant (its process) |
| Reuse of existing repo | High — keeps current review/store/export shell; webview CSP stays locked, raw titles never cross into renderer | Low — rebuild review/export/store | Low — adopt its model |
| Added complexity | IPC + helper packaging/lifecycle (crash/restart health already modelled, §4) | Rebuild UI/store/export from scratch | New runtime + schema + weak pi/Langfuse fit |
| Privacy boundary | Strong if capture stays in helper, behind `connect-src ipc:` CSP; raw titles never enter webview or network | Strong (single native process) | External process owns the data model |

**Observed friction signal:** the capture APIs themselves are equally reachable from a
native helper or a Swift-first app — capture feasibility does **not** favour either. The
real trade is **helper packaging / IPC / TCC-trust placement** (Tauri+helper) **vs. rebuild
cost of review/store/export** (Swift-first). The AX grant must attach to whichever binary
actually calls AX (the helper, not the webview). TASK-003 should weigh helper packaging
cost against shell-reuse value with this in hand; nothing here forces the decision.

---

## 11. Exit gate

- [x] Capture feasibility validated across NSWorkspace, AXUIElement, optional Quartz, and
  idle signals, with evidence quality and degraded states documented (§3–§4).
- [x] Permission burden documented: required (Accessibility) vs optional/not-recommended
  (Screen Recording); frontmost-app and idle need no grant (§5).
- [x] Positive field allowlist and explicit non-collection list produced and mapped to the
  six-field UX record; APP-005 / SEC-001 / SEC-007 implications for TASK-005 identified (§7).
- [x] Manual validation matrix provided (§9).
- [x] Tauri+helper-vs-Swift-first signal produced for TASK-003 (§10) — **decision deferred**.
- [x] FB-002 (UX permission-row accuracy) confirmed and routed to BA/UX (§8).
- [x] No capture MVP shipped; no product evidence rows written; no implementation path
  decided; legacy manual-tracker surface untouched (reference-only).

**Gate met:** *capture feasibility and permission burden documented; APP-005 field-allowlist
implications identified.* Ready for SW-3 (QA).

---

## 12. Open questions routed downstream

- **Idle thresholds** (60 s / 300 s) are directional — finalise with Janne in TASK-005.
- **Electron / browser title reliability** — quantify per-core-app in TASK-005 using the
  matrix (§9 case 5); the degraded-state model already absorbs the variability.
- **Helper TCC-trust placement** — TASK-003 must decide which binary holds the AX grant if
  the Tauri+helper path is chosen (§10).
- **FB-002 permissions-screen reconciliation** — BA/UX to align `setup-permissions.html`
  with §5 before TASK-005 builds the permissions flow.

---

## 13. References

- BA `04_technical_plan.md` §3 (decision table), §4 (components / capture adapter), §5
  (spike plan), §6 (risk register), §13 (WP-002).
- BA `05_project_plan_epics.md` §3 (TASK-002), §6 (risk register).
- UX `ux/prototype/setup-permissions.html` (six-field evidence record; permission rows).
- OpenSpec change `task-002-macos-capture-feasibility/` (proposal, design, spec, arch-review).
- Apple docs (read, per tech-plan provenance): NSWorkspace; AXUIElement / Accessibility;
  Quartz Window Services; CGEventSource / CGEventType; Screen & System Audio Recording
  privacy control.
- Supporting probe: `spikes/task-002-macos-capture/` (compile-validated, redaction-by-default).
