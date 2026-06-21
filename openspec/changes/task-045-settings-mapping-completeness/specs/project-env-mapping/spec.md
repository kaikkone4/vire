# Spec delta — project-env-mapping

Refines the existing `project-env-mapping` capability (added in TASK-027, UX-corrected in TASK-030) so
the mapping surface lists **every environment that needs a project mapping**, not only the environments
that the recent discovery scan happened to surface. This closes the gap where backfilled/imported
environments older than the discovery window had imported AI evidence but no mapping row, so they could
never be mapped (and their evidence stayed untrackable). The Vire-authoritative, suggestion-first
contract (DEC-001 / DEC-006) is unchanged; only the completeness of the environment list changes.

## MODIFIED Requirements

### Requirement: An environment can be mapped to a Vire project

The app SHALL let the user map a Langfuse environment to an existing Vire project, and SHALL persist
that mapping locally. Imported AI evidence for a mapped environment SHALL be associated with its mapped
project. The mapping SHALL be Vire-authoritative: the environment is an external key and the Vire
project record is the source of truth. Changing or removing a mapping SHALL NOT rewrite or delete
previously imported evidence rows.

The set of environments the app offers for mapping SHALL include **every environment that needs a
project mapping** — specifically the union of: environments discovered in the source, environments that
have imported AI evidence, and environments that already have a mapping. An environment that has
imported evidence SHALL appear in the mapping surface even when it was not seen by the most recent
discovery scan (e.g. its traces predate the discovery look-back). An environment that already has a
mapping SHALL remain visible in the mapping surface (with its mapping shown and clearable) even after it
no longer appears in recent discovery. The surface SHALL contain each environment exactly once and SHALL
render in a deterministic order.

#### Scenario: User maps an environment to a project

- **WHEN** the user maps an environment shown in the mapping surface to a Vire project
- **THEN** the app persists the environment → project mapping locally
- **AND** subsequent reads associate that environment's imported evidence with the mapped project.

#### Scenario: An imported environment can be mapped even if not recently discovered

- **WHEN** an environment has imported AI evidence but was not surfaced by the most recent discovery scan
- **THEN** that environment appears in the mapping surface as unmappable-until-mapped (unmapped)
- **AND** the user can map it to a project (or create-and-map one) the same as any discovered environment.

#### Scenario: A mapped environment stays visible after it ages out of discovery

- **WHEN** an environment that already has a project mapping no longer appears in recent discovery
- **THEN** the mapping surface still shows that environment with its current mapping
- **AND** the user can still clear or change that mapping.

#### Scenario: Each environment appears once

- **WHEN** an environment is present in more than one source (discovered, has evidence, and/or mapped)
- **THEN** the mapping surface lists it exactly once with its correct mapped/unmapped state.

#### Scenario: Changing a mapping does not destroy evidence

- **WHEN** the user changes or clears an environment's project mapping
- **THEN** the previously imported evidence rows are preserved
- **AND** only the environment → project association changes.

### Requirement: An unmapped environment suggests creating a project, never auto-creates one

For an environment in the mapping surface that has no project mapping, the app SHALL surface a suggestion
to create a Vire project for that environment. A project SHALL be created only by an explicit user
action; the app SHALL NOT auto-create a project or auto-map an environment silently. When the user
accepts the suggestion, the app SHALL create the project through the normal project-creation path and
then record the environment → project mapping.

The app SHALL collect the new project's name through an in-app input affordance rendered inside the
app's own window (an inline field per environment row). It SHALL NOT depend on a native `window.prompt()`
text-input dialog. This affordance SHALL be available for every environment in the mapping surface,
including environments surfaced because they have imported evidence rather than recent discovery.

#### Scenario: Unmapped environment offers a create-project suggestion

- **WHEN** the mapping surface shows an environment that is not mapped to any project
- **THEN** the app shows an inline "Create & map" affordance for that environment
- **AND** no project is created until the user explicitly accepts.

#### Scenario: Accepting the suggestion creates and maps in one step

- **WHEN** the user accepts the suggestion to create a project for an environment and supplies a
  non-empty name through the in-app input
- **THEN** the app creates the project via the normal creation path
- **AND** records the environment → project mapping for it.

#### Scenario: Mapping data carries no secrets

- **WHEN** the mapping list, the mapping surface (discovered ∪ has-evidence ∪ mapped), or a suggestion
  is produced
- **THEN** it contains only environment names, project references, and mapping state
- **AND** no credential or secret material appears in it.
