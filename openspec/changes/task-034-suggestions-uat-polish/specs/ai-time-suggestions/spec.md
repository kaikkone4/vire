# Spec delta — ai-time-suggestions (TASK-034 UAT polish)

Refines the `ai-time-suggestions` capability added by TASK-032 after UAT. Three behavioral refinements,
all additive and within the existing capability: (1) a timed suggestion is always acceptable and never
stores a zero-duration entry (DEC-034); (2) AI cost is reported separately in summaries and CSV, not only
in the Suggestions view (completes DEC-003 for cost); (3) every "no suggestion" cause is explained with
an action path (extends the DEC-004 absence-≠-zero posture to UX). No secrets are added (SEC-012); the
engine remains pure-local (DEC-001/017).

Plus one **contract correction** (no code change): a post-merge code-to-spec check found TASK-032 worded
the inter-trace clustering gap as *"configurable … (default 30 minutes)"*, but the shipped engine uses a
**fixed** 30-minute gap (`engine.rs` `GAP_MINUTES`) with no config surface — none was ever built and no
UAT item asks for one. This delta corrects the contract to the fixed policy so spec and code agree; adding
real configurability is out of scope (net-new settings/persistence/UI, not UAT polish). See `design.md` §7.

> **Dependency:** these MODIFY the requirements introduced by TASK-032. TASK-032 must be merged and its
> `ai-time-suggestions` capability archived to `openspec/specs/` before this delta is applied/validated.

## MODIFIED Requirements

### Requirement: Imported AI evidence is aggregated into suggested time blocks

The app SHALL aggregate `langfuse_ai_evidence` that is mapped to a Vire project (via the
environment→project mapping) into suggested time blocks, grouped by project and local date and clustered
into contiguous blocks by a **fixed 30-minute inter-trace gap**: a new block opens when the next trace
starts more than 30 minutes after the running block's end. The gap is a fixed engine policy, not a
user-configurable setting. Each suggestion SHALL carry its time span, suggested duration, aggregate token
and cost figures with currency, trace and session counts, worst health state, a confidence bucket, a
source identifying the originating environment(s), and a human-readable reason. Generation SHALL be
idempotent: re-generating replaces only undecided (`pending`) suggestions and SHALL NOT regenerate or
duplicate suggestions the user has already accepted or dismissed. Generation SHALL NOT contact Langfuse or
any network host.

#### Scenario: Mapped evidence produces grouped suggestions

- **WHEN** AI evidence exists for an environment mapped to a project and suggestions are generated
- **THEN** the app produces suggested time blocks grouped by project and date
- **AND** each block reports its span, suggested duration, tokens, cost, trace/session counts, health,
  confidence, source, and reason.

#### Scenario: Clustering uses a fixed 30-minute gap

- **WHEN** consecutive mapped traces for a project on one date are clustered
- **THEN** traces no more than 30 minutes apart fall in the same block
- **AND** a gap of more than 30 minutes opens a new block
- **AND** the 30-minute gap is fixed, with no setting to change it.

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

A timed suggestion SHALL always be acceptable without manual time editing: when its start and end resolve
to the same clock minute, accepting it SHALL store a positive span equal to the suggestion's computed
duration (at least one minute) within the same local day. The app SHALL anchor that span on its start
(end = start + duration) except when the start resolves to the final minute of the local day, in which
case it SHALL instead anchor on the end (end = the final minute, start = that minute minus the computed
duration), so the stored span never crosses into the next day. The app SHALL NEVER store a zero- or
negative-duration time entry from an accept, including via direct command invocation. An accepted
suggestion's AI cost figure (amount and currency when known) SHALL be carried onto the created AI-origin
entry.

#### Scenario: Accepting a same-minute suggestion stores a non-zero span

- **WHEN** the user accepts a timed suggestion whose start and end fall in the same clock minute
- **THEN** the app creates one AI-origin time entry whose end is after its start by at least one minute
- **AND** no manual time edit is required to accept it
- **AND** no zero- or negative-duration entry is ever stored.

#### Scenario: A same-minute suggestion at the end of the day stays within the day

- **WHEN** the user accepts a timed suggestion whose start and end both resolve to the final minute of the
  local day
- **THEN** the app creates one AI-origin time entry whose span is the computed duration, ending at the
  day's final minute and starting earlier the same day
- **AND** no manual time edit is required to accept it
- **AND** the entry does not cross into the next day and is never stored as a zero- or negative-duration
  span.

#### Scenario: Accepting a suggestion creates one AI-origin entry carrying cost

- **WHEN** the user accepts a suggestion that has an aggregate cost figure
- **THEN** the app creates exactly one AI-origin time entry carrying the block's provenance and its cost
- **AND** marks the suggestion accepted and links it to the created entry.

#### Scenario: Dismissing writes nothing

- **WHEN** the user dismisses a suggestion
- **THEN** the suggestion is recorded as dismissed
- **AND** no time entry is created.

### Requirement: AI-suggested time and cost are reported separately from human time

The app SHALL report accepted AI-suggested time and its cost as figures distinct from manually-entered
human time. Summaries SHALL expose AI-suggested duration and AI-suggested cost separately from human
duration, and SHALL NOT fold AI figures into a human total. CSV export SHALL include each entry's origin
and its cost as structured columns, so AI cost is never represented only as free-text note content.
Unknown cost SHALL render as an explicit "unknown" indicator, never as zero (absence ≠ zero).

#### Scenario: Summary separates AI cost from human time

- **WHEN** a date range contains both manual entries and accepted AI-suggested entries
- **THEN** the summary reports human duration, AI-suggested duration, and AI-suggested cost as separate
  figures
- **AND** AI cost is never added into a human total
- **AND** a range with no AI cost reports the AI cost as unknown ("—"), never zero.

#### Scenario: CSV carries cost as a structured column

- **WHEN** entries are exported to CSV
- **THEN** each row carries its origin and its cost as columns
- **AND** an entry with no cost leaves the cost cell empty rather than emitting zero.

### Requirement: Absence is explained, never shown as zero

The app SHALL surface, never silently drop or zero, evidence that produces no suggestion, and SHALL
explain each cause with an action the user can take. Unknown durations, token totals, and costs SHALL be
shown as an "unknown" indicator, never as zero. The Suggestions surface SHALL distinguish and explain at
least: an environment with evidence but no project mapping (action: map it in Settings); evidence with no
usable timestamps that is therefore not auto-trackable (action: add a start/end to log it manually); an
AI evidence source that is disabled or unavailable (action: review the source in Settings); and the
absence of any imported evidence (action: import from Langfuse in Settings). The surface SHALL NOT render
a bare empty list.

#### Scenario: Each no-suggestion cause is named with an action

- **WHEN** the Suggestions view has no acceptable suggestions for a reason
- **THEN** the view names the cause (unmapped environment / untimed evidence / source disabled or down /
  nothing imported)
- **AND** offers the corresponding action path
- **AND** never shows a bare empty list or a zero in place of an unknown value.

#### Scenario: Untimed evidence is shown as not auto-trackable, not zero

- **WHEN** mapped evidence has no usable timestamps
- **THEN** the block is shown as needing manual time and as not auto-trackable
- **AND** its duration, tokens, and cost render as "unknown" ("—"), never as zero.
