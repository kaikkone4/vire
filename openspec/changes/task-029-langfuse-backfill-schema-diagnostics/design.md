# Design — TASK-029

Scope: the **Langfuse importer component only** (`src-tauri/src/langfuse/*`), its IPC surface
(`lib.rs`), the `settings` store, and the import-result rendering in `src/main.ts`. No change to the
capture subsystem, the runtime observer, the classifier, env→project mapping, or CSV export.

## 1. Root-cause analysis (to be confirmed by Workstream A before any widening)

### 1.1 Why 611/640 traces are skipped

The import loop deserializes each list-payload entry all-or-nothing:

```
// importer.rs:222
let trace: Trace = match serde_json::from_value(value.clone()) {
    Ok(t) => t,
    Err(_) => { schema_issues += 1; warnings.push("a trace did not match the expected shape".into()); continue; }
};
```

`Trace` (`model.rs:128`) requires only `id`; every other field is `#[serde(default)]`. `#[serde(default)]`
fills a default **only when the field is absent** — a field that is **present with an unexpected type**
still fails. The typed field `observations: Vec<Observation>` (`model.rs:146`) is the prime suspect:

- Langfuse **v3** `GET /api/public/traces` (the **list** endpoint the importer uses) returns
  `observations` as an **array of observation ID strings**, and `scores` likewise. Deserializing a
  `string` into the struct `Observation` fails → the **whole trace** is dropped.
- This fits the observed numbers precisely: of 640 traces, the **611** that *have* observations fail to
  parse; the **29** that parse are observation-less traces already seen (duplicates); **0** new.
- Discovery still found 9 environments because it tolerates per-trace parse failures and only needs
  `Trace.environment` from the traces that *do* parse (`discovery.rs:42`).

**This is a hypothesis with a strong fit, not a verified fact.** Workstream A's diagnostics exist to
**prove the dominant reason against Janne's live stack** before Workstream B changes any parsing. The
design must not hard-code "fix observations" as an article of faith — it must surface *whatever* the
classifier finds (there may be a secondary reason, e.g. a null typed field).

### 1.2 Why usage/cost are `schema_changed` even for traces that parse

`normalize_trace` (`importer.rs:317`) fetches observations via `get_observations(trace_id)` when the
embedded array is empty, then flags `schema_changed` if any generation `lacks_usage_and_cost()`
(`model.rs:259`). TASK-027 already widened the token/cost readers to cover `usageDetails`/`costDetails`
(`model.rs:180`). So once identification stops dropping traces (§2), usage/cost should resolve for v3
generations; any residual `schema_changed` is then a **genuine** unsupported shape — exactly what the
state is for.

## 2. Architectural principle: decouple identification from usage extraction

**Today** one brittle `from_value::<Trace>` conflates two questions:
*"can I identify this trace?"* and *"can I read its peripheral fields?"*. A single bad peripheral field
answers "no" to both and the trace vanishes.

**Target:** identification is **total over an identifiable payload**. The import loop reads, from the
raw `serde_json::Value`, the fields needed to *identify and place* a trace — `id` (required),
`timestamp`, `environment`, `sessionId` — with a tolerant reader that never fails the whole trace on a
peripheral-field type mismatch. Usage/cost come **only** from observations (embedded objects when
present, else the fetch path). Consequences:

- A trace with a usable `id` is **always imported**. If usage is unreadable it is imported as
  `schema_changed` (counted, surfaced for review) — consistent with "absence is a warning, not zero"
  (`03_architecture_plan.md:89`) and the existing `schema_changed` semantics.
- `observations` becomes a **tolerant** field: accept embedded objects (fast path) **or** ID strings
  (ignore, fall through to fetch). Implementation options, in preference order:
  1. A `#[serde(deserialize_with = …)]` on `observations` that keeps only JSON **object** elements and
     drops ID strings — minimal, keeps `Trace` typed.
  2. Type `observations` as `Vec<serde_json::Value>` and filter to objects at use-site.
  Option 1 is preferred (localized, no call-site churn). Either way the embedded fast path is an
  optimization; the `get_observations` fetch remains the authoritative usage source for the v3 list shape.
- `MissingTraceId` (no usable `id`) is the **only** identification failure that still skips a trace — and
  it is classified and counted, never an opaque repeated string.

## 3. Forensic diagnostics (Workstream A) — secret-free by construction

### 3.1 The `SkipReason` taxonomy (fixed, secret-free enum)

A bounded enum, each variant a stable label string in the report:

