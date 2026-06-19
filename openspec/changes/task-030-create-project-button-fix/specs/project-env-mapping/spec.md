# Spec delta — project-env-mapping

Refines the existing `project-env-mapping` capability (added in TASK-027) so the "create a project for an
unmapped environment" suggestion is accepted through an **in-app** input rather than a native browser
text-input dialog. The behavioural contract (explicit, human-approved, create-then-map; DEC-006) is
unchanged — only the affordance that the macOS WKWebView cannot render is corrected.

## MODIFIED Requirements

### Requirement: An unmapped environment suggests creating a project, never auto-creates one

For a discovered environment that has no project mapping, the app SHALL surface a suggestion to create a
Vire project for that environment. A project SHALL be created only by an explicit user action; the app
SHALL NOT auto-create a project or auto-map an environment silently. When the user accepts the
suggestion, the app SHALL create the project through the normal project-creation path and then record
the environment → project mapping.

The app SHALL collect the new project's name through an **in-app input affordance** rendered inside the
app's own window (e.g. an inline field or in-app form). It SHALL NOT depend on a native
`window.prompt()` text-input dialog, which is not available in the app's macOS webview and silently
returns no value. Accepting with a non-empty name SHALL create and map the project; accepting with an
empty or whitespace-only name SHALL surface a validation message and create nothing.

#### Scenario: Unmapped environment offers a create-project suggestion

- **WHEN** Vire discovers an environment that is not mapped to any project
- **THEN** the app shows a suggestion to create a project for that environment
- **AND** no project is created until the user explicitly accepts.

#### Scenario: Accepting the suggestion creates and maps in one step

- **WHEN** the user accepts the suggestion to create a project for an environment and supplies a
  non-empty name through the in-app input
- **THEN** the app creates the project via the normal creation path
- **AND** records the environment → project mapping for it.

#### Scenario: The create affordance works inside the packaged macOS app

- **WHEN** the user activates "Create project for &lt;env&gt;" in the packaged macOS app
- **THEN** an in-app input for the project name is shown within the app window
- **AND** the action does not rely on a native `window.prompt()` dialog
- **AND** confirming with a valid name creates and maps the project without a silent no-op.

#### Scenario: Empty name is rejected

- **WHEN** the user accepts the create-project suggestion with an empty or whitespace-only name
- **THEN** the app creates no project and records no mapping
- **AND** surfaces a validation message via the app's normal error surface.

#### Scenario: Mapping data carries no secrets

- **WHEN** the mapping list, a discovered-environment list, or a create suggestion is produced
- **THEN** it contains only environment names, project references, and mapping state
- **AND** no credential or secret material appears in it.
