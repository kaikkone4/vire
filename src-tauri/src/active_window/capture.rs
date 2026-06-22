//! Zero-permission active-app + idle capture loop (TASK-048).
//!
//! The first capture slice. It fills the TASK-046 store with the two signals that need **no** macOS
//! permission and nothing else:
//!
//! - frontmost-app identity, via `NSWorkspace.frontmostApplication` (`objc2-app-kit`), and
//! - idle/away state, derived from the *age* of the last input event, via
//!   `CGEventSource::seconds_since_last_event_type` (`objc2-core-graphics`).
//!
//! In-process Rust only (design §3): no Swift sidecar, no `externalBin`, no IPC. The loop reads only
//! event *age* — it never installs an event tap and never reads event content, which is exactly what
//! keeps it out of Accessibility and Input Monitoring. It never queries the Accessibility trust
//! state, never reads a window title, and never copies the on-screen window list. Every persisted
//! row therefore carries `window_title = NULL`, `title_state = absent_no_permission`, and
//! `source = nsworkspace`. Writes go **only** through the TASK-046 [`store`] API; there is no new
//! table, no renderer/IPC/network/CSP surface, and no association into `time_entries`.

use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use chrono::NaiveDateTime;
use rusqlite::Connection;
use uuid::Uuid;

use super::config::{self, CaptureConfig};
use super::model::{
    health_state, idle_state, source, title_state, CaptureHealthEvent, EvidenceBlock,
    RawObservation,
};
use super::store;

/// Canonical capture timestamp format — matches the crate-root `now()` helper so the store's
/// `datetime(...)` retention comparisons line up across the raw, evidence, and health tables.
const TS_FMT: &str = "%Y-%m-%d %H:%M:%S";

/// One sampled tick. `frontmost = None` means the OS reported no frontmost application (no GUI
/// session). Built by the native readers in the live loop, and synthetically in tests so the
/// coalescing / health / store logic is exercised with zero native calls.
#[derive(Debug, Clone)]
pub struct Sample {
    pub frontmost: Option<FrontmostApp>,
    /// Age in seconds since the most recent input event of any tracked type.
    pub idle_seconds: f64,
}

/// Allowlisted frontmost-app identity. Either field may be `None` if the OS reports none (e.g. an
/// app with no `Info.plist` has no bundle id). No window title or path is ever carried here.
#[derive(Debug, Clone)]
pub struct FrontmostApp {
    pub app_name: Option<String>,
    pub app_bundle_id: Option<String>,
}

/// In-memory loop state carried across ticks: the currently open evidence block, the previous tick
/// timestamp (for sampling-gap detection), and the last day a retention prune ran.
#[derive(Debug, Default)]
pub struct CaptureState {
    open: Option<OpenBlock>,
    last_sample_ts: Option<String>,
    last_prune_day: Option<String>,
}

/// The open normalized block being extended while `(day, app_bundle_id, idle_state)` stays constant.
#[derive(Debug, Clone)]
struct OpenBlock {
    id: String,
    day: String,
    start_ts: String,
    end_ts: String,
    app_name: Option<String>,
    app_bundle_id: Option<String>,
    idle_state: &'static str,
}

// ----- pure helpers ----------------------------------------------------------------------------

/// Map an idle age (seconds) to a controlled-vocabulary `idle_state`, using configured thresholds.
/// `>= away` → `away`; else `>= idle_candidate` → `idle_candidate`; else `active`.
fn idle_state_for(idle_seconds: f64, cfg: &CaptureConfig) -> &'static str {
    let s = idle_seconds.max(0.0);
    if s >= cfg.away_seconds as f64 {
        idle_state::AWAY
    } else if s >= cfg.idle_candidate_seconds as f64 {
        idle_state::IDLE_CANDIDATE
    } else {
        idle_state::ACTIVE
    }
}

fn parse_ts(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, TS_FMT).ok()
}

/// Whole seconds between two capture timestamps, clamped at 0. Unparseable inputs yield 0.
fn duration_secs(start: &str, end: &str) -> i64 {
    match (parse_ts(start), parse_ts(end)) {
        (Some(a), Some(b)) => (b - a).num_seconds().max(0),
        _ => 0,
    }
}

/// Day bucket (`YYYY-MM-DD`) for a capture timestamp.
fn day_of(ts: &str) -> String {
    ts.get(..10).unwrap_or(ts).to_string()
}