| Reason | Meaning |
| --- | --- |
| `missing_trace_id` | No usable `id` on the entry — cannot identify; skipped. |
| `observations_not_embedded` | `observations` present but elements are IDs/strings, not objects — handled via fetch; informational, **not** a drop after §2. |
| `field_type_mismatch` | An identification field had an unexpected JSON type (carries the **field name** only). |
| `generation_lacks_usage_and_cost` | A generation observation had no token/cost in any supported location → trace imported as `schema_changed`. |
| `observations_fetch_failed` | The per-trace observations fetch failed → `schema_changed`. |

The classifier inspects the raw `serde_json::Value` **structurally** — it never deserializes for the
message and never reads field *values*.

### 3.2 Bounded structural sample (SEC-011)

Per reason, keep at most a small fixed N (e.g. **3**) samples. A sample contains only:

- the **list of top-level JSON key names** present on the entry (e.g. `["id","timestamp","environment","observations","scores"]`);
- for `field_type_mismatch` / `observations_not_embedded`: the **field name** and its **JSON type name**
  (`"string" | "number" | "bool" | "array" | "object" | "null"`), and for arrays the element type name.

A sample MUST NOT contain: any field **value**, any array/object **contents**, raw payload bytes, a
`serde_json` error string, or any prompt/session/metadata/credential material. Type/key **names** are
structure, not content. This is the SEC-011 invariant and is unit-tested (a sample serialized to JSON
contains none of `sk-`, `pk-`, `Bearer`, a known prompt fixture, or a raw value).

### 3.3 Report shape

`ImportSummary`/`EnvImportLine` gain `skip_reasons: Vec<SkipReasonCount>` (`{reason, count}`) and
`skip_samples: Vec<SkipSample>` (bounded). The repeated free-string `warnings` for shape skips is
**removed** in favour of the grouped breakdown; transport/auth/persist warnings (already secret-free,
fixed strings) stay as-is. `ImportReport::from_summaries` aggregates `skip_reasons` across environments.

## 4. Range, cursor, and backfill (Workstream C)

### 4.1 The setting

New key-value `settings` row `langfuse_import_range`, value one of:
`last_7d | last_30d | last_90d | all | since:<RFC3339>`. Absent → default `last_30d`. Resolved
settings-first (mirrors TASK-026 `resolve_config`), validated (a malformed `since:` falls back to the
default with a secret-free note). The value maps to a **range floor**:

- `last_Nd` → `now − N days`
- `all` → epoch (`1970-01-01T00:00:00Z`)
- `since:<ts>` → that timestamp

### 4.2 Two window modes over one engine

`recent_window(7)` (`mod.rs:26`) is replaced by a window resolver that produces a **per-environment**
window:

```
fn import_window(env, mode, range_floor, cursor_ts) -> ImportWindow
  incremental: from = max(range_floor, cursor_ts − OVERLAP); to = now
  backfill:    from = range_floor;                            to = now   // ignores cursor; re-scan range
```

- **Incremental** (normal manual + auto import): resumes from the per-env cursor. `OVERLAP` is a small
  reconciliation lookback (default ~1h, a constant) so late/delayed traces inside the overlap are
  re-seen; durable dedupe (`store.rs:213`) makes the overlap free, and the cursor never regresses
  (`importer.rs:277`). First import of an env (no cursor) starts at the range floor.
- **Backfill** ("Backfill now"): re-scans floor→now regardless of cursor. Used to pull history after the
  user widens the range or on first setup.

Discovery (`discover_and_record`, `mod.rs:69`) keeps a **bounded recent** window (it only enumerates env
names; it does not need the full backfill range) to avoid a full-history scan on every import.

### 4.3 Chunked, resumable, atomic backfill

`persist_import_run` commits one run atomically at the **end** (`store.rs:135`). A single giant backfill
would therefore (a) hold all evidence in memory, (b) hold a long write, and (c) **lose everything** if the
bounded wait or the process is interrupted. Design: a backfill over `[floor, now]` is executed as an
**ordered sequence of bounded sub-windows** (e.g. monthly chunks, newest→oldest or oldest→newest), each
run through the existing single-window engine and **persisted atomically as its own run** that advances
the per-env cursor. Therefore:

- Progress is **durable as it goes** (each chunk commits); an interruption loses at most the in-flight
  chunk.
- A backfill is **resumable**: re-running "Backfill now" skips already-imported traces via durable
  dedupe and fills any gap; the non-regressing cursor and `(env,trace_id)` PK guarantee idempotency.
- The S-3 atomic-per-run invariant and the TASK-021 in-band persist-failure surfacing are **preserved**
  unchanged — a chunk is just another run.

`MAX_PAGES = 1000` × `PAGE_LIMIT = 50` bounds a single window to ~50k traces. The chunk boundaries keep
each window small enough that this backstop is not hit in practice; if a window *does* hit it, the report
**says so** (a `reached_page_limit` note) and the user re-runs to continue — **never silent truncation**
(no-silent-caps).

