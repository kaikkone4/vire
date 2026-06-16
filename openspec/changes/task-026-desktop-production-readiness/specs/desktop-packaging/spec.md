# Spec delta — desktop-packaging

New capability: how Vire is packaged and run as a finished macOS desktop app — a production build with
no dev-server dependency, a real application icon, install/run documentation, and a documented release
compatibility / rollback posture for the downstream release artifact (SW-6).

## ADDED Requirements

### Requirement: The app builds into a self-contained macOS bundle that runs without a dev server

The build SHALL produce a self-contained macOS application bundle (`.app`, and a `.dmg` where the
toolchain supports it) from `npm run tauri:build`, loading its frontend from the bundled production
assets (`frontendDist`) rather than the Vite dev server. The packaged app SHALL launch and operate
**without** running `npm run dev` / `npm run tauri:dev` or any local dev server at runtime.

#### Scenario: Built app launches with no dev server

- **WHEN** `npm run tauri:build` completes and the resulting `Vire.app` is launched directly
- **THEN** the app starts and renders its UI from the bundled production assets
- **AND** it requires no Vite dev server or `tauri:dev` process to be running.

#### Scenario: AI import still depends only on the local Langfuse stack

- **WHEN** the packaged app performs a Langfuse import
- **THEN** it uses the in-app Langfuse settings and the local Langfuse stack on loopback
- **AND** a down stack is reported as unavailable, never as zero AI usage or cost.

### Requirement: The app shows a Vire icon in the Dock and app switcher

The packaged app SHALL ship a macOS application icon (generated into the Tauri icon set and referenced
by `bundle.icon`) so it appears as a Vire icon in the Dock and app switcher while running. If no
branded asset exists, a simple temporary Vire mark MAY be used, and the documentation SHALL describe
the **replacement path** so a branded asset can be swapped in without code changes.

#### Scenario: Running app shows its own icon

- **WHEN** the packaged app is running
- **THEN** the Dock and app switcher show the Vire application icon, not the generic default
- **AND** the documentation describes how to replace the icon with a branded asset (drop the source
  image at the documented path, regenerate the icon set, rebuild).

### Requirement: Installing and running the packaged app is documented

The documentation SHALL describe how to build the app, where the artifact is produced
(`src-tauri/target/release/bundle/`), how to install and run it, and that no dev server is required at
runtime. It SHALL note that the app reads its Langfuse configuration from in-app settings and that AI
import additionally requires the local Langfuse stack to be running.

#### Scenario: A reader can build, install, and run from the docs alone

- **WHEN** a local user follows the install/run documentation
- **THEN** they can build the bundle, locate the artifact, install it, and launch the app
- **AND** the docs state that no dev server is needed at runtime and where Langfuse configuration comes
  from.

### Requirement: Release compatibility and rollback are represented for the release artifact

The change SHALL document the packaged build's data and secret compatibility and its rollback path, so
the downstream release artifact (SW-6) can represent them. The packaged app SHALL reuse the same local
SQLite database and app-scoped Keychain entries as prior builds; the new Langfuse settings SHALL be
additive (idempotent initialization, no destructive migration); and reverting to a prior build SHALL
read the same database without data loss.

#### Scenario: Rollback to a prior build loses no data

- **WHEN** a user reverts from this build to the immediately prior build
- **THEN** the prior build opens the same local SQLite database and ignores unknown additive settings
  rows
- **AND** no data is lost and no destructive migration is required; the prior build falls back to env
  for Langfuse configuration.
