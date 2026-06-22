# TASK-048 — Design: zero-permission app + idle capture

The first capture slice. It fills the TASK-046 store with the two signals that need **no macOS
permission** — frontmost-app identity and idle/away state — and nothing else. It inherits the
TASK-035 constraints C1–C7 verbatim (`task-035-active-window-tracking/design.md` §2); this design
operationalizes **C2 (zero-grant baseline), C3 (allowlist), C4 (health), C6 (retention), C7 (no
egress)** at the capture layer. **C5 (title redaction) is honored by not reading any title at all**,
and **C1 (sidecar placement) is scoped** — see §3 and FB-048 in `arch-review.md`.

## 1. Where this sits

```
                         ┌─────────────────────────────────────────────┐
  Tauri .setup()         │  Rust core (the native host process)         │
  background OS thread ──▶│  active_window::capture loop  ◀── THIS TASK  │
  (mirrors lib.rs:1189)  │    NSWorkspace.frontmostApplication          │
                         │    CGEventSource last-event age              │
                         │        │ RawObservation (window_title=NULL,  │
                         │        │   title_state=absent_no_permission)  │
                         │        ▼                                      │
                         │  active_window::store  (TASK-046, built)      │
                         │    insert_raw_observation / upsert_evidence_  │
                         │    block / record_capture_health / prune      │
                         └─────────────────────────────────────────────┘
                                  │ local SQLite (same db)
   AX-title task (later) ─────────┘  (adds kAXTitle + sidecar/externalBin/TCC — FB-048)
   permissions UI (later) ────────┘  (user toggle + transparency; owns FB-002)
   suggestion integration (later) ┘  (read seam into task-032)
```

The capture loop is a **sibling of the Langfuse auto-import scheduler**: a dedicated background OS
thread spawned in `.setup()`, gated by an enable switch, opening throwaway connections, making **no**
renderer call and **no** network call (`lib.rs:1185–1195` is the template). The webview is never
involved; this is native host code exactly where `runtime_observer` and the importer already run.

## 2. The two zero-permission signals (verified — see §6 source table)

| Signal | API | Field(s) | TCC grant |
| --- | --- | --- | --- |
| Active app | `NSWorkspace.shared.frontmostApplication` → `NSRunningApplication` | `app_name` (`localizedName`), `app_bundle_id` (`bundleIdentifier`), `source='nsworkspace'` | **none** |
| Idle / away | `CGEventSource.secondsSinceLastEventType(.combinedSessionState, …)`, min across input types | `idle_state` ∈ `active` / `idle_candidate` / `away` | **none** |

`idle_state` thresholds: `idle_candidate` ≥ 60 s, `away` ≥ 300 s (TASK-002 §3.4 — directional,
configurable; `active` otherwise). The loop computes idle from event **age** only — it creates no
`CGEventTap` and reads no event content, which is the exact mechanism that keeps it out of both
Accessibility and Input Monitoring (§6, Apple forum thread/122492). `window_title` is **always**
`NULL` with `title_state = absent_no_permission`; no AX call (`AXIsProcessTrusted`,
`AXUIElementCopyAttributeValue`) is ever made in this slice.

Tick cadence: a single configurable interval (default ~5 s) reads both signals per tick. Event-driven
`NSWorkspace` activation notifications (TASK-002 §3.1) are a later optimization — **poll-only is the
smallest, lowest-risk path** and is what this slice ships.

## 3. Build-shape decision — **in-process Rust core, not a Swift `externalBin` sidecar** (load-bearing)

TASK-035 §3 sized *TASK-037a* as *"sidecar scaffold + `externalBin` + IPC + frontmost-app + idle."*
New primary-source evidence (§6) revises that shape **for the zero-permission slice only**:

**Decision: implement this slice in the Rust core using `objc2-app-kit` (NSWorkspace) and
`objc2-core-graphics` (CGEventSource). Do not add a Swift sidecar, `externalBin`, or IPC here.**

Rationale:

1. **C1's driver is absent in this slice.** C1/DEC-019 places capture in a native sidecar so *"the
   helper, not the webview, holds the **AX** grant"* and *"raw **titles** never cross into the
   renderer."* This slice requests **no AX grant** and reads **no titles** — so there is no trust
   boundary to externalize and no sensitive value to isolate. App identity (bundle id/name) and idle
   state are allowlisted, non-sensitive, and already destined for the same SQLite store.
2. **`externalBin` imports real, documented packaging risk now, for no benefit here.** Tauri's own
   issue tracker shows sidecars break macOS **codesigning/notarization** (§6: tauri#11992, #9422,
   #12690). Pulling that into the *smallest* slice contradicts both TASK-035 §3 (*"don't strand the
   high-risk native-packaging work inside otherwise-low-risk work"*) and the TASK-048 mandate to
   proceed **without requiring Apple Developer ID signing if possible**.
3. **Zero-permission capture is signing-independent.** Because no TCC grant is requested, **no TCC
   entry is created**, so the app's (currently ad-hoc/unsigned — no signing config in
   `tauri.conf.json`) signature is irrelevant to this slice. This is precisely why it can ship for
   local use with no Developer ID. (Contrast: an AX grant is keyed to the code signature and *resets
   on each rebuild* of an unsigned app — a real friction that belongs to the AX-title task, not here.)
4. **Migration cost is low and non-throwaway.** The store-write + coalescing logic stays in the Rust
   core regardless. If the AX-title task later introduces the sidecar, only the ~30-line poll moves
   into Swift behind IPC, feeding the *same* `store` API — work that task does anyway. In-process-first
   defers the sidecar to when AX titles actually require it.

