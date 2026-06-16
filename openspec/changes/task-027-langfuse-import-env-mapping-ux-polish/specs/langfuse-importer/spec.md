# Spec delta — langfuse-importer

These requirements extend the TASK-019/020 importer so a manual import returns an **understandable
result**, the importer **tolerates the current Langfuse payload shape**, import runs **automatically**
in addition to manually, and Vire can **discover** the environments that exist in the source. They
strengthen — never relax — the existing absence-≠-zero and no-secret-exposure contracts and add no new
state to the ten-state health taxonomy.

## ADDED Requirements

### Requirement: A manual import returns a secret-free, understandable result

The manual import command SHALL return a result that explains what the import did, including, per
environment and in total: the number of traces seen, the number newly imported (unique), the number
suppressed as duplicates, the number skipped due to an unrecognized shape, and the resulting health
state. An import that finds or imports nothing SHALL produce an explicit, human-readable result rather
than an empty or silent one. The result SHALL NOT contain any credential, raw API response body, or
trace prompt/session content; only counts, health states, and secret-free warning strings.

#### Scenario: An empty import is explained, not silent

- **WHEN** a manual import finds no matching traces for the configured environments
- **THEN** the result states that zero traces were imported and shows the per-environment health
- **AND** the user sees an explanation (e.g. no traces in this environment/window) rather than a blank
  panel.

#### Scenario: A partial import reports counts and skips

- **WHEN** a manual import imports some traces and skips others for shape reasons
- **THEN** the result reports the imported, duplicate, and skipped counts per environment and in total.

#### Scenario: The import result carries no secrets

- **WHEN** the import result, its counts, or any warning string is produced
- **THEN** no API key, local stack secret, raw response body, or trace prompt/session content appears in
  it.

### Requirement: The importer tolerates the current Langfuse usage/cost payload shape

The importer SHALL read token usage and cost from the field locations the current Langfuse public API
returns, in addition to the previously supported locations, so that present usage and cost are captured
rather than withheld. A field that is genuinely absent in every supported location SHALL remain absent
(`null`), never coerced to zero. A genuinely unrecognized shape SHALL still degrade to the
`schema_changed` health state with the affected count surfaced, never a silent zero.

#### Scenario: Current-shape usage and cost are captured

- **WHEN** a generation observation reports token usage and cost in the current Langfuse field locations
- **THEN** the importer records the present usage and cost values
- **AND** does not degrade the trace to `schema_changed` solely because the legacy field names are
  absent.

#### Scenario: Absent usage stays absent, not zero

- **WHEN** a generation observation carries no token usage or cost in any supported location
- **THEN** the importer records absence (no value), never a numeric zero
- **AND** surfaces the affected trace as `schema_changed`, counted in the import result.

### Requirement: Vire imports automatically in addition to manually

The app SHALL run an import automatically on application startup and on a periodic background interval,
in addition to the explicit manual import action, which SHALL remain available and unchanged. Automatic
and manual imports SHALL NOT run concurrently against the local store. Automatic import SHALL honor the
integration enable/disable switch and the loopback boundary identically to manual import, SHALL run off
the user-interface thread, and SHALL never block the UI. A failed or unavailable automatic import SHALL
resolve to an existing health state, never to zero AI usage or cost.

#### Scenario: Startup and periodic import keep evidence current

- **WHEN** the app starts and later while it runs
- **THEN** Vire imports AI evidence automatically without the user clicking import
- **AND** the manual import action still works and is unchanged.

#### Scenario: Automatic and manual imports do not overlap

- **WHEN** a periodic import is due while another import is already running
- **THEN** the imports are serialized so no two run concurrently against the local store.

#### Scenario: Automatic import respects the disabled switch

- **WHEN** the integration is disabled
- **THEN** no automatic import runs and no health probe fires
- **AND** the source state remains an explicit disabled state, never zero usage or cost.

### Requirement: Vire discovers the environments present in the source

The app SHALL discover the set of Langfuse environments that actually exist in the configured source,
rather than requiring the user to type the environment list by hand. Discovery SHALL be read-only and
SHALL stay within the existing URL allowlist and loopback boundary (no new host, no path outside the
public API root). Discovered environments SHALL be surfaced to the user for selection. The hand-entered
environment list MAY remain available as an advanced fallback, and the existing default SHALL be
unchanged.

#### Scenario: Discovered environments are offered for selection

- **WHEN** the source contains traces in one or more environments
- **THEN** Vire discovers those environment names and offers them to the user to select
- **AND** the user does not have to type the environment list manually.

#### Scenario: Discovery preserves the network boundary

- **WHEN** Vire discovers environments
- **THEN** every request stays under the public API root on the configured host
- **AND** a `local` source still requires a loopback host, the same as for trace import.
