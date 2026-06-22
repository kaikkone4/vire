# Spec delta — active-window-capture

## ADDED Requirements

### Requirement: Capture records active-app and idle state using only zero-permission macOS APIs

The capture loop SHALL record the frontmost application identity (via `NSWorkspace`
`frontmostApplication`) and the idle/away state (via `CGEventSource` last-event age) and SHALL request
**no** macOS TCC permission. It SHALL NOT call any Accessibility API (`AXIsProcessTrusted`,
`AXUIElementCopyAttributeValue`), SHALL NOT create a `CGEventTap`, SHALL NOT use Quartz
`CGWindowListCopyWindowInfo`, and SHALL NOT read any event content — only event *age*. Every captured
row SHALL have `window_title = NULL` and `title_state = absent_no_permission`, and `source` SHALL be
`nsworkspace`.

#### Scenario: Capture runs with no permission prompt

- **WHEN** capture is enabled and the loop samples on the target macOS
- **THEN** no TCC permission prompt is shown and no Accessibility/Screen-Recording/Input-Monitoring
  grant is requested
- **AND** the code path makes no AX call, creates no event tap, and reads no event content.

#### Scenario: Idle state is derived from last-event age

- **WHEN** the last input event is 0 s / 90 s / 360 s old
- **THEN** `idle_state` is `active` / `idle_candidate` / `away` respectively (thresholds configurable)
- **AND** no keystroke, mouse coordinate, or event content is read or stored.

### Requirement: Capture writes only through the TASK-046 allowlist-enforcing store API

The capture loop SHALL persist observations exclusively through the existing
`active_window::store` API (`insert_raw_observation`, `upsert_evidence_block`,
`record_capture_health`). It SHALL NOT define a new table, a generic column writer, or any
arbitrary-SQL entry point, and SHALL NOT bypass the `title_mode` redaction gate. Contiguous samples of
the same `(app_bundle_id, idle_state)` SHALL coalesce into one normalized `active_window_evidence`
block via the idempotent `(day, start_ts, app_bundle_id)` upsert. No capture row SHALL be associated
to `time_entries` or carry a `project_id` (association is read-time only, DEC-001).

#### Scenario: A sample persists as an allowlisted observation

- **WHEN** the loop samples a frontmost app and an idle state
- **THEN** one `active_window_raw_evidence` row is written with `source='nsworkspace'`,
  `window_title IS NULL`, and `title_state='absent_no_permission'`
- **AND** the title stays NULL under the default redacted `title_mode`.

#### Scenario: Contiguous samples coalesce into one reviewable block

- **WHEN** several consecutive samples share the same `app_bundle_id` and `idle_state`
- **THEN** they collapse into a single `active_window_evidence` block with `end_ts` and
  `duration_seconds` extended via upsert
- **AND** a change in app or idle state closes the open block and opens the next.

### Requirement: Capture is opt-in and adds no renderer, IPC, network, sidecar, or CSP surface

Capture SHALL be governed by an enable switch (`active_window_capture_enabled` settings key /
`VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED` env) that defaults to **disabled**, resolved with the existing
settings > env > default precedence. The loop SHALL run on a dedicated background OS thread started
from the Tauri setup hook and SHALL NOT add an `invoke_handler` command, change the renderer or
`src/main.ts`, make any network call, add an `externalBin` / sidecar, or alter the webview CSP.

#### Scenario: Disabled by default writes nothing

- **WHEN** the app starts with no capture-enable override set
- **THEN** the loop samples nothing and writes no `active_window_*` rows
- **AND** the renderer, IPC surface, and existing features behave exactly as before.

#### Scenario: Enabled capture adds no new surface

- **WHEN** capture is enabled via the settings key or env
- **THEN** observations accumulate in the local store on the background thread
- **AND** no new IPC command is registered, no network call is made, the `connect-src ipc:` CSP is
  unchanged, and no `externalBin` is bundled.

### Requirement: Degraded capture states are recorded as first-class health rows

The loop SHALL record reachable degraded states as `active_window_capture_health` rows rather than
silently dropping samples: `no_gui_session` when no frontmost application exists, and `sampling_gap`
when the wall-clock interval between consecutive ticks exceeds twice the configured tick interval
(covering sleep/suspend without notification observers). A health row's `detail` SHALL carry only a
bounded coarse reason code and SHALL NEVER carry a title, app name, path, command, or secret. No
degraded period SHALL be backfilled as fabricated activity.

#### Scenario: No GUI session is explained, not fabricated

- **WHEN** `frontmostApplication` returns nil at a tick
- **THEN** a `no_gui_session` health row is written and no activity row is fabricated.

#### Scenario: A sampling gap is bounded

- **WHEN** the gap between two ticks exceeds twice the tick interval (e.g. after sleep)
- **THEN** a `sampling_gap` health row bounds the gap and the missing span is not backfilled as
  activity.

### Requirement: Capture drives bounded retention without touching approved time

The loop SHALL invoke the existing `prune_expired(now, retention_days)` primitive on a coarse cadence
so raw evidence stays within the configured retention window. Pruning SHALL delete only
`active_window_*` rows and SHALL NEVER delete or mutate `time_entries` or any approved summary.

#### Scenario: Retention prunes only active-window rows

- **WHEN** the retention driver runs with expired and in-window active-window rows present
- **THEN** only expired `active_window_*` rows are removed
- **AND** in-window active-window rows and a sentinel `time_entries` row both survive.