// ----- sampling --------------------------------------------------------------------------------

/// Enable-gated entry point: persist this tick only when capture is enabled. With capture disabled
/// this writes nothing and the in-memory open block is dropped, so re-enabling starts a clean block.
pub fn maybe_sample(
    conn: &Connection,
    state: &mut CaptureState,
    sample: &Sample,
    cfg: &CaptureConfig,
    sample_ts: &str,
    day: &str,
) -> rusqlite::Result<()> {
    if !cfg.capture_enabled {
        state.open = None;
        return Ok(());
    }
    sample_once(conn, state, sample, cfg, sample_ts, day)
}

/// Persist exactly one enabled tick through the TASK-046 store API:
///
/// 1. **Sampling gap** — if the wall-clock gap since the previous tick exceeds `2 ×` the configured
///    interval, record a `sampling_gap` health row (coarse bound, never a title) and drop the open
///    block so the missing span is not backfilled as activity.
/// 2. **No GUI session** — if no frontmost app, record a `no_gui_session` health row and drop the
///    open block. No raw row and no fabricated activity are written.
/// 3. **Active app** — insert one raw observation (title NULL, `absent_no_permission`,
///    `nsworkspace`) and coalesce into the open evidence block, closing + reopening when
///    `(day, app_bundle_id, idle_state)` changes.
fn sample_once(
    conn: &Connection,
    state: &mut CaptureState,
    sample: &Sample,
    cfg: &CaptureConfig,
    sample_ts: &str,
    day: &str,
) -> rusqlite::Result<()> {
    // 1. Sampling-gap detection (covers sleep/suspend without notification observers).
    if let Some(prev) = state.last_sample_ts.clone() {
        let gap = duration_secs(&prev, sample_ts);
        if gap > (cfg.sample_seconds.saturating_mul(2)) as i64 {
            let ev = CaptureHealthEvent {
                day: day.to_owned(),
                start_ts: sample_ts.to_owned(),
                end_ts: None,
                state: health_state::SAMPLING_GAP.to_owned(),
                // Coarse numeric bound only — never a title, app name, path, or secret.
                detail: Some(format!("gap_seconds={gap}")),
                source: source::NSWORKSPACE.to_owned(),
            };
            store::record_capture_health(conn, &ev, sample_ts)?;
            state.open = None;
        }
    }

    let Some(app) = sample.frontmost.as_ref() else {
        // 2. No frontmost app → explain, do not fabricate.
        let ev = CaptureHealthEvent {
            day: day.to_owned(),
            start_ts: sample_ts.to_owned(),
            end_ts: None,
            state: health_state::NO_GUI_SESSION.to_owned(),
            detail: None,
            source: source::NSWORKSPACE.to_owned(),
        };
        store::record_capture_health(conn, &ev, sample_ts)?;
        state.open = None;
        state.last_sample_ts = Some(sample_ts.to_owned());
        return Ok(());
    };

    // 3. Active app: one raw observation per tick.
    let idle = idle_state_for(sample.idle_seconds, cfg);
    let obs = RawObservation {
        sample_ts: sample_ts.to_owned(),
        day: day.to_owned(),
        app_name: app.app_name.clone(),
        app_bundle_id: app.app_bundle_id.clone(),
        window_title: None,
        title_state: title_state::ABSENT_NO_PERMISSION.to_owned(),
        idle_state: idle.to_owned(),
        source: source::NSWORKSPACE.to_owned(),
        capture_health: None,
    };
    store::insert_raw_observation(conn, &obs, cfg.title_mode, sample_ts)?;

    // Coalesce into the open block, or open a new one on any key change (incl. day rollover).
    let continues = state.open.as_ref().is_some_and(|b| {
        b.day == day && b.app_bundle_id == app.app_bundle_id && b.idle_state == idle
    });
    if continues {
        if let Some(b) = state.open.as_mut() {
            b.end_ts = sample_ts.to_owned();
        }
    } else {
        state.open = Some(OpenBlock {
            id: Uuid::new_v4().to_string(),
            day: day.to_owned(),
            start_ts: sample_ts.to_owned(),
            end_ts: sample_ts.to_owned(),
            app_name: app.app_name.clone(),
            app_bundle_id: app.app_bundle_id.clone(),
            idle_state: idle,
        });
    }

    if let Some(b) = state.open.clone() {
        let block = EvidenceBlock {
            id: b.id,
            day: b.day,
            start_ts: b.start_ts.clone(),
            end_ts: b.end_ts.clone(),
            duration_seconds: duration_secs(&b.start_ts, &b.end_ts),
            app_name: b.app_name,
            app_bundle_id: b.app_bundle_id,
            window_title: None,
            title_state: title_state::ABSENT_NO_PERMISSION.to_owned(),
            idle_state: b.idle_state.to_owned(),
            source: source::NSWORKSPACE.to_owned(),
            capture_health: None,
        };
        store::upsert_evidence_block(conn, &block, cfg.title_mode, sample_ts)?;
    }

    state.last_sample_ts = Some(sample_ts.to_owned());
    Ok(())
}

