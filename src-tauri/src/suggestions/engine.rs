//! The suggestion engine (design §2). Pure over SQLite: read evidence → associate to a project at
//! read time → bucket by `(project, local date)` → cluster timed rows by the `GAP` constant →
//! aggregate tokens/cost/counts/health → persist the recomputed `pending` set. No network, no
//! Langfuse egress (DEC-001/017).

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use uuid::Uuid;

use rusqlite::Connection;

use crate::langfuse::model::HealthState;

use super::store::{self, EvidenceRow};
use super::{Suggestion, SuggestionList, UnmappedEnv};

/// Clustering gap: a new block opens when the next row starts more than this many minutes after the
/// current block's running end (design §2.2). DEC-033 default, flagged to BA (F1) as tunable — read
/// from this single constant so the tuning point is one edit, never a scattered literal.
const GAP_MINUTES: i64 = 30;

/// Recompute and persist the pending suggestion set, returning it together with the unmapped-env
/// summary. Idempotent: drops the prior `pending` rows, re-inserts the recomputed blocks, and never
/// resurrects an `accepted`/`dismissed` block (the decided-row guard in `store::insert_if_not_decided`).
/// Writes only `time_entry_suggestions` — never a `time_entries` row (DEC-006, no auto-posting).
pub fn generate(conn: &Connection) -> rusqlite::Result<SuggestionList> {
    let rows = store::load_evidence(conn)?;
    let (blocks, unmapped) = cluster(&rows);

    store::delete_pending(conn)?;
    let ts = now();
    for block in &blocks {
        let suggestion = block.to_suggestion(Uuid::new_v4().to_string(), ts.clone());
        store::insert_if_not_decided(conn, &suggestion)?;
    }

    let suggestions = store::list_pending(conn)?;
    Ok(SuggestionList {
        suggestions,
        unmapped,
    })
}

/// Read the current `pending` suggestions together with a freshly-computed unmapped-env summary
/// **without** regenerating — no `pending` row is deleted or inserted, no `time_entries` row is written.
/// Backs the non-`regenerate` list path (Workstream B) so the "map this env" notice still shows when the
/// user only opens the view and has not asked to refresh.
pub fn current(conn: &Connection) -> rusqlite::Result<SuggestionList> {
    let rows = store::load_evidence(conn)?;
    let unmapped = unmapped_summary(&rows);
    let suggestions = store::list_pending(conn)?;
    Ok(SuggestionList {
        suggestions,
        unmapped,
    })
}

/// A computed block before it is given an id/timestamps and persisted. Holds only secret-free
/// aggregates. `block_start_ts`/`duration_minutes` are `None` for an untimed ("needs manual time")
/// block — never a 0-minute span (absence ≠ zero).
struct ComputedBlock {
    project_id: String,
    project_name: String,
    date: String,
    block_start_ts: Option<String>,
    block_end_ts: Option<String>,
    duration_minutes: Option<i64>,
    trace_count: i64,
    session_count: i64,
    total_tokens: Option<i64>,
    cost_total: Option<f64>,
    cost_currency: Option<String>,
    health: String,
    confidence: String,
    source: String,
    reason: String,
}

impl ComputedBlock {
    fn to_suggestion(&self, id: String, ts: String) -> Suggestion {
        Suggestion {
            id,
            project_id: self.project_id.clone(),
            project_name: self.project_name.clone(),
            date: self.date.clone(),
            block_start_ts: self.block_start_ts.clone(),
            block_end_ts: self.block_end_ts.clone(),
            duration_minutes: self.duration_minutes,
            trace_count: self.trace_count,
            session_count: self.session_count,
            total_tokens: self.total_tokens,
            cost_total: self.cost_total,
            cost_currency: self.cost_currency.clone(),
            health: self.health.clone(),
            confidence: self.confidence.clone(),
            source: self.source.clone(),
            reason: self.reason.clone(),
            status: "pending".to_string(),
            accepted_entry_id: None,
            created_at: ts.clone(),
            updated_at: ts,
        }
    }
}

/// A timed evidence row whose start and end both parsed — the only rows that form a duration block.
struct Timed<'a> {
    row: &'a EvidenceRow,
    start: DateTime<Local>,
    end: DateTime<Local>,
}

