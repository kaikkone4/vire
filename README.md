# Vire

Vire v0.1 is a local-only Tauri v2 desktop time tracker. It supports projects, manual time entries, simple summaries, and CSV export. Automatic activity capture is intentionally deferred.

## Run locally

Prerequisites: macOS with Rust, Node.js/npm, and Tauri v2 system dependencies installed.

```sh
npm install
npm run tauri:dev
```

## Build

```sh
npm install
npm run tauri:build
```

## Tests

```sh
npm test
npm run test:frontend
```

The test suite covers project create/update/archive persistence and active filtering, manual entry create/update/delete and validation, summary totals, CSV filtering/escaping/formula neutralization, text length validation, archived-project historical edits, inverted date-range rejection, SQLite persistence across reopen, and frontend HTML escaping for adversarial payloads.

## Manual verification

1. Launch with `npm run tauri:dev` and confirm the sidebar includes Today, Projects, Manual Entry, Reports, and Settings.
2. Confirm the Today/Settings capture status says `Manual Mode / Capture deferred` and there are no automatic capture controls.
3. Create a project, edit it, then archive it. Confirm archived projects disappear from active entry pickers but remain visible in all-project/report history.
4. Add, edit, and delete a manual entry; deletion requires confirmation.
5. Restart the app and confirm projects and entries persist.
6. In Reports, choose a date range/project filter and export CSV to a selected local destination. Confirm the file contains only matching manual entries.

## Privacy status

Vire v0.1 stores data in a local SQLite database on this Mac. It has no accounts, cloud sync, hosted API, or network feature. It does not capture active windows, idle state, screenshots, keystrokes, browser contents, full URLs, terminal commands, screen pixels, or file contents.