// ----- retention -------------------------------------------------------------------------------

/// Drive bounded retention on a coarse cadence — at most once per calendar day. Delegates to the
/// TASK-046 `prune_expired` primitive, which touches only `active_window_*` tables (C6).
pub fn maybe_prune(
    conn: &Connection,
    state: &mut CaptureState,
    cfg: &CaptureConfig,
    now_ts: &str,
) -> rusqlite::Result<()> {
    let day = day_of(now_ts);
    if state.last_prune_day.as_deref() == Some(day.as_str()) {
        return Ok(());
    }
    store::prune_expired(conn, now_ts, cfg.retention_days)?;
    state.last_prune_day = Some(day);
    Ok(())
}

// ----- native readers (macOS, zero-permission) -------------------------------------------------

/// Read the frontmost application's identity. Returns `None` when no app is frontmost (no GUI
/// session). Uses `NSWorkspace`, which is documented thread-safe — no main-thread hop required.
#[cfg(target_os = "macos")]
fn read_frontmost_app() -> Option<FrontmostApp> {
    use objc2_app_kit::NSWorkspace;
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    Some(FrontmostApp {
        app_name: app.localizedName().map(|s| s.to_string()),
        app_bundle_id: app.bundleIdentifier().map(|s| s.to_string()),
    })
}

/// Seconds since the most recent input event of any tracked type. Reads only event *age* from the
/// combined session counter — it installs no event tap and reads no event content.
#[cfg(target_os = "macos")]
fn idle_seconds() -> f64 {
    use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};
    const EVENTS: [CGEventType; 5] = [
        CGEventType::KeyDown,
        CGEventType::MouseMoved,
        CGEventType::LeftMouseDown,
        CGEventType::RightMouseDown,
        CGEventType::ScrollWheel,
    ];
    EVENTS
        .iter()
        .map(|&et| {
            CGEventSource::seconds_since_last_event_type(
                CGEventSourceStateID::CombinedSessionState,
                et,
            )
        })
        .fold(f64::INFINITY, f64::min)
        .max(0.0)
}

#[cfg(target_os = "macos")]
fn read_sample() -> Sample {
    Sample {
        frontmost: read_frontmost_app(),
        idle_seconds: idle_seconds(),
    }
}

/// Non-macOS fallback: there is no zero-permission frontmost/idle source, so capture yields a
/// no-GUI-session tick. Keeps the loop and tests compiling cross-platform (Vire is mac-first).
#[cfg(not(target_os = "macos"))]
fn read_sample() -> Sample {
    Sample {
        frontmost: None,
        idle_seconds: 0.0,
    }
}

// ----- loop / lifecycle ------------------------------------------------------------------------

/// Spawn the capture loop on a dedicated OS thread (mirrors the Langfuse auto-import scheduler at
/// `lib.rs`). The thread owns its own SQLite connection, makes no renderer/IPC/network call, and is
/// reclaimed on process exit. Capture stays OFF until the enable switch is set.
pub fn spawn(db_path: PathBuf) {
    thread::spawn(move || run_loop(&db_path));
}

fn run_loop(db_path: &Path) {
    let mut state = CaptureState::default();
    let mut conn: Option<Connection> = None;
    loop {
        let interval = run_tick(db_path, &mut conn, &mut state);
        thread::sleep(interval);
    }
}

