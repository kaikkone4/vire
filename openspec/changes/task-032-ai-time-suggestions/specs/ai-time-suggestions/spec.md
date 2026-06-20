# Spec delta — ai-time-suggestions

Adds the `ai-time-suggestions` capability: turn imported Langfuse AI evidence
(`langfuse_ai_evidence`, made correct by TASK-029) into **user-reviewable time-entry suggestions**.
Realizes the architecture plan's **Suggestion Engine** and **Review & Approval UI** components
(`03_architecture_plan.md` §3, §6). The contract is suggestion-first and human-approved (DEC-006), keeps
AI evidence separate from human time (DEC-003), never treats absence as zero (DEC-004 posture), and
exposes no secrets (SEC-012, extends SEC-010/011). Vire remains authoritative; Langfuse is read only via
the existing local store (DEC-001/DEC-017) — the engine performs no network access.

## ADDED Requirements

### Requirement: Imported AI evidence is aggregated into suggested time blocks

The app SHALL aggregate `langfuse_ai_evidence` that is mapped to a Vire project (via the
environment→project mapping) into suggested time blocks, grouped by project and local date and clustered
into contiguous blocks by a configurable inter-trace gap (default 30 minutes). Each suggestion SHALL
carry its time span, suggested duration, aggregate token and cost figures with currency, trace and
session counts, worst health state, a confidence bucket, a source identifying the originating
environment(s), and a human-readable reason. Generation SHALL be idempotent: re-generating replaces only
undecided (`pending`) suggestions and SHALL NOT regenerate or duplicate suggestions the user has already
accepted or dismissed. Generation SHALL NOT contact Langfuse or any network host.

#### Scenario: Mapped evidence produces grouped suggestions

- **WHEN** AI evidence exists for an environment mapped to a project and suggestions are generated
- **THEN** the app produces suggested time blocks grouped by project and date
- **AND** each block reports its span, suggested duration, tokens, cost, trace/session counts, health,
  confidence, source, and reason.

#### Scenario: Regeneration preserves user decisions

- **WHEN** suggestions are regenerated after some were accepted or dismissed
- **THEN** previously accepted and dismissed suggestions are preserved unchanged
- **AND** only undecided suggestions are recomputed, with no duplicate created for a decided block.

### Requirement: Suggestions are reviewed and accepted explicitly; nothing is auto-posted

The app SHALL NOT create a time entry automatically from imported evidence or from suggestion
generation. A time entry SHALL be created only when the user explicitly accepts a suggestion. On accept
the app SHALL create exactly one time entry tagged as AI-origin, optionally applying user edits (project,
date, start, end, note) supplied at accept time, and mark the suggestion accepted. The user SHALL be able
to dismiss a suggestion, which records the decision and creates no time entry.

#### Scenario: Accepting a suggestion creates one AI-origin entry

- **WHEN** the user accepts a suggestion
- **THEN** the app creates exactly one time entry marked AI-origin, carrying the block's provenance
- **AND** marks the suggestion accepted and links it to the created entry
- **AND** the entry appears in the normal project/time-entry surfaces.

#### Scenario: Import and generation never post time

- **WHEN** evidence is imported or suggestions are generated or regenerated
- **THEN** no time entry is created as a side effect
- **AND** time entries appear only after an explicit accept.

#### Scenario: Dismissing writes nothing

- **WHEN** the user dismisses a suggestion
- **THEN** the suggestion is recorded as dismissed
- **AND** no time entry is created.

### Requirement: AI-suggested time is kept separate from human-entered time

The app SHALL distinguish AI-origin time entries from manually entered (human) time entries. Project
summaries and CSV export SHALL report AI-origin duration distinctly from human duration and SHALL NOT
silently sum AI time into a billable human total. AI cost SHALL be reported as its own figure, never
folded into a human time total.

#### Scenario: Summaries separate AI and human totals

- **WHEN** a project summary or export is produced for a range containing both manual and AI-origin
  entries
- **THEN** the human-entered duration and the AI-origin duration are reported as distinct figures
- **AND** the AI duration is not added into the billable human total.

### Requirement: Absence is never rendered as zero

The app SHALL NOT represent missing AI evidence as zero. A project/date group whose traces carry no
usable timestamps SHALL yield a suggestion with unknown duration flagged for manual completion, never a
zero-minute block. Evidence belonging to an environment with no project mapping SHALL be surfaced as a
prompt to map that environment, never silently dropped or counted as zero. Unknown token or cost figures
SHALL be shown as unknown, not as zero. Degraded source health SHALL be carried onto the affected
suggestions and surfaced, not converted into absence-as-zero.

#### Scenario: Untimed evidence yields an unknown-duration suggestion

- **WHEN** a project/date group's evidence has no usable timestamps
- **THEN** the suggestion shows an unknown duration flagged for manual completion
- **AND** no zero-minute block is produced
- **AND** accepting it requires the user to supply a start and end time.

#### Scenario: Unmapped evidence prompts mapping

- **WHEN** AI evidence exists for an environment that is not mapped to any project
- **THEN** the app surfaces a prompt to map that environment (with its trace count)
- **AND** does not drop the evidence or report it as zero.

### Requirement: Suggestions and the review surface carry no secrets

Every suggestion, the review/accept surface, and the provenance recorded on an accepted entry SHALL
contain only environment names, project references, time bounds, aggregate token/cost numbers, trace and
session counts, health, confidence, and a secret-free reason. They SHALL NOT contain credentials, raw
trace payloads, deserialization error strings, raw session-id values, or any prompt, session, or
metadata content.

#### Scenario: Suggestion output is secret-free

- **WHEN** a suggestion list, the review surface, or an accepted entry's provenance is produced
- **THEN** it contains only environment names, project references, time bounds, aggregate numbers,
  counts, health, confidence, and a secret-free reason
- **AND** no credential, raw payload, raw session id, or prompt/session/metadata content appears in it.
</content>
