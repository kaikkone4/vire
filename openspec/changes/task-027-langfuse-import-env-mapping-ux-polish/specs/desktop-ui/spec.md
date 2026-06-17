# Spec delta — desktop-ui

Desktop chrome polish for the packaged macOS app: stop drawing fake window controls when the native
titlebar already provides them, and size the app icon to sit at parity with other Dock icons. No
data-model or behavioural change; presentation only.

## ADDED Requirements

### Requirement: The app does not draw fake macOS window controls

The app SHALL NOT render decorative imitation macOS window controls (close / minimize / zoom dots) in
its own content area while the native macOS titlebar with the real controls is present. The window's
real controls SHALL remain the only close / minimize / zoom affordance.

#### Scenario: Only native window controls are shown

- **WHEN** the packaged app window is displayed with the native macOS titlebar
- **THEN** no imitation traffic-light buttons are drawn in the app's content
- **AND** the user uses the real native controls to close, minimize, or zoom the window.

#### Scenario: Removing the fakes leaves the titlebar coherent

- **WHEN** the imitation controls are removed
- **THEN** the in-app titlebar (brand and version text) remains correctly aligned with no empty gap.

### Requirement: The Dock icon sits at visual parity with other Dock icons

The app icon SHALL include transparent safe-area padding so that, in the macOS Dock and app switcher,
the icon's visible mark is sized comparably to other apps' icons rather than appearing oversized. The
safe-area requirement SHALL be documented so a later branded icon asset inherits it.

#### Scenario: Icon is not oversized in the Dock

- **WHEN** the packaged app is shown in the Dock
- **THEN** the Vire icon's mark is sized comparably to neighbouring app icons
- **AND** it does not visibly overflow relative to other icons.

#### Scenario: Safe-area requirement is documented for the branded asset

- **WHEN** the branded icon asset is later supplied
- **THEN** the documented safe-area/padding requirement is available so the branded icon keeps Dock
  parity
- **AND** no change to application code is required to adopt the branded asset.
