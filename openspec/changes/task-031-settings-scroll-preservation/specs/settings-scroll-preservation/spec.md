# Spec delta — settings-scroll-preservation

Adds a new capability: the app preserves the user's vertical scroll position within a view when that view
re-renders in response to an action, and resets to the top only when the user navigates to a different
view. The capability is observed and verified in **Settings** (the longest, most control-dense view,
where the loss of scroll position is felt), but the contract is expressed in terms of the shared view
render so it holds for every view that re-renders.

## ADDED Requirements

### Requirement: An in-view action preserves scroll position

The app SHALL keep the current view's scroll position when an action re-renders that same view in place
(for example saving settings, testing the connection, saving the import range, importing, mapping an
environment, or creating and mapping a project — all in Settings), rather than jumping the viewport to
the top of the view.

The app SHALL reset the scroll position to the top only when the user navigates to a **different** view.
Re-rendering the same view in place SHALL NOT reset the scroll position.

#### Scenario: A Settings control below the fold keeps its place

- **WHEN** the user scrolls down in Settings and presses a control that re-renders the view (e.g. Save
  settings, Test connection, Save range, Import now, Backfill now, Map, Clear mapping, Create & map)
- **THEN** after the view re-renders the viewport remains at substantially the same scroll position
- **AND** the viewport does NOT jump back to the top of Settings.

#### Scenario: Navigating to another view starts at the top

- **WHEN** the user selects a different view from the navigation
- **THEN** the newly shown view is displayed scrolled to its top.

#### Scenario: First render starts at the top

- **WHEN** the app renders a view for the first time, with no previously rendered view to restore from
- **THEN** the view is displayed scrolled to its top, with no error.

### Requirement: Mapping-panel help text matches the create control's label

The environment → project mapping panel's explanatory help text SHALL refer to the create affordance by
the same label the control actually displays, so the instructions and the button agree.

#### Scenario: Help text and button label agree

- **WHEN** the user reads the environment → project mapping panel in Settings
- **THEN** the help text refers to the create-and-map affordance using the same wording shown on the
  control (**Create & map**), not a different phrase.