**This refines a BA-ratified decision (DEC-019), so it is routed to ba-architect as FB-048**
(`arch-review.md`) for ratification. It is **non-blocking**: the task proceeds either way and neither
shape needs signing. If BA declines the refinement, the fallback is the strict-C1 Swift sidecar
(more packaging risk, no functional change to this spec's behavior).

**Known implementation note (for the developer, not a blocker):** `objc2-app-kit` may mark
`NSWorkspace` reads as main-thread-affined. The poll can either run the NSWorkspace read on the app's
main thread (dispatch from the background tick) or confirm off-main access is sound; CGEventSource
queries are thread-safe. Resolve during SW-2.

## 4. Coalescing — raw samples → normalized blocks

Each tick inserts one `active_window_raw_evidence` row (short retention). The loop also maintains the
**open normalized block** and calls `upsert_evidence_block` (keyed `(day, start_ts, app_bundle_id)`,
idempotent) so a contiguous run of the same `(app_bundle_id, idle_state)` collapses into one
reviewable `active_window_evidence` block; a change in app or idle-state closes the block (sets
`end_ts`, `duration_seconds`) and opens the next. `title_state` stays `absent_no_permission` and
`window_title` stays `NULL` throughout. No association to `time_entries` is written (read-time only,
DEC-001).

## 5. Lifecycle, config, retention

- **Enable switch (default OFF):** `active_window_capture_enabled` (settings) /
  `VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED` (env), resolved with the same precedence as the existing
  `ActiveWindowConfig` (settings > env > default). Default OFF keeps capture from running silently
  before the downstream transparency/consent UI exists; Janne flips it on for dogfooding. Tick
  interval is configurable (`VIRE_ACTIVE_WINDOW_SAMPLE_SECONDS`, default 5) for tests/power-use.
- **Idle thresholds** are capture-side config (not stored on rows): `VIRE_ACTIVE_WINDOW_IDLE_*`
  defaults 60 / 300 s.
- **Retention (C6):** the loop calls the existing `prune_expired(now, retention_days)` on a coarse
  cadence (e.g. once per day-boundary crossing), using the `ActiveWindowConfig.retention_days` already
  defined in TASK-046. It touches only `active_window_*` tables; a `time_entries` sentinel survives
  (structurally — no join/cascade). This task adds the **driver**, reusing TASK-046's tested primitive.

## 6. Verified vs. assumption (primary sources)

| Claim | Status | Source |
| --- | --- | --- |
| `NSWorkspace.frontmostApplication` returns the active app (name/bundle/pid) and needs **no** TCC grant | **Verified** (Apple ref + TASK-002 compile-probe; TCC gates `kCGWindowName`/AX, not frontmost-app identity) | developer.apple.com/documentation/appkit/nsworkspace/frontmostapplication ; hacktricks.wiki/en/macos-hardening/.../macos-input-monitoring-screen-capture-accessibility.html |
| Idle via `CGEventSource.secondsSinceLastEventType(.combinedSessionState, …)` reads event **age** only, no content | **Verified** (Apple CoreGraphics ref; reads counter/timer, no tap) | developer.apple.com/documentation/coregraphics/cgeventsource |
| TCC permission attaches to **creating a `CGEventTap`** (`defaultTap`→Accessibility, `listenOnly`→Input Monitoring), **not** to `secondsSinceLastEventType` (no tap created) | **Verified (mechanism)** — Apple engineer note in thread | developer.apple.com/forums/thread/122492 |
| Idle read produces **no TCC prompt** on the target macOS | **Assumption — confirm by live run** (TASK-002 compile-validated only; not executed against live activity). First acceptance step in `tasks.md`. | TASK-002 §2, §3.4 |
| `objc2-app-kit` exposes `NSWorkspace`; `objc2-core-graphics` exposes `CGEventSource::seconds_since_last_event_type` | **Verified** (docs.rs) | docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSWorkspace.html ; docs.rs/objc2-core-graphics |
| Tauri `externalBin` sidecars have **known macOS codesigning/notarization friction** | **Verified** (Tauri issue tracker) | github.com/tauri-apps/tauri/issues/11992 ; /issues/9422 ; /issues/12690 ; v2.tauri.app/develop/sidecar/ |
| Zero-permission capture is **signing-independent** (no TCC entry created → signature irrelevant) | **Verified (mechanism)** + repo state (no signing/sandbox config in `tauri.conf.json`) | reasoning from TCC model + `src-tauri/tauri.conf.json` |
| App is **not sandboxed** (NSWorkspace cross-app reads unrestricted) | **Verified** (no `app-sandbox`/entitlements in repo) | `src-tauri/` (no `*.entitlements`, no sandbox config) |

## 7. Out of scope (guard against creep)

Window titles / `kAXTitle` / Accessibility grant; Quartz / Screen Recording; any `CGEventTap` or
Input Monitoring; the Swift sidecar / `externalBin` / codesign / notarization / TCC-trust work
(deferred to the AX-title task, FB-048); permissions/privacy/retention **UI** and any user-facing
capture toggle (downstream permissions task); `src/main.ts` / renderer / `invoke_handler!` changes and
any renderer status surface; feeding evidence into the task-032 suggestion/review pipeline;
classification/approval into `time_entries`; CSV export of active-window evidence; any network client
or CSP change; sleep/wake/lock **notification observers** (this slice covers gaps via `no_gui_session`
+ `sampling_gap` only); changes to the legacy manual `time_entries` surface.
