# SW-2 Workstream B notes — TASK-032 AI time-entry suggestions

Accept/dismiss IPC + `time_entries.origin` + DEC-003 reporting separation. Backend only. Builds on
Workstream A (engine + `time_entry_suggestions`, A-checkpoint PASS). Workstream **C** (Review/Accept UI)
is **not** implemented here.

## What shipped (tasks.md B1–B5)

- **B1** Additive `time_entries.origin` column (`TEXT NOT NULL DEFAULT 'manual'`) via the shared
  duplicate-column-tolerant `langfuse::store::add_column_if_absent` (now `pub(crate)`), called from
  `init_db`. Existing rows backfill to `'manual'` — prior human totals unchanged. `TimeEntry` gains
  `origin`; the manual input path (`TimeEntryInput`/`create_entry_repo`) never sets it (DB default wins).
- **B2** Three IPC commands registered in `generate_handler!`:
  `list_time_entry_suggestions(regenerate)`, `accept_time_entry_suggestion(id, edits?)`,
  `dismiss_time_entry_suggestion(id)`. `SuggestionEdit { project_id?, date?, start_time?, end_time?,
  note? }` added (`suggestions/mod.rs`). `list` with `regenerate=true` → `suggestions::generate`;
  `false` → new `suggestions::current` (pending set + freshly-computed unmapped summary, **no** delete/
  insert).
- **B3** `accept_suggestion_repo` is the **only** writer of an `origin='ai_suggested'` entry. Single
  `rusqlite` transaction: insert the entry → guarded `UPDATE … WHERE id=? AND status='pending'` to
  `accepted` + `accepted_entry_id`. An untimed block REQUIRES `edits` supplying start/end (else error —
  duration is never invented). Re-deciding an accepted/dismissed suggestion is rejected; a failed accept
  rolls back (zero writes). `dismiss_suggestion_repo` writes no entry, is idempotent on an already-
  dismissed row, and refuses to dismiss an accepted one.
- **B4** DEC-003 separation. `summary_repo` returns `duration_minutes` (human/`manual` only — meaning
  unchanged) **and** a distinct `ai_minutes` (`origin='ai_suggested'`); non-AI origins count as human
  (conservative — AI can never land in the billable column). `export_csv_repo` gains an `origin` column
  so each row is labelled and AI time is never folded into the human total.
- **B5** 4 unit tests (`lib.rs` test module). `cargo test` green, `cargo fmt --check` clean,
  `cargo clippy` adds no new findings.

## B-checkpoint evidence — accept/dismiss/reporting vs seeded suggestions

Each test seeds a `time_entry_suggestions` row directly (deterministic; importer-independent) and drives
the Workstream-B surface.

| Test | Seeded fixture | Verified output |
|---|---|---|
| `accept_creates_exactly_one_ai_entry_marks_accepted_and_is_decided_once` | timed pending block 09:00–10:00 | entry `duration=60`, `start=09:00`/`end=10:00`, `origin='ai_suggested'`, provenance note present; **exactly one** `ai_suggested` row; suggestion `accepted` + `accepted_entry_id`=entry.id; re-accept and dismiss both rejected; entry count stays 1 |
| `accept_of_untimed_block_requires_edits_and_never_invents_a_duration` | untimed block (`block_start/end_ts NULL`, `duration NULL`) | accept w/o edits → error "…start and end time…", **0** entries written, suggestion still `pending`; accept with start/end edits → `duration=90`, `origin='ai_suggested'` |
| `dismiss_writes_no_entry_is_idempotent_and_cannot_undo_an_accept` | one block dismissed; one block accepted | dismiss writes 0 entries, status `dismissed`; dismiss again Ok (idempotent); accept-after-dismiss rejected; dismiss-of-accepted rejected ("…accepted…") |
| `summary_and_csv_report_human_and_ai_time_separately` | 60 min manual + 30 min accepted-AI, same project/day | `summary.duration_minutes=60` (human), `summary.ai_minutes=30` (AI); CSV header has `origin`, rows carry `,manual,` and `,ai_suggested,` |

Full suite: `cargo test` **158 passed / 0 failed** (was 154 at A; +4 B tests; 0 regressions),
`cargo fmt --check` clean, `cargo clippy --all-targets` — no new findings (only the pre-existing
`langfuse/importer.rs:1170-1181` and `lib.rs` `db_path` `io::Error::new` warnings, both pre-dating
TASK-032 per the A QA report; line numbers shifted by this change but content untouched).

## Guarantees (design §5) held by B-scope

- **AI ≠ human time** (DEC-003) — `origin` tag + separated `summary`/`CSV`. Two `INSERT INTO
  time_entries` exist: `create_entry_repo` (defaults `'manual'`) and `accept_suggestion_repo`
  (hardcoded `'ai_suggested'`) — accept is the sole writer of AI entries.
- **no auto-posting** (DEC-006) — `generate`/`current` write only `time_entry_suggestions`; only
  `accept` writes a `time_entries` row. Verified: untimed-accept failure and dismiss both write 0 rows.
- **absence ≠ zero** (DEC-004) — accept of an unknown-duration block errors unless edits supply a span;
  never a fabricated duration. `current` preserves the A engine's NULL-not-zero output unchanged.
- **no secrets** (SEC-012) — the provenance note is built from `source` + `reason` only (both already
  secret-free); no session-id/payload/credential. `SuggestionEdit` carries only times/refs/notes.
- **Vire authoritative, no egress** (DEC-001/017) — all B paths are SQLite-only; no network added.

## Interpretation decisions (non-blocking; flag to QA/BA)

1. **`dismiss` idempotency vs "re-deciding a decided suggestion → rejected".** design §3 says dismiss is
   *idempotent*; tasks.md B3 says re-deciding a decided suggestion is rejected. Reconciled as: changing
   a decision is rejected (accept-of-decided, dismiss-of-accepted) but re-dismissing an already-dismissed
   row is a no-op success (the literal meaning of "idempotent"). Accept-of-dismissed and accept-of-
   accepted are both rejected.
2. **Provenance note vs user edit.** When `edits.note` is provided it replaces the default note (the
   user's explicit choice); otherwise a secret-free provenance summary `AI-suggested (<source>):
   <reason>` is recorded. The `origin='ai_suggested'` tag is the authoritative provenance marker
   regardless of the note, so a user-supplied note never loses the AI attribution.
3. **Human-minutes definition.** `summary.duration_minutes` = `origin != 'ai_suggested'` (not strictly
   `='manual'`) so any future/legacy origin counts as human — AI can never be silently tallied into the
   billable column, the DEC-003 hazard. After this migration every existing row is `'manual'`, so the
   number is identical to the pre-TASK-032 total.
4. **Editing an accepted AI entry.** `update_entry_repo` does not touch `origin`, so a later manual edit
   of an accepted entry keeps `origin='ai_suggested'` (provenance is sticky). Acceptable; note for QA.

## Not in B-scope (next: Workstream C, frontend)

`'Suggestions'` view + `renderSuggestions()`, the list/Accept/Edit/Dismiss UI, the unmapped "map it"
notice, absence/empty copy, and secret-free render (`design.md` §4 / tasks.md C1–C5). The new API fields
(`TimeEntry.origin`, `SummaryRow.ai_minutes`) are additive and forward-compatible; the existing
frontend `Entry`/`Summary` structural types ignore them until C consumes them. No frontend file was
changed in B (`git diff main` shows no `.ts`/`.mjs` changes).
</content>