/// All evidence for one `(project_id, local date)` bucket, split into rows that can form a timed
/// block and rows that cannot (missing/unusable timestamps → the "needs manual time" path).
struct Bucket<'a> {
    project_name: String,
    timed: Vec<Timed<'a>>,
    untimed: Vec<&'a EvidenceRow>,
}

/// Bucket evidence by `(project, local date)`, cluster timed rows into blocks, and fold every
/// untimed row into a per-bucket "needs manual time" block. Returns the blocks plus a per-environment
/// count of evidence in **unmapped** environments (reported, never dropped).
///
/// Interpretation note (design §2.4 specifies the all-untimed case explicitly; this generalises it
/// safely): a bucket that mixes timed and untimed rows yields both its timed block(s) **and** one
/// untimed block for the leftover rows, so no evidence is ever dropped or zeroed. A mapped row with
/// no usable timestamp at all is dated by the generation day (it carries no calendar day of its own)
/// and surfaced as "needs manual time" — its duration stays `NULL`, never invented.
fn cluster(rows: &[EvidenceRow]) -> (Vec<ComputedBlock>, Vec<UnmappedEnv>) {
    let fallback_date = local_date(&now_local());
    let mut buckets: BTreeMap<(String, String), Bucket<'_>> = BTreeMap::new();

    for row in rows {
        let Some(project_id) = row.project_id.clone() else {
            // Unmapped evidence is tallied by `unmapped_summary` and reported separately — never
            // folded into a block (absence ≠ zero).
            continue;
        };
        let project_name = row.project_name.clone().unwrap_or_default();
        let start = row.ai_start_ts.as_deref().and_then(parse_local);
        let end = row.ai_end_ts.as_deref().and_then(parse_local);
        let date = match (&start, &end) {
            (Some(s), _) => local_date(s),
            (None, Some(e)) => local_date(e),
            (None, None) => fallback_date.clone(),
        };
        let bucket = buckets.entry((project_id, date)).or_insert_with(|| Bucket {
            project_name,
            timed: Vec::new(),
            untimed: Vec::new(),
        });
        match (start, end) {
            (Some(start), Some(end)) => bucket.timed.push(Timed { row, start, end }),
            _ => bucket.untimed.push(row),
        }
    }

    let mut blocks = Vec::new();
    for ((project_id, date), mut bucket) in buckets {
        bucket.timed.sort_by_key(|t| t.start);
        for cluster in cluster_by_gap(&bucket.timed) {
            blocks.push(timed_block(
                &project_id,
                &bucket.project_name,
                &date,
                &cluster,
            ));
        }
        if !bucket.untimed.is_empty() {
            blocks.push(untimed_block(
                &project_id,
                &bucket.project_name,
                &date,
                &bucket.untimed,
            ));
        }
    }

    (blocks, unmapped_summary(rows))
}

