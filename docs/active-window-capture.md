# Active-App and Idle Capture (TASK-048)

Vire can optionally sample the frontmost macOS application and a coarse idle state at a configurable cadence using only zero-permission OS APIs.

**Default: OFF.** A capture thread is spawned at startup and reads the configuration on each tick, but while capture is disabled it calls no native capture API and writes no evidence.

## Privacy posture

| What is captured | What is never captured |
|---|---|
| Frontmost app bundle ID + display name (e.g. `com.apple.Terminal` / `Terminal`) | Window titles |
| Coarse idle state (`active` / `idle_candidate` / `away`) | Accessibility tree, screen pixels |
| Observation timestamp | Keystrokes, mouse content, clipboard |
| | URLs, file paths, terminal command bodies |
| | Prompts / responses, secrets |

Evidence rows always carry `window_title = NULL` and `title_state = absent_no_permission`. This is a code-layer invariant — no path in this release writes a window title.

**APIs used:** `NSWorkspace.frontmostApplication` (bundle ID) and `CGEventSource.secondsSinceLastEventType(kCGAnyInputEventType)` (idle age). Both are zero-permission; no TCC grant (`Accessibility`, `Screen Recording`) is requested or required. No `CGEventTap` or window-list call is made.

Data is written only to the local SQLite database (`vire.sqlite`) and never sent over the network.

## Enabling capture

Capture is controlled by the `active_window_capture_enabled` settings key (settings table takes precedence) or the `VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED` environment variable. Only explicit affirmatives enable it: `1`, `true`, `yes`, `on` (case-insensitive). All other values keep it OFF.

```sh
export VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED=1
npm run tauri:dev
```

In-app controls (TASK-056): **Settings → Active-window capture** provides the enable toggle and the
sample-interval, idle-candidate, away, and retention knobs, plus a live capture status/health readout
and the privacy table above. The panel reads and writes the same `settings` keys via the
`get_active_window_capture_settings` / `set_active_window_capture_settings` IPC commands, so a change
takes effect on the loop's next tick — no app restart. Inputs are validated to safe bounds
(`sample_seconds ∈ [1,3600]`, `idle_candidate_seconds ∈ [1,86400]`, `idle_away_seconds` greater than
`idle_candidate_seconds` and ≤ `86400`, `retention_days ∈ [1,3650]`) and rejected with a clear message
rather than silently clamped. Capture
stays **OFF by default**; the UI never enables it without an explicit toggle. `title_mode` is shown
read-only as *never captured* and is **not** user-togglable — storing window titles requires an
Accessibility grant that this surface does not request.

## Configuration

Settings-table key > env var > compiled default.

| Settings key | Env var | Default | Notes |
|---|---|---|---|
| `active_window_capture_enabled` | `VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED` | `false` | Main on/off switch. |
| `active_window_sample_seconds` | `VIRE_ACTIVE_WINDOW_SAMPLE_SECONDS` | `5` | Sampling cadence in seconds. Must be > 0. |
| `active_window_idle_candidate_seconds` | `VIRE_ACTIVE_WINDOW_IDLE_CANDIDATE_SECONDS` | `60` | Inactivity before `idle_candidate`. |
| `active_window_idle_away_seconds` | `VIRE_ACTIVE_WINDOW_IDLE_AWAY_SECONDS` | `300` | Inactivity before `away`. |
| `active_window_retention_days` | `VIRE_ACTIVE_WINDOW_RETENTION_DAYS` | `30` | Evidence retention window (days). |
| `active_window_title_mode` | `VIRE_ACTIVE_WINDOW_TITLE_MODE` | `redacted` | `redacted` (always NULL) or `stored` (future opt-in). |

## How the loop works

The capture loop runs on a dedicated OS thread spawned at app startup. Per tick:

1. Reads `CaptureConfig` from the settings DB.
2. If disabled: clears any open evidence block and returns — no native API is called.
3. Queries `NSWorkspace.frontmostApplication` for bundle ID and display name.
4. Queries `CGEventSource` for idle age; maps to `active` / `idle_candidate` / `away`.
5. Writes a raw observation, upserts the coalescing evidence block, and records a health row in one atomic SQLite transaction. In-memory state updates only after commit.

If no GUI session is available (e.g. early startup), a `no_gui_session` health row is written and no raw or evidence row is produced.

## Storage tables (TASK-046 schema, read/written by TASK-048)

| Table | Purpose |
|---|---|
| `active_window_raw_evidence` | Per-sample observations (append-only, integer PK). |
| `active_window_evidence` | Coalesced reviewable blocks (upserted on `(day, start_ts, app_bundle_id)`). |
| `active_window_capture_health` | First-class health and gap rows — every capture gap is explained, never silent. |

These tables carry `window_title` and `title_state` columns (TASK-046 schema), but TASK-048 never collects a title — `window_title` is always persisted as `NULL` (the code-layer invariant above). No column exists in any of these tables for screenshots, keystrokes, URLs, or credentials.

## Not yet available

- Window title capture (requires an Accessibility grant; planned)
- Accessibility-based metadata (requires AX permission; planned)
