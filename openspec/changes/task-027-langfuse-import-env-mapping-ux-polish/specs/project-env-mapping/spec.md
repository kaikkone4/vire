# Spec delta — project-env-mapping

A new capability: map each discovered Langfuse environment to a Vire project, and suggest creating a
project for an environment that has none. This realizes the architecture plan's environment-first
`PROJECT_MAPPING` (`03_architecture_plan.md:141,175`). The Vire project record remains the source of
truth (DEC-001); mapping and project creation are suggestion-first and human-approved (DEC-006).

## ADDED Requirements

### Requirement: An environment can be mapped to a Vire project

The app SHALL let the user map a discovered Langfuse environment to an existing Vire project, and SHALL
persist that mapping locally. Imported AI evidence for a mapped environment SHALL be associated with its
mapped project. The mapping SHALL be Vire-authoritative: the environment is an external key and the Vire
project record is the source of truth. Changing or removing a mapping SHALL NOT rewrite or delete
previously imported evidence rows.

#### Scenario: User maps an environment to a project

- **WHEN** the user maps a discovered environment to a Vire project
- **THEN** the app persists the environment → project mapping locally
- **AND** subsequent reads associate that environment's imported evidence with the mapped project.

#### Scenario: Changing a mapping does not destroy evidence

- **WHEN** the user changes or clears an environment's project mapping
- **THEN** the previously imported evidence rows are preserved
- **AND** only the environment → project association changes.

### Requirement: An unmapped environment suggests creating a project, never auto-creates one

For a discovered environment that has no project mapping, the app SHALL surface a suggestion to create a
Vire project for that environment. A project SHALL be created only by an explicit user action; the app
SHALL NOT auto-create a project or auto-map an environment silently. When the user accepts the
suggestion, the app SHALL create the project through the normal project-creation path and then record
the environment → project mapping.

#### Scenario: Unmapped environment offers a create-project suggestion

- **WHEN** Vire discovers an environment that is not mapped to any project
- **THEN** the app shows a suggestion to create a project for that environment
- **AND** no project is created until the user explicitly accepts.

#### Scenario: Accepting the suggestion creates and maps in one step

- **WHEN** the user accepts the suggestion to create a project for an environment
- **THEN** the app creates the project via the normal creation path
- **AND** records the environment → project mapping for it.

#### Scenario: Mapping data carries no secrets

- **WHEN** the mapping list, a discovered-environment list, or a suggestion is produced
- **THEN** it contains only environment names, project references, and mapping state
- **AND** no credential or secret material appears in it.