**Page-limit continuation (DEC-032 — supersedes the SW-4 second-oldest-instant scheme; see
`arch-review.md` §8).** Pagination restarts at page 1, so a naive re-run of a page-limited window would
re-walk the same first `MAX_PAGES` pages forever and history beyond the backstop would stay unreachable —
durable dedupe suppresses rows but never advances the offset. The continuation uses the API's **inclusive**
`fromTimestamp` (verified: `fromTimestamp` is "on or after" ≥; `toTimestamp` is "before" <):

- Every trace-import page request is ordered **`orderBy=timestamp.asc`** (oldest → newest; the default order
  is undocumented so it is set explicitly). Backfill chunks run **oldest → newest**.
- A page-limited backfill persists a single durable **inclusive resume-cursor** `resume_from` =
  `max_reached`, the chronological **maximum** parseable `timestamp` the run returned (the same `max_ts` the
  incremental cursor already computes — no oldest-instant tracking).
- The next run resumes with `fromTimestamp = resume_from` (**inclusive**), so it **re-reads the entire
  boundary instant** from page 1; durable `(environment, trace_id)` dedupe suppresses the overlap and
  pagination advances into strictly-newer history. Because the whole boundary instant is re-scanned each run
  (never resumed mid-instant), **equal-timestamp traces at the cut are fully re-read, never skipped, and no
  stable tie-breaker is needed** — the verified API offers no page-token and no guaranteed secondary sort,
  so a keyset cursor is not available and the inclusive-re-read obviates it.
- A run that drains `[resume_from, now)` without hitting the backstop **clears** the cursor (range fully
  covered); a hard-down stop leaves it untouched. Store read/write/clear faults still surface in-band via
  `PERSIST_FAILURE_MSG`.

**Single-instant saturation (the one unreachable corner).** Let reachable depth `D = MAX_PAGES × PAGE_LIMIT
= 50 000`. The only case the inclusive cursor cannot drain is a single `timestamp` instant holding `≥ D`
traces (then `max_reached == resume_from`, cursor cannot advance). This is handled as an **explicit terminal
state**: detect (backstop hit AND cursor did not advance), **never push the cursor past unread data** (no
skip), and surface a **distinct secret-free terminal diagnostic** (a count, no timestamp value) meaning the
instant exceeds the page-depth limit — **never an infinite "re-run to continue" loop**. Because Langfuse
timestamps are millisecond-precision and the cursor stores the source value verbatim, saturation needs
≥ 50 000 traces at one millisecond — unreachable for this single-user prototype; the terminal-surface exists
for the invariant, not because it fires. The resume-cursor is a single UTC RFC3339 timestamp — a position
marker, never trace content or a credential (excluded from every serialized/rendered/logged surface).

**Invariant.** Every trace whose `timestamp`-instant is shared by fewer than `D` traces is eventually
imported **exactly once** (dedupe ⇒ exactly once); an instant with `≥ D` traces is surfaced as a named
terminal diagnostic, never silently skipped or looped. No instant in this product approaches `D`, so the
operative guarantee is unconditional: every trace is eventually imported exactly once.

### 4.4 Efficiency at backfill scale

- **N+1 observations fetch.** `normalize_trace` fetches `/api/public/observations?traceId=` per trace
  (`importer.rs:321`). Over thousands of backfilled traces that is thousands of round-trips. Recommended
  (Workstream C): for a backfill chunk, prefer a **windowed observations scan** (one paginated
  `GET /api/public/observations` over the chunk window) joined to traces by `traceId`, falling back to
  per-trace fetch only for traces the windowed scan did not cover. Stays read-only, same allowlist, no
  new path beyond the already-allowlisted `observations` endpoint (`config.rs:103`). If this proves more
  than the workstream warrants, the per-trace path is correct (just slower) and the optimization can be a
  follow-up — but the N+1 cost MUST be acknowledged, not hidden.
- **`seen_trace_ids` memory.** `seen_trace_ids` loads all of an env's trace ids into a `HashSet` per run
  (`store.rs:213`). For a single-user prototype with chunked runs this is acceptable; flag for a future
  bounded-cursor dedupe if histories grow large.

### 4.5 Bounding the backfill wait

`import_langfuse_now` runs under `run_bounded_result(IMPORT_TIMEOUT_SECS, …)` (`lib.rs:764`). Backfill is
inherently slower; the `backfill_langfuse_now` command uses a **larger** bound. Because backfill is
chunked-durable-resumable (§4.3), a timeout is non-destructive — committed chunks persist and the user
re-runs to continue. The command stays on the dedicated OS thread off the UI/Tauri runtime and serialized
via `import_lock`, identical to manual import.

