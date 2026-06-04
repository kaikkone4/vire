# Spec delta — repo-path-assessment

## ADDED Requirements

### Requirement: Salvage/reuse inventory deliverable

The assessment SHALL produce a salvage/reuse inventory covering the current Tauri shell, Rust
backend, SQLite layer, project/manual-entry domain, summary/export, tests, observability tooling,
and privacy posture. Each inventoried asset SHALL be classified as one of `reuse-as-is`,
`reuse-with-changes`, `reference-only`, or `retire/replace`, with the architectural rationale and
the referenced BA artifact / EPIC / SEC control.

#### Scenario: Every inventoried area is classified

- **WHEN** the assessment artifact is produced
- **THEN** each of the named areas (shell, backend, SQLite, domain, summary/export, tests,
  observability tooling, privacy posture) appears in the inventory
- **AND** each listed asset carries exactly one classification and a rationale referencing a BA
  artifact, EPIC, or SEC control.

#### Scenario: Data model coverage is mapped

- **WHEN** the SQLite layer is inventoried
- **THEN** the existing tables are mapped against the BA evidence data model entities
- **AND** entities that are present, partially present, or absent are explicitly recorded.

### Requirement: No assumption of wipe or reuse

The assessment SHALL NOT assume, recommend as final, or implement either a repo wipe or a repo
reuse. Both replacement and reuse SHALL remain open decisions handed to TASK-003.

#### Scenario: Assessment leaves the path decision open

- **WHEN** the assessment artifact reaches its conclusion
- **THEN** it records evidence for and against reuse without selecting a final implementation path
- **AND** it explicitly defers the implementation-path decision to TASK-003.

#### Scenario: No source or schema is modified

- **WHEN** the assessment change is applied
- **THEN** no file under `src/`, `src-tauri/`, or `observability/` is created, modified, or deleted
- **AND** no SQLite schema migration or data deletion is performed.

### Requirement: Privacy and security guardrails preserved

The assessment and any artifact it produces SHALL preserve the BA privacy/security guardrails:
local-only raw evidence, no SaaS/cloud sync, no raw activity egress, a positive collection
allowlist posture, no credentials in logs/SQLite/exports/tests, reviewed-summary export defaults,
and the DEC-017 boundary (Langfuse import as primary AI evidence; local runtime observation for
reconciliation/health only; no new pi/Claude adapter in MVP).

#### Scenario: No secrets in produced artifacts

- **WHEN** the assessment artifact is reviewed before completion
- **THEN** it contains no Langfuse credentials, secret-shaped values, raw window/app titles, prompt
  or response text, terminal command bodies, or environment dumps.

#### Scenario: APP-005 control coverage is mapped

- **WHEN** the privacy/security posture is inventoried
- **THEN** existing coverage and gaps are recorded against SEC-001, SEC-002, SEC-003, SEC-005,
  SEC-006, and SEC-008
- **AND** the absence of an outbound network client and of SBOM/signing/notarization tooling is
  recorded as a known gap for downstream tasks.

#### Scenario: DEC-017 tension recorded without resolution

- **WHEN** the observability tooling is inventoried
- **THEN** the assessment records that `pi-observe` is a trace emitter while DEC-017 mandates a
  Langfuse importer as the primary AI evidence path
- **AND** the assessment routes this tension to TASK-003/TASK-006/TASK-007 as a decision input
  rather than resolving it.
