# Spec delta — implementation-path-decision

## ADDED Requirements

### Requirement: The implementation path is decided from spike evidence before schema build

The decision SHALL select one implementation path from the three BA-mandated options
(Tauri+native-helper reuse, Swift/AppKit-first, ActivityWatch import/reference) using a weighted
comparison grounded in the completed TASK-001, TASK-002, and TASK-007 spikes, and SHALL be recorded
before the TASK-004 schema/feature build starts.

#### Scenario: Options are compared against spike-derived criteria

- **WHEN** the implementation path is decided
- **THEN** all three options are scored against criteria drawn from the spikes (reuse value of
  validated assets, SEC-002 Langfuse-importer fit, added complexity, privacy-boundary strength, and
  EPIC-006 extension seams)
- **AND** capture API access and permission burden are treated as non-differentiating gates because
  TASK-002 found the capture APIs equally reachable from a native helper or a Swift-first app.

#### Scenario: A single path is recorded with rationale before TASK-004

- **WHEN** the comparison concludes
- **THEN** exactly one path is recorded as an ADR (DEC-019) with rationale traced to the TASK-001/
  002/007 evidence and to DEC-008/DEC-009/DEC-017/DEC-018
- **AND** the decision exists before the TASK-004 schema/lifecycle migrations begin.

### Requirement: Every named technology in the decision is verified

The decision SHALL verify each named technology it depends on against official documentation or an
authoritative API reference, and SHALL mark each as verified with a source URL or as an assumption
with a named spike/packaging follow-up.

#### Scenario: Technology register records verification status

- **WHEN** the decision relies on a named technology (Tauri v2 sidecar/`externalBin`, Tauri v2 IPC,
  Tauri v2 HTTP client, macOS TCC/codesigning, Swift/AppKit, ActivityWatch, SQLite)
- **THEN** the technology register records each as **verified (source URL)** or **assumption +
  follow-up**
- **AND** any unverified, decision-load-bearing claim (e.g. nested-binary codesign/notarization of a
  sidecar) is explicitly marked as an assumption with a downstream spike that must confirm it.

### Requirement: Downstream architecture constraints are fixed by the decision

The decision SHALL fix the binding architecture constraints that TASK-004, TASK-005, TASK-006, and
the TASK-007 MVP inherit, consistent with the L2 privacy posture and DEC-017/DEC-018.

#### Scenario: Capture and credential boundaries are bound to the helper and core

- **WHEN** the decision is recorded
- **THEN** the native capture helper (not the webview) holds the Accessibility grant and is the
  TCC-trusted binary
- **AND** raw window titles flow helper → Rust core only, never into the webview or any network path,
  and the Langfuse importer is a read-only Rust-core REST client scoped to the configured Langfuse
  base URL with no raw-activity egress (SEC-002, DEC-018).

#### Scenario: Reuse stays evidence-driven and excludes the legacy capture surface

- **WHEN** reuse of the existing repo is selected
- **THEN** only evidence-classified `reuse-as-is` / `reuse-with-changes` assets are carried forward
  (locked CSP, capabilities, CSV escaping/formula-neutralization, validation/error patterns)
- **AND** the legacy generic-tracker CRUD surface (`time_entries`, manual-entry view, stopwatch) is
  not re-admitted as a capture path, and its migrate-vs-retire fate is deferred to TASK-004.

### Requirement: The decision task ships no product runtime and no MVP

The decision SHALL remain a decision/spike. It SHALL NOT build the SQLite schema, the capture
adapter, the durable Langfuse importer, or the runtime observer, and SHALL NOT migrate or wipe the
legacy manual-tracker surface.

#### Scenario: No product-runtime change is made by the decision

- **WHEN** the decision is produced
- **THEN** no file under `src/`, `src-tauri/src/`, or `observability/` is created, modified, or
  deleted and no build target is added
- **AND** the schema (TASK-004), capture adapter (TASK-005), durable importer (TASK-007 MVP), and
  runtime observer (TASK-006) remain unbuilt, with the legacy surface left reference-only.
