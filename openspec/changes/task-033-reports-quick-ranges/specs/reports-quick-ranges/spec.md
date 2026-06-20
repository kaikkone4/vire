# Spec delta — reports-quick-ranges

Adds a new capability: the Reports view offers quick-range preset buttons that set the report's date range
to a common relative window ending today, as a convenience over the existing custom start/end date inputs.
The presets feed the same range the user could type by hand; the report query, totals, entry table, and
CSV export are unchanged and reuse those dates.

## ADDED Requirements

### Requirement: Quick-range presets set the report date range

The Reports view SHALL present quick-range preset controls for common relative windows, including at
minimum **Last 7 days**, **Last 14 days**, and **Last 30 days**.

Activating a preset SHALL set the report's start and end dates to that window — end = today, start = today
minus (N − 1) days, an **inclusive** window of N calendar days ending today — and SHALL refresh the
report (project totals and the entry table) to that range while keeping the currently selected project
filter.

The window SHALL be computed in the user's local time, consistent with how the app derives "today"
elsewhere, so the dates do not shift by a day across time zones or near midnight.

A preset window SHALL always be a valid range (start on or before end), so applying a preset never
produces a date-range error.

The existing custom start/end date inputs SHALL remain available and functional; a preset fills those
dates but does not remove or disable manual date entry.

#### Scenario: Pressing "Last 7 days" sets a seven-day window ending today

- **WHEN** the user opens the Reports view and presses **Last 7 days**
- **THEN** the report's end date is set to today and the start date to six days before today (a seven-day
  inclusive window)
- **AND** the project totals and entry table refresh to show that range.

#### Scenario: A preset keeps the selected project filter

- **WHEN** the user has a project selected in the Reports project filter and presses a quick-range preset
- **THEN** the report refreshes to the preset's window with that project filter still applied.

#### Scenario: CSV export uses the preset-selected range

- **WHEN** the user presses a quick-range preset and then exports the report to CSV
- **THEN** the exported file covers the preset's date window (the same start and end the preset set).

#### Scenario: Manual date entry still works after using a preset

- **WHEN** the user has pressed a preset and then types a custom start and end date and applies the range
- **THEN** the report refreshes to the custom range (the preset did not disable manual date entry).

#### Scenario: A relative window near a month boundary stays correct in local time

- **WHEN** today is the 3rd of a month and the user presses **Last 7 days**
- **THEN** the start date is the 28th of the previous month and the end date is the 3rd (a seven-day
  inclusive window), with no off-by-one from time-zone conversion.