/// Per-environment count of evidence in **unmapped** environments — surfaced so the UI can prompt the
/// user to map it (absence ≠ zero: never dropped, never zeroed). Kept independent of bucketing so the
/// non-regenerate list path (`current`) can compute it without recomputing any block.
fn unmapped_summary(rows: &[EvidenceRow]) -> Vec<UnmappedEnv> {
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    for row in rows {
        if row.project_id.is_none() {
            *counts.entry(row.environment.clone()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .map(|(environment, trace_count)| UnmappedEnv {
            environment,
            trace_count,
        })
        .collect()
}

/// Cluster start-sorted timed rows into contiguous blocks: a new block opens when the next row starts
/// more than `GAP_MINUTES` after the running block end (compared in **seconds** so a 30:01 gap splits
/// while a flat 30:00 gap does not — minute truncation would blur the boundary).
fn cluster_by_gap<'a, 'b>(timed: &'b [Timed<'a>]) -> Vec<Vec<&'b Timed<'a>>> {
    let mut blocks: Vec<Vec<&Timed<'a>>> = Vec::new();
    let mut current: Vec<&Timed<'a>> = Vec::new();
    let mut current_end: Option<DateTime<Local>> = None;

    for t in timed {
        match current_end {
            Some(end) if (t.start - end).num_seconds() > GAP_MINUTES * 60 => {
                blocks.push(std::mem::take(&mut current));
                current.push(t);
                current_end = Some(t.end);
            }
            _ => {
                current.push(t);
                current_end = Some(current_end.map_or(t.end, |end| end.max(t.end)));
            }
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
}

/// Build a timed block: span = `min(start)`..`max(end)`, duration `max(1, round(minutes))` so a tiny
/// real span never reports as 0. Confidence is `high` only when every row is healthy and the block
/// draws from a single environment; any degraded health or mixed sources drops it to `medium`.
fn timed_block(
    project_id: &str,
    project_name: &str,
    date: &str,
    cluster: &[&Timed<'_>],
) -> ComputedBlock {
    let start = cluster.iter().map(|t| t.start).min().expect("non-empty");
    let end = cluster.iter().map(|t| t.end).max().expect("non-empty");
    let rows: Vec<&EvidenceRow> = cluster.iter().map(|t| t.row).collect();

    let duration = ((end - start).num_seconds() as f64 / 60.0).round() as i64;
    let duration = Some(duration.max(1));

    let agg = Aggregate::of(&rows);
    let all_healthy = rows
        .iter()
        .all(|r| HealthState::parse(&r.health) == HealthState::Healthy);
    let confidence = if all_healthy && agg.environments.len() == 1 {
        "high"
    } else {
        "medium"
    };

    let reason = format!(
        "{} Langfuse trace{}, {} session{} in {}, {}–{}{}",
        agg.trace_count,
        plural(agg.trace_count),
        agg.session_count,
        plural(agg.session_count),
        env_phrase(&agg.environments),
        hhmm(&start),
        hhmm(&end),
        agg.cost_note(),
    );

    ComputedBlock {
        project_id: project_id.to_string(),
        project_name: project_name.to_string(),
        date: date.to_string(),
        block_start_ts: Some(local_ts(&start)),
        block_end_ts: Some(local_ts(&end)),
        duration_minutes: duration,
        trace_count: agg.trace_count,
        session_count: agg.session_count,
        total_tokens: agg.total_tokens,
        cost_total: agg.cost_total,
        cost_currency: agg.cost_currency,
        health: agg.worst_health,
        confidence: confidence.to_string(),
        source: source_label(&agg.environments),
        reason,
    }
}

/// Build the per-bucket "needs manual time" block from rows with no usable timestamp pair. Duration,
/// span, and (when absent) tokens/cost stay `None` — never 0. Confidence is always `low`.
fn untimed_block(
    project_id: &str,
    project_name: &str,
    date: &str,
    rows: &[&EvidenceRow],
) -> ComputedBlock {
    let agg = Aggregate::of(rows);
    let reason = format!(
        "{} Langfuse trace{} with no usable timestamps in {} — needs manual time{}",
        agg.trace_count,
        plural(agg.trace_count),
        env_phrase(&agg.environments),
        agg.cost_note(),
    );

    ComputedBlock {
        project_id: project_id.to_string(),
        project_name: project_name.to_string(),
        date: date.to_string(),
        block_start_ts: None,
        block_end_ts: None,
        duration_minutes: None,
        trace_count: agg.trace_count,
        session_count: agg.session_count,
        total_tokens: agg.total_tokens,
        cost_total: agg.cost_total,
        cost_currency: agg.cost_currency,
        health: agg.worst_health,
        confidence: "low".to_string(),
        source: source_label(&agg.environments),
        reason,
    }
}

/// Secret-free aggregates over a set of evidence rows: counts, summed tokens/cost, worst health, and
/// the contributing environments. Absence is preserved as `None` throughout — a token/cost total is
/// `None` only when **every** contributing row was `None`, never a fabricated `0`.
struct Aggregate {
    trace_count: i64,
    session_count: i64,
    total_tokens: Option<i64>,
    cost_total: Option<f64>,
    cost_currency: Option<String>,
    mixed_currency: bool,
    worst_health: String,
    environments: BTreeSet<String>,
}

impl Aggregate {
    fn of(rows: &[&EvidenceRow]) -> Aggregate {
        let trace_count = rows.len() as i64;
        let session_count = rows
            .iter()
            .filter_map(|r| r.session_id.as_deref())
            .collect::<BTreeSet<_>>()
            .len() as i64;

        let tokens: Vec<i64> = rows.iter().filter_map(|r| r.total_tokens).collect();
        let total_tokens = if tokens.is_empty() {
            None
        } else {
            Some(tokens.iter().sum())
        };

        // Cost is summed only within a single currency. Mixed distinct currencies (including a labelled
        // currency alongside an unlabelled one) are ambiguous → leave the total NULL and note it.
        let costed: Vec<&&EvidenceRow> = rows.iter().filter(|r| r.cost_total.is_some()).collect();
        let currencies: BTreeSet<Option<String>> =
            costed.iter().map(|r| r.cost_currency.clone()).collect();
        let (cost_total, cost_currency, mixed_currency) = if currencies.len() > 1 {
            (None, None, true)
        } else if costed.is_empty() {
            (None, None, false)
        } else {
            let sum: f64 = costed.iter().filter_map(|r| r.cost_total).sum();
            (Some(sum), currencies.into_iter().next().flatten(), false)
        };

        let worst_health = rows
            .iter()
            .map(|r| HealthState::parse(&r.health))
            .max_by_key(|h| health_severity(*h))
            .unwrap_or(HealthState::Unknown)
            .as_str()
            .to_string();

        let environments = rows.iter().map(|r| r.environment.clone()).collect();

        Aggregate {
            trace_count,
            session_count,
            total_tokens,
            cost_total,
            cost_currency,
            mixed_currency,
            worst_health,
            environments,
        }
    }

    /// A secret-free clause appended to a reason when costs spanned multiple currencies.
    fn cost_note(&self) -> &'static str {
        if self.mixed_currency {
            " (mixed currencies; cost omitted)"
        } else {
            ""
        }
    }
}

/// Severity ranking for "worst health across the block" (design §2.4). `Healthy` is least severe;
/// everything else degrades confidence to at least `medium`. The exact ordering among degraded states
/// is for display only — the load-bearing contract is "all healthy ⇒ healthy, otherwise degraded".
fn health_severity(h: HealthState) -> u8 {
    match h {
        HealthState::Healthy => 0,
        HealthState::Stale => 1,
        HealthState::Delayed => 2,
        HealthState::Duplicate => 3,
        HealthState::WrongEnv => 4,
        HealthState::SchemaChanged => 5,
        HealthState::Missing => 6,
        HealthState::Unavailable => 7,
        HealthState::AuthOrNetworkError => 8,
        HealthState::Unknown => 9,
    }
}

/// `'langfuse:env'` for a single environment, `'langfuse:a,b'` (sorted) when a project's block draws
/// from several mapped environments. Carries only environment names — no secret (SEC-012).
fn source_label(environments: &BTreeSet<String>) -> String {
    let envs: Vec<&str> = environments.iter().map(String::as_str).collect();
    format!("langfuse:{}", envs.join(","))
}

/// Human-readable environment clause for a reason, e.g. ``env `veronavi` `` or ``envs `a`, `b` ``.
fn env_phrase(environments: &BTreeSet<String>) -> String {
    let quoted: Vec<String> = environments.iter().map(|e| format!("`{e}`")).collect();
    let label = if quoted.len() == 1 { "env" } else { "envs" };
    format!("{label} {}", quoted.join(", "))
}

fn plural(n: i64) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// Parse a Langfuse timestamp into local time. Accepts RFC-3339 (with `Z`/offset, the imported shape)
/// and a couple of offset-less forms treated as local wall-clock. Returns `None` for anything
/// unparseable, so an unreadable timestamp degrades the row to "untimed" rather than dropping it.
fn parse_local(ts: &str) -> Option<DateTime<Local>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
        return Some(dt.with_timezone(&Local));
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(ts, fmt) {
            return Local.from_local_datetime(&naive).earliest();
        }
    }
    None
}

fn local_date(dt: &DateTime<Local>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

fn local_ts(dt: &DateTime<Local>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn hhmm(dt: &DateTime<Local>) -> String {
    dt.format("%H:%M").to_string()
}

fn now_local() -> DateTime<Local> {
    Local::now()
}

fn now() -> String {
    local_ts(&now_local())
}
