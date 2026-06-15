# Spec delta — csv-export

## ADDED Requirements

### Requirement: Exported user-controlled text cells neutralize spreadsheet formula injection

The CSV exporter SHALL neutralize spreadsheet/CSV formula injection in every exported user-controlled
text cell — project name, entry note, and any future exported user-controlled text field. A cell SHALL
be neutralized by prepending a single-quote (`'`) guard when, after stripping any leading whitespace,
its first character is one of `=`, `+`, `-`, or `@`, or when its first raw character is a tab (`\t`),
carriage return (`\r`), or line feed (`\n`). Numeric and fixed-format columns (date, start/end time,
durations) are not user-controlled free text and SHALL NOT be passed through this neutralizer.

#### Scenario: Formula-like project name is neutralized

- **WHEN** a project name begins with a formula character such as `=WEBSERVICE("https://example.invalid/")`
- **THEN** the exported project cell is prefixed with `'` so a spreadsheet does not evaluate it
- **AND** the raw formula is never emitted as an executable cell (no `,=WEBSERVICE…` appears in the file).

#### Scenario: Formula-like note after leading whitespace is neutralized

- **WHEN** an entry note's first non-whitespace character is a formula character (e.g. ` +SUM(1,2)…`)
- **THEN** the exported note cell is prefixed with `'`, because a spreadsheet strips leading whitespace
  before interpreting a cell
- **AND** the raw formula is never emitted as an executable cell (no `,+SUM…` appears in the file).

#### Scenario: Leading control character is neutralized

- **WHEN** a user-controlled text cell begins with a tab, carriage return, or line feed
- **THEN** the exported cell is prefixed with `'` so the leading control character cannot drive
  spreadsheet evaluation.

### Requirement: The exporter preserves legitimate user text rather than mutating it

Neutralization SHALL prepend the `'` guard to the **original** cell value and SHALL NOT delete the
user's leading or trailing whitespace or any internal characters. The input layer SHALL persist note
content verbatim, collapsing only empty or all-whitespace input to "no note"; it SHALL NOT trim
non-empty note content. Internal carriage returns and line feeds in a cell SHALL be retained.

#### Scenario: Leading whitespace survives to the exported cell

- **WHEN** a note ` +SUM(1,2) with bare\rcarriage return` (leading space, internal bare CR) is created
  and then exported
- **THEN** the exported note cell is `"' +SUM(1,2) with bare\rcarriage return"` — the `'` guard is
  prepended ahead of the preserved leading space, and the internal carriage return is retained.

#### Scenario: Empty or whitespace-only note is stored as no note

- **WHEN** a note is empty or contains only whitespace
- **THEN** it is stored as "no note" (none) rather than as a whitespace string
- **AND** non-empty notes are stored and exported with their whitespace intact.

### Requirement: CSV delimiter, quote, and newline escaping is applied after neutralization

After formula neutralization, the exporter SHALL wrap any cell that contains a comma, double-quote,
carriage return, or line feed in double-quotes, and SHALL escape embedded double-quotes by doubling
them. A cell forced to neutralize (`'`-guarded) and also containing such a character SHALL be both
guarded and quoted. The column header order SHALL remain stable.

#### Scenario: Delimiter and embedded quote are escaped

- **WHEN** a cell contains a comma (e.g. `A, Inc`) or an embedded double-quote (e.g. `said "hi"`)
- **THEN** the cell is wrapped in double-quotes and embedded double-quotes are doubled
  (`"A, Inc"`, `"said ""hi"""`).

#### Scenario: A neutralized cell containing a control character is quoted

- **WHEN** a neutralized cell also contains a carriage return, line feed, or comma
- **THEN** the cell is emitted as a quoted field (e.g. `"' +SUM(1,2) with bare\rcarriage return"`),
  preserving the control character inside the quotes.

### Requirement: CSV export stays scoped to reviewed summary fields and never expands to raw or secret data

The CSV export SHALL emit only the reviewed manual-summary columns
(`date, project, start_time, end_time, duration_minutes, note, total_duration_hours`) and SHALL NOT
add raw captured activity, app/window logs, AI prompt or response text, terminal command bodies, or
secret-shaped values. This hardening change SHALL NOT broaden the exported field set.

#### Scenario: Export contains no raw activity, prompt, or secret fields

- **WHEN** a report is exported to CSV
- **THEN** the file contains only the reviewed summary columns
- **AND** no raw activity log, AI prompt/response, command body, or credential value is present.
