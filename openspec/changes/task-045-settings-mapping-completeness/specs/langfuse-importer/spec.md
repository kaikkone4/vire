# Spec delta — langfuse-importer

Refines the discovery requirement (added in TASK-027) so environment discovery covers the **configured
import range** rather than a fixed short look-back. A backfill imports AI evidence across the whole
configured range; discovery must enumerate environments across that same range so the discovered set
does not lag the imported set. Discovery's read-only, name-only, allowlisted, loopback-gated, and
bounded posture is unchanged.

## MODIFIED Requirements

### Requirement: Vire discovers the environments present in the source

The app SHALL discover the set of Langfuse environments that actually exist in the configured source,
rather than requiring the user to type the environment list by hand. Discovery SHALL be read-only and
SHALL stay within the existing URL allowlist and loopback boundary (no new host, no path outside the
public API root). Discovered environments SHALL be surfaced to the user for selection. The hand-entered
environment list MAY remain available as an advanced fallback, and the existing default SHALL be
unchanged.

Discovery SHALL scan the source over a window whose floor is the **configured import-range floor** (the
same floor the import resolves), so an environment whose traces fall within the configured range — but
outside any shorter recent window — is still discovered. Discovery SHALL enumerate only environment
**names** (never trace content) and SHALL remain bounded by the existing pagination backstop, so an
`all`-history range degrades to "as many environments as the backstop allows" and never produces wrong
data or an unbounded scan. A discovery failure SHALL remain best-effort and SHALL NOT fail an otherwise
successful import.

#### Scenario: Discovery covers the configured import range

- **WHEN** the configured import range reaches back beyond a short recent window (e.g. 30/90 days or all
  history) and the source has traces in environments active only in that older span
- **THEN** Vire discovers those environment names as part of the import
- **AND** they are offered to the user for selection and for mapping.

#### Scenario: Discovered environments are offered for selection

- **WHEN** the source contains traces in one or more environments
- **THEN** Vire discovers those environment names and offers them to the user to select
- **AND** the user does not have to type the environment list manually.

#### Scenario: Discovery stays bounded for a wide range

- **WHEN** discovery runs over an `all`-history range
- **THEN** the scan is bounded by the existing pagination backstop
- **AND** discovery returns the environment names found within that bound rather than scanning without limit.

#### Scenario: Discovery preserves the network boundary

- **WHEN** Vire discovers environments
- **THEN** every request stays under the public API root on the configured host
- **AND** a `local` source still requires a loopback host, the same as for trace import.
