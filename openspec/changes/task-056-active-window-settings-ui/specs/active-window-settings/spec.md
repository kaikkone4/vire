# Spec delta — active-window-settings

## ADDED Requirements

### Requirement: Users can view and control zero-permission capture from Settings

The app SHALL expose the existing zero-permission active-window capture through a Settings surface and a
read/write IPC seam. It SHALL provide `get_active_window_capture_settings` returning the resolved
configuration (capture enabled, sample interval seconds, idle-candidate seconds, idle-away seconds,
retention days, a fixed `title_mode='redacted'` marker, and `platform_supported`) and
`set_active_window_capture_settings` persisting those knobs to the `settings` table keys the capture loop
already reads (`active_window_capture_enabled`, `active_window_sample_seconds`,
`active_window_idle_candidate_seconds`, `active_window_idle_away_seconds`, `active_window_retention_days`).
It SHALL NOT add, alter, or migrate any table or column, SHALL NOT change capture behavior, and SHALL NOT
require an application restart for a change to take effect (the loop re-reads config each tick).

#### Scenario: Reading current capture settings

- **WHEN** the user opens Settings
- **THEN** the panel shows the current enabled state and every knob resolved with the
  `settings > env > default` precedence
- **AND** on non-macOS the controls are disabled and the panel states capture is available on macOS only.

#### Scenario: Changing a setting takes effect without restart

- **WHEN** the user saves a new sample interval or toggles capture on
- **THEN** the corresponding `settings` row is written and the returned view reflects it
- **AND** the running capture loop applies the change on its next tick with no app restart.

### Requirement: Capture settings are validated to safe bounds

`set_active_window_capture_settings` SHALL reject invalid input with a clear error rather than persisting
it. It SHALL require `sample_seconds ≥ 1`, `idle_candidate_seconds ≥ 1`, `idle_away_seconds >
idle_candidate_seconds`, and `retention_days ≥ 1`, and SHALL apply sane upper caps. Capture SHALL remain
disabled by default: the task SHALL NOT enable capture except by an explicit user action, and a rejected or
malformed write SHALL never yield a zero/negative cadence that disables sampling.

#### Scenario: Invalid interval is rejected

- **WHEN** the user submits `sample_seconds = 0` or an away threshold not greater than the idle-candidate
  threshold
- **THEN** the write is rejected with a clear message and no `settings` row is changed
- **AND** the previously saved, valid configuration remains in effect.

#### Scenario: Default remains off

- **WHEN** the feature ships and the user has never enabled capture
- **THEN** `capture_enabled` resolves to `false` and no sampling occurs
- **AND** enabling capture requires an explicit toggle by the user.

### Requirement: Capture health and status are visible, with every gap explained

The Settings surface SHALL show a capture status derived from the existing store tables via a read-only
snapshot (last sample timestamp, samples captured today, evidence blocks within the retention window, and
current open plus recent `capture_health` states). A degraded or absent-capture condition SHALL be named
with its cause (e.g. `no_gui_session`, `sampling_gap`, capture disabled) and SHALL NOT be shown as a silent
empty result. The status projection SHALL carry only allowlisted, non-sensitive values (counts, timestamps,
bundle identifiers, and bounded coarse state/detail codes) and SHALL NOT expose a window title, path, URL,
command, or secret.

#### Scenario: Degraded capture is explained

- **WHEN** capture is enabled but a health row is open (e.g. `no_gui_session`)
- **THEN** the status readout names the degraded state and when it began
- **AND** it does not render as an empty or zeroed status.

#### Scenario: Enabled but no samples yet

- **WHEN** capture was just enabled and no sample has been written
- **THEN** the status states capture is on and awaiting the first sample
- **AND** the last-sample field is shown as absent rather than as an error.

### Requirement: The privacy explanation is truthful and matches actual capture

The Settings surface SHALL present a plain-language explanation of exactly what is and is not captured, and
the app SHALL NOT display copy that contradicts the actual capture behavior. It SHALL state that captured
data is limited to the frontmost application bundle identifier and display name, a coarse idle state, and an
observation timestamp, and that window titles, the Accessibility tree, screen pixels, keystrokes, mouse
content, URLs, file paths, terminal command bodies, clipboard, prompt/response text, and secrets are never
captured. The prior static claim that the app "does not collect active windows or idle state" SHALL be
replaced with copy driven by the real setting.

#### Scenario: Copy reflects real state when capture is enabled

- **WHEN** capture is enabled
- **THEN** the Settings copy and the Today capture banner state that active-app and idle state are being
  captured locally, and link to the what-is/never-captured explanation
- **AND** no on-screen text claims that active windows or idle state are not collected.

### Requirement: The settings UI requests no new permission and no window titles

The change SHALL request **no** macOS TCC permission and SHALL add no Accessibility, Screen Recording, or
event-tap code. `title_mode` SHALL remain `redacted` and SHALL NOT be user-togglable through this surface;
enabling stored window titles (which requires an Accessibility grant) is out of scope and deferred to the
Accessibility-title task. The change SHALL add no network call and SHALL NOT alter the webview CSP.

#### Scenario: No window-title opt-in is offered

- **WHEN** the user views the capture settings
- **THEN** `title_mode` is shown read-only as "never captured" and there is no control to store window
  titles
- **AND** no code path requests an Accessibility grant or reads a window title.