## 5. IPC surface (additive)

- `get_langfuse_import_range` / `set_langfuse_import_range(range)` — read/validate/persist the setting.
- `backfill_langfuse_now` — like `import_langfuse_now` but backfill mode + larger bound; returns the same
  secret-free `ImportOutcome { snapshot, report }`.
- `import_langfuse_now` and `run_blocking_import` keep their signatures; internally they resolve the
  incremental per-env window instead of the fixed 7-day window.

## 6. Security / L2 thresholds

- **SEC-011 (new):** skip-reason classification, structural samples, grouped report, and range/backfill
  surfaces carry only fixed reason labels, JSON key names, JSON type names, and counts — **never** raw
  payload bytes, field values, `serde` error strings, prompt/session/metadata, or credentials. Extends
  SEC-003 (no secret in errors/logs) and SEC-010 (secret-free import report).
- **SEC-002 unchanged:** backfill/incremental/discovery all build URLs via `ImporterConfig::build_url`
  under `/api/public/*`; `local` still requires loopback, `cloud` stays the only explicit off-host path;
  GET-only, no new egress host, redirects still refused.
- **Disabled short-circuit unchanged:** backfill honours `langfuse_enabled` before any probe/Keychain/
  socket, exactly like `import_langfuse_now` (`lib.rs:748`).
- **Absence ≠ zero unchanged:** widening never coerces a missing value to `0`; an unreadable usage shape
  is `schema_changed`, not a zero total.

## 7. Compatibility & rollback

- **Additive data model:** one new `settings` row (`langfuse_import_range`) and one new importer-owned
  table `langfuse_backfill_progress` (a single global continuation-cursor row, §4.3 / DEC-032). Both are
  created by the existing idempotent `store::migrate`; no existing table/column is altered. The row holds
  only a UTC RFC3339 timestamp (the **inclusive `resume_from`** cursor) + the fixed marker key — no trace
  content or credential. Fresh installs and reopens converge via the same migration; an absent cursor means
  "start a fresh backfill from the configured range floor". Absent range setting → `last_30d`.
- **Behaviour change to flag:** the first-import / range floor moves from a fixed **7 days** to **30
  days** (default), and the incremental `from` is now cursor-driven (previously the window ignored the
  cursor). Re-importing a trace already stored is a durable-dedupe no-op, so the change is safe to apply
  to an existing store.
- **Rollback:** revert the importer/settings/UI changes; the new `settings` row is inert to older builds
  (unknown key ignored), and the durable cursor/dedupe rows are forward/back compatible. The new
  `langfuse_backfill_progress` table is likewise inert to older builds (an unknown table is simply never
  read), so a downgrade silently ignores it and a re-upgrade re-creates it idempotently. RELEASE.md (SW-6)
  records this as additive with a partial-automated rollback, consistent with the TASK-026/027 posture.

## 8. Verification flags (confirm during SW-2, not assumed here)

- **VF-1 (gates B):** Workstream A diagnostics confirm, against Janne's live v3 stack, that the dominant
  skip reason is `observations_not_embedded` (ID-string list shape) — *before* the parser is widened. If
  the classifier shows a different dominant reason, B targets that instead.
- **VF-2:** After §2, the 611 previously-skipped traces import as `healthy` (usage/cost read from fetched
  observations) or `schema_changed` (genuinely unreadable) — **none silently dropped**; new-trace count > 0.
- **VF-3:** A backfill over `last_90d` is resumable via the inclusive `resume_from` cursor (DEC-032):
  interrupting and re-running converges (no duplicate rows, cursor advances monotonically forward), a
  page-limited window re-reads the whole boundary instant so equal-timestamp traces are never skipped, and a
  single instant exceeding the page-depth limit `D` is surfaced as a distinct terminal diagnostic rather
  than silently truncated or looped (see `arch-review.md` §8.4 invariant).
- **VF-4 (SEC-011):** the serialized report + samples contain no secret/prompt/session/value material
  (negative fixture test).

## 9. Out of scope — TASK-030 boundary

`langfuse_ai_evidence` already carries the substrate a future suggestion needs (`session_id`,
`ai_start_ts`/`ai_end_ts`, tokens, `cost_total`, health — `model.rs:276`). TASK-029 makes that substrate
**correct and complete**; producing **suggested time entries** from it is **TASK-030** and is not built
here. Rationale (architect): time-entry suggestion is a different competency (the classifier/suggestion
engine + review UI), is gated on this data first being trustworthy, and pulling it in would couple a
high-risk attribution surface (DEC-001/006) to an import-correctness task. See `arch-review.md` §Split.