/// One loop iteration. Resolves config every tick so a settings/env toggle takes effect without a
/// restart; when disabled it samples nothing and touches no native API. Returns the sleep interval.
fn run_tick(db_path: &Path, conn: &mut Option<Connection>, state: &mut CaptureState) -> Duration {
    let fallback = Duration::from_secs(config::DEFAULT_SAMPLE_SECONDS);

    if conn.is_none() {
        match Connection::open(db_path) {
            Ok(c) => {
                let _ = c.busy_timeout(Duration::from_secs(5));
                *conn = Some(c);
            }
            Err(_) => return fallback,
        }
    }
    let Some(c) = conn.as_ref() else {
        return fallback;
    };

    let cfg = match CaptureConfig::from_settings(c) {
        Ok(cfg) => cfg,
        Err(_) => return fallback,
    };
    let interval = Duration::from_secs(cfg.sample_seconds.max(1));

    if !cfg.capture_enabled {
        // Disabled: no native read, no write. Drop any open block so re-enabling starts clean.
        state.open = None;
        return interval;
    }

    let ts = crate::now();
    let day = day_of(&ts);
    let sample = read_sample();
    // Persist through the enable-gated entry point (the gate above already skipped the native read
    // when disabled; this keeps the gate enforced at the single tested call site). Errors are
    // intentionally dropped: a transient write failure must not wedge the loop, degraded states are
    // already first-class health rows, and no app name/title is ever logged.
    let _ = maybe_sample(c, state, &sample, &cfg, &ts, &day);
    let _ = maybe_prune(c, state, &cfg, &ts);
    interval
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_window::model::TitleMode;
    use rusqlite::Connection;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        crate::init_db(&c).unwrap();
        c
    }

    fn cfg() -> CaptureConfig {
        CaptureConfig {
            capture_enabled: true,
            sample_seconds: 5,
            idle_candidate_seconds: config::DEFAULT_IDLE_CANDIDATE_SECONDS,
            away_seconds: config::DEFAULT_AWAY_SECONDS,
            retention_days: config::DEFAULT_RETENTION_DAYS,
            title_mode: TitleMode::Redacted,
        }
    }

    fn app(bundle: &str) -> Option<FrontmostApp> {
        Some(FrontmostApp {
            app_name: Some(format!("App {bundle}")),
            app_bundle_id: Some(bundle.to_owned()),
        })
    }

    fn raw_count(c: &Connection) -> i64 {
        c.query_row("SELECT COUNT(*) FROM active_window_raw_evidence", [], |r| {
            r.get(0)
        })
        .unwrap()
    }

    // ----- §6: idle_state threshold mapping ----------------------------------------------------

    #[test]
    fn idle_state_threshold_mapping() {
        let cfg = cfg(); // 60 / 300
        assert_eq!(idle_state_for(0.0, &cfg), idle_state::ACTIVE);
        assert_eq!(idle_state_for(59.0, &cfg), idle_state::ACTIVE);
        assert_eq!(idle_state_for(90.0, &cfg), idle_state::IDLE_CANDIDATE);
        assert_eq!(idle_state_for(299.0, &cfg), idle_state::IDLE_CANDIDATE);
        assert_eq!(idle_state_for(360.0, &cfg), idle_state::AWAY);
        // Exact boundaries are inclusive (>=).
        assert_eq!(idle_state_for(60.0, &cfg), idle_state::IDLE_CANDIDATE);
        assert_eq!(idle_state_for(300.0, &cfg), idle_state::AWAY);
    }

    // ----- §6: sample-to-store (allowlisted, title NULL) ---------------------------------------

    #[test]
    fn sample_persists_nsworkspace_observation_with_null_title() {
        let c = conn();
        let mut st = CaptureState::default();
        let s = Sample {
            frontmost: app("com.example.editor"),
            idle_seconds: 0.0,
        };
        sample_once(&c, &mut st, &s, &cfg(), "2026-06-21 09:00:00", "2026-06-21").unwrap();

        let (title, title_st, src, idle): (Option<String>, String, String, String) = c
            .query_row(
                "SELECT window_title, title_state, source, idle_state
                   FROM active_window_raw_evidence",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert!(title.is_none(), "window_title must be NULL");
        assert_eq!(title_st, title_state::ABSENT_NO_PERMISSION);
        assert_eq!(src, source::NSWORKSPACE);
        assert_eq!(idle, idle_state::ACTIVE);
    }

    // ----- §6: coalescing ----------------------------------------------------------------------

    #[test]
    fn contiguous_same_key_coalesces_then_change_opens_new_block() {
        let c = conn();
        let mut st = CaptureState::default();
        let cfg = cfg();
        let a = Sample {
            frontmost: app("com.app.a"),
            idle_seconds: 0.0,
        };
        // Two contiguous active ticks for app A → one block.
        sample_once(&c, &mut st, &a, &cfg, "2026-06-21 09:00:00", "2026-06-21").unwrap();
        sample_once(&c, &mut st, &a, &cfg, "2026-06-21 09:00:01", "2026-06-21").unwrap();
        // Idle-state change (same app, now idle) → closes A-active, opens A-idle.
        let a_idle = Sample {
            frontmost: app("com.app.a"),
            idle_seconds: 120.0,
        };
        sample_once(
            &c,
            &mut st,
            &a_idle,
            &cfg,
            "2026-06-21 09:00:02",
            "2026-06-21",
        )
        .unwrap();

        let views =
            store::evidence_blocks_in_range(&c, "2026-06-21", "2026-06-21", TitleMode::Redacted)
                .unwrap();
        assert_eq!(
            views.len(),
            2,
            "one coalesced active block + one idle block"
        );
        let active = views
            .iter()
            .find(|v| v.idle_state == idle_state::ACTIVE)
            .unwrap();
        assert_eq!(active.start_ts, "2026-06-21 09:00:00");
        assert_eq!(active.end_ts, "2026-06-21 09:00:01");
        assert_eq!(active.duration_seconds, 1);
        assert!(views
            .iter()
            .any(|v| v.idle_state == idle_state::IDLE_CANDIDATE));
        // Three ticks, three raw rows (one per tick).
        assert_eq!(raw_count(&c), 3);
    }

    // ----- §6: health (no_gui_session + sampling_gap) ------------------------------------------

    #[test]
    fn no_gui_session_writes_health_row_without_fabricating_activity() {
        let c = conn();
        let mut st = CaptureState::default();
        let s = Sample {
            frontmost: None,
            idle_seconds: 0.0,
        };
        sample_once(&c, &mut st, &s, &cfg(), "2026-06-21 09:00:00", "2026-06-21").unwrap();

        let health = store::capture_health_in_range(&c, "2026-06-21", "2026-06-21").unwrap();
        assert_eq!(health.len(), 1);
        assert_eq!(health[0].state, health_state::NO_GUI_SESSION);
        assert_eq!(raw_count(&c), 0, "no fabricated activity row");
        let ev_count: i64 = c
            .query_row("SELECT COUNT(*) FROM active_window_evidence", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(ev_count, 0, "no evidence block fabricated");
    }

    #[test]
    fn oversized_gap_writes_bounded_sampling_gap_row_without_title() {
        let c = conn();
        let mut st = CaptureState::default();
        let cfg = cfg(); // interval 5 → gap threshold 10s
        let secret_app = "com.secret.bundle";
        let s1 = Sample {
            frontmost: app(secret_app),
            idle_seconds: 0.0,
        };
        sample_once(&c, &mut st, &s1, &cfg, "2026-06-21 09:00:00", "2026-06-21").unwrap();
        // Next tick 30s later (> 2× interval) → sampling_gap.
        let s2 = Sample {
            frontmost: app(secret_app),
            idle_seconds: 0.0,
        };
        sample_once(&c, &mut st, &s2, &cfg, "2026-06-21 09:00:30", "2026-06-21").unwrap();

        let health = store::capture_health_in_range(&c, "2026-06-21", "2026-06-21").unwrap();
        let gap = health
            .iter()
            .find(|h| h.state == health_state::SAMPLING_GAP)
            .expect("a sampling_gap row must be written");
        let detail = gap.detail.clone().unwrap_or_default();
        assert!(
            !detail.contains(secret_app) && !detail.contains("App "),
            "detail must carry only a coarse bound, never a title/app name; got `{detail}`"
        );
    }

    // ----- §6: disabled-by-default -------------------------------------------------------------

    #[test]
    fn disabled_capture_writes_nothing() {
        let c = conn();
        let mut st = CaptureState::default();
        let mut disabled = cfg();
        disabled.capture_enabled = false;
        let s = Sample {
            frontmost: app("com.app.a"),
            idle_seconds: 0.0,
        };
        maybe_sample(
            &c,
            &mut st,
            &s,
            &disabled,
            "2026-06-21 09:00:00",
            "2026-06-21",
        )
        .unwrap();

        assert_eq!(raw_count(&c), 0);
        for table in [
            "active_window_evidence",
            "active_window_capture_health",
            "active_window_raw_evidence",
        ] {
            let n: i64 = c
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "disabled capture must write no rows to {table}");
        }
    }

    #[test]
    fn config_default_is_disabled() {
        // Fresh DB, no env override controlling the switch: capture must default OFF.
        std::env::remove_var("VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED");
        let c = conn();
        let resolved = CaptureConfig::from_settings(&c).unwrap();
        assert!(
            !resolved.capture_enabled,
            "capture must be disabled by default"
        );
    }

    // ----- §6: no-AX / no-title structural -----------------------------------------------------

    #[test]
    fn capture_path_makes_no_ax_tap_or_window_list_call() {
        // Structural proof by absence-of-construction. Needles are split so this test's own source
        // (included below) cannot match itself; the only way a joined token appears is a real call.
        let src = include_str!("capture.rs");
        let forbidden = [
            format!("{}{}", "AXIsProcess", "Trusted"),
            format!("{}{}", "AXUIElement", "CopyAttributeValue"),
            format!("{}{}", "CGEvent", "TapCreate"),
            format!("{}{}", "CGEvent", "Tap"),
            format!("{}{}", "kAX", "TitleAttribute"),
            format!("{}{}", "CGWindowList", "CopyWindowInfo"),
        ];
        for needle in &forbidden {
            assert!(
                !src.contains(needle.as_str()),
                "capture path must contain no `{needle}` call (zero-permission invariant)"
            );
        }
    }

    // ----- §6: retention driver ----------------------------------------------------------------

    #[test]
    fn retention_driver_prunes_only_active_window_rows() {
        let c = conn();
        let mut st = CaptureState::default();
        // Sentinel approved-time row (C6): must survive retention untouched.
        let project = crate::create_project_repo(
            &c,
            crate::ProjectInput {
                name: "Sentinel".into(),
                notes: None,
            },
        )
        .unwrap();
        crate::create_entry_repo(
            &c,
            crate::TimeEntryInput {
                project_id: project.id,
                date: "2026-01-01".into(),
                start_time: "09:00".into(),
                end_time: "10:00".into(),
                note: None,
            },
        )
        .unwrap();
        // Expired raw row (> 30 days before now).
        store::insert_raw_observation(
            &c,
            &RawObservation {
                sample_ts: "2026-05-01 10:00:00".into(),
                day: "2026-05-01".into(),
                app_name: None,
                app_bundle_id: Some("com.old".into()),
                window_title: None,
                title_state: title_state::ABSENT_NO_PERMISSION.into(),
                idle_state: idle_state::ACTIVE.into(),
                source: source::NSWORKSPACE.into(),
                capture_health: None,
            },
            TitleMode::Redacted,
            "2026-05-01 10:00:00",
        )
        .unwrap();

        let mut cfg = cfg();
        cfg.retention_days = 30;
        maybe_prune(&c, &mut st, &cfg, "2026-06-21 00:00:00").unwrap();

        assert_eq!(raw_count(&c), 0, "expired active-window row pruned");
        let te: i64 = c
            .query_row("SELECT COUNT(*) FROM time_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(te, 1, "time_entries sentinel must survive (C6)");

        // Second call same day is a no-op (coarse cadence).
        assert_eq!(st.last_prune_day.as_deref(), Some("2026-06-21"));
        maybe_prune(&c, &mut st, &cfg, "2026-06-21 12:00:00").unwrap();
    }

    // ----- §0: live zero-permission probe (skipped unless explicitly requested) ----------------

    #[test]
    fn zero_permission_live_probe() {
        // §0 empirical gate. Skipped in normal/CI `cargo test` so the suite stays synthetic-only.
        // Run on the target Mac with VIRE_TASK048_LIVE_PROBE=1 to exercise the real NSWorkspace +
        // CGEventSource readers and confirm no TCC prompt blocks them. Prints states/counts only —
        // never an app name or title.
        if std::env::var("VIRE_TASK048_LIVE_PROBE").is_err() {
            return;
        }
        let sample = read_sample();
        let st = idle_state_for(sample.idle_seconds, &cfg());
        println!(
            "LIVE_PROBE source={} frontmost_present={} idle_seconds={:.1} idle_state={}",
            source::NSWORKSPACE,
            sample.frontmost.is_some(),
            sample.idle_seconds,
            st
        );
        assert!(
            sample.idle_seconds >= 0.0,
            "idle age must be a real non-negative number from CGEventSource"
        );
    }
}
