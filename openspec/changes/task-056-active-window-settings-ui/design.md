# Design — TASK-056 active-window capture Settings + privacy/status UI

Technical design SW-2 implements from. Scope, rationale, and `feedback_to_ba[]` are in `arch-review.md`.
This is the **permissions/privacy-UI leg** TASK-046/048 deferred: it *exposes and controls* the
already-shipped zero-permission capture — it does **not** capture, change capture behavior, request any
permission, or reach the network. Do the minimum; reuse the Langfuse/env-mapping settings pattern rather
than inventing a new one.

## 0. Where this sits

```
settings table  (active_window_capture_enabled, _sample_seconds, _idle_candidate_seconds,
   ▲   │          _idle_away_seconds, _retention_days)   ← CaptureConfig::from_settings reads these each tick
   │   ▼
set_/get_active_window_capture_settings  IPC (A)          active_window::store.capture_status_snapshot (B)
   │   │  validate + upsert / resolve view                   COUNT/MAX over the 3 existing tables (read-only)
   ▼   ▼                                                        ▲
Settings "Active-window capture" panel (C)  ── toggle · interval · idle · retention · STATUS/HEALTH · privacy table
   │
   └─ writes settings → capture loop picks up the new config on its NEXT tick (no restart; loop already
      re-reads CaptureConfig::from_settings every tick — lib.rs .setup() spawn, capture.rs run_tick)
```

The capture loop (TASK-048) is spawned unconditionally at `.setup()` and **re-reads `CaptureConfig` on
every tick**, sampling nothing while disabled. So this task needs **no** thread lifecycle control: writing
the `settings` rows is sufficient and takes effect on the next tick. The UI is a thin, honest window onto
config the backend already owns.

## 1. IPC contract (Workstream A)

Three commands, registered in `generate_handler!` (`lib.rs:1205`), mirroring the shape of the Langfuse
settings commands (`get_langfuse_settings` / `set_langfuse_settings`).

```rust
#[tauri::command] fn get_active_window_capture_settings(state) -> CmdResult<CaptureSettingsView>
#[tauri::command] fn set_active_window_capture_settings(state, input: CaptureSettingsInput) -> CmdResult<CaptureSettingsView>
```

```rust
pub struct CaptureSettingsView {
  platform_supported: bool,          // cfg!(target_os = "macos"); false ⇒ controls disabled in UI
  capture_enabled: bool,
  sample_seconds: u64,
  idle_candidate_seconds: u64,
  idle_away_seconds: u64,
  retention_days: i64,
  title_mode: String,                // ALWAYS "redacted" — informational; not user-editable (§4)
  status: CaptureStatusView,         // §2 (Workstream B)
}

pub struct CaptureSettingsInput {    // serde DROPS any non-allowlisted key (runtime_observer discipline)
  capture_enabled: bool,
  sample_seconds: u64,
  idle_candidate_seconds: u64,
  idle_away_seconds: u64,
  retention_days: i64,
}
```

- **get** = `CaptureConfig::from_settings(conn)` → the view, plus `platform_supported = cfg!(macos)` and
  `status` from §2. Pure read; never mutates.
- **set** = **validate, then upsert** the exact `settings` keys `CaptureConfig` reads
  (`active_window_capture_enabled` as `"true"`/`"false"`, `active_window_sample_seconds`,
  `active_window_idle_candidate_seconds`, `active_window_idle_away_seconds`,
  `active_window_retention_days`), each via `INSERT … ON CONFLICT(key) DO UPDATE`. Then re-resolve and
  return the fresh view so the UI reflects exactly what the backend will act on. `title_mode` is **not**
  accepted from input (§4). One transaction.

**Validation (safe bounds — reject, do not silently clamp; return a clear error string):**

| Field | Rule | Rationale |
| --- | --- | --- |
| `sample_seconds` | `1 ≤ n ≤ 3600` | 0 would busy-spin / disable cadence (the backend already floors >0; the UI must not offer 0). Upper cap keeps sampling meaningful. |
| `idle_candidate_seconds` | `1 ≤ n ≤ 86_400` | Idle threshold must be positive. |
| `idle_away_seconds` | `idle_candidate_seconds < n ≤ 86_400` | **Ordering invariant**: away must be later than idle-candidate, else the state machine is ill-defined. |
| `retention_days` | `1 ≤ n ≤ 3650` | ≥1 day; sane upper cap. |
| `capture_enabled` | bool | Persisted as truthy `"true"`/`"false"` (the backend `parse_bool` accepts `true`). |

The backend `positive_u64_setting` already ignores a `≤0`/garbage stored value (falls through to default),
so validation here is defense-in-depth + a good UX error — a bad write can never disable sampling by
yielding a zero cadence. Enabling capture is only ever an explicit `capture_enabled = true` from the user;
**default stays OFF** and this task never writes `true` on its own.

## 2. Capture status / health projection (Workstream B) — read-only, no schema change

One read-only function over the **existing** TASK-046 tables — no new column, no write path:

```rust
pub fn capture_status_snapshot(conn, now_day: &str, retention_from_day: &str) -> rusqlite::Result<CaptureStatusView>;

pub struct CaptureStatusView {
  last_sample_ts: Option<String>,        // MAX(sample_ts) FROM active_window_raw_evidence
  samples_today: i64,                    // COUNT(*) raw WHERE day = now_day
  evidence_blocks_retained: i64,         // COUNT(*) active_window_evidence WHERE day >= retention_from_day
  open_health: Vec<HealthMarker>,        // capture_health WHERE end_ts IS NULL  (ongoing degraded states)
  recent_health: Vec<HealthMarker>,      // last N capture_health rows (state + coarse detail code only)
}
pub struct HealthMarker { state: String, since_or_start_ts: String, detail: Option<String> }
```

- All fields are aggregates/state codes over allowlisted columns; `detail` is the already-200-byte-bounded
  coarse reason code (never a title/path/command — TASK-046 invariant). Nothing here can carry a prohibited
  value; a schema/structural test already guarantees the columns exist and no prohibited column does.
- Drives the UI's *"Capture is healthy"* vs *"Capture degraded: `no_gui_session` since 14:02"* readout, so
  **every gap is explained, never a silent empty box** (TASK-002 §4 posture). If capture is disabled, the
  status reads *"off"* and the readout invites enabling it; if enabled but no samples yet, it says so.

## 3. Frontend (Workstream C)

- **"Active-window capture" panel** in `renderSettings()` (`src/main.ts:75`), placed after the Storage/
  privacy panel and before the Langfuse panel. Loaded via `get_active_window_capture_settings`; saved via
  `set_active_window_capture_settings`. Controls: an **enable toggle**, four numeric inputs (sample interval,
  idle-candidate seconds, away seconds, retention days), a **Save** button, and a live **status/health**
  block from `status`. Bind fn mirrors `bindLangfuse()`; on save, inline-validate (same bounds as §1) before
  the call and surface the backend error verbatim on failure. On success, `rerender()` so status refreshes.
- **Privacy table (truthful, source-of-truth copy).** A two-column table — *Captured* (frontmost app
  bundle id + display name, coarse idle state `active`/`idle_candidate`/`away`, observation timestamp) vs
  *Never captured* (window titles, Accessibility tree, screen pixels, keystrokes, mouse content, URLs,
  file paths, terminal command bodies, clipboard, prompts/responses, secrets). Mirrors the table in
  `docs/active-window-capture.md` §"Privacy posture" so doc and UI agree. States plainly: zero macOS
  permissions requested; local SQLite only; never sent over the network.
- **Correct the inaccurate copy (load-bearing).** The current static sentence in `renderSettings()` —
  *"Automatic capture is deferred. This app does not collect … active windows, idle state …"* — is
  **replaced** with copy driven by the real setting: when capture is on it states what is being captured
  (and links to the privacy table); when off it says capture is available but disabled. The Today `capture()`
  banner likewise reflects actual enabled/deferred state rather than the hard-coded v0.1 string. This is a
  correctness fix (the app currently asserts something false once capture is enabled), not polish.
- **macOS-only.** When `platform_supported === false`, render the panel read-only with an *"Active-window
  capture is available on macOS only"* note and disabled controls.
- **Builder split (optional, recommended).** Extract pure string/validation builders into
  `src/active-window-settings-ui.ts` (like `env-mapping-ui.ts` / `langfuse-settings.ts`) so the panel,
  validation, and status/health copy get pure `tests/*.test.mjs` coverage without a running backend.

## 4. `title_mode` stays redacted and unexposed (the load-bearing boundary)

The store defines a `title_mode='stored'` opt-in (TASK-046 §3) *"for the future privacy UI to enable."*
This task is that UI — but it **deliberately does not expose it.** `stored` mode only matters if something
captures a window title, and title capture requires an **Accessibility (AX) grant** that TASK-048 never
takes and this task does not add. Exposing a "store window titles" toggle here would be **misleading**
(nothing writes a title) and would imply a permission this task must not request. So:

- `title_mode` is shown **read-only** as *"Window titles: never captured"* and is **not** accepted by
  `set_active_window_capture_settings`.
- Enabling real title capture (AX grant, `kAXTitle`, sidecar/`externalBin`/TCC) belongs to the **AX-title
  task** (FB-048), which owns both the capture and the matching consent UI. This is routed to ba-architect
  as proposed **DEC-044** (non-blocking) to ratify the boundary.

## 5. Effect model & concurrency

- **No restart, no thread control.** The capture thread already re-reads `CaptureConfig::from_settings`
  each tick; a settings write is picked up on the next tick (≤ `sample_seconds`). The UI may note *"changes
  apply within a few seconds."* This task adds **no** thread start/stop, no channel, no signal.
- **Single connection discipline.** Commands take the shared `AppState` DB mutex exactly like the existing
  settings commands; the capture loop opens its own throwaway connections (TASK-048). SQLite serializes the
  settings-row write vs the loop's read; there is no shared in-memory state to coordinate.

## 6. Guarantees checklist (must all hold)

- **capture stays OFF by default** — this task writes `capture_enabled=true` only on an explicit user
  toggle; it never auto-enables. (DEC-019 posture)
- **zero new permissions** — no AX, no Screen Recording, no event tap, no TCC prompt; `title_mode` stays
  `redacted` and unexposed. (SEC-007, C2)
- **secret-free / no titles** — the view, status, and health carry only bundle id/name, coarse idle,
  timestamps, counts, and bounded state/detail codes; no window title, screenshot, URL, path, keystroke, or
  credential (structurally absent upstream). (SEC-012/SEC-001)
- **truthful transparency** — the Settings copy and Today banner reflect the *actual* capture state; the
  inaccurate pre-capture claim is removed. (FB-002)
- **every gap explained** — degraded/no-sample states are named with cause + since-time, never a silent
  empty box. (TASK-002 §4, DEC-004 posture)
- **additive-only, local, reversible** — no schema/table/column/migration; reads/writes existing `settings`
  rows and reads `active_window_*`; no network; no CSP change. (DEC-001/017)

## 7. Tests

- **A (backend):** `set` validation — reject `sample_seconds=0`, `idle_away_seconds ≤ idle_candidate_seconds`,
  `retention_days=0`, and out-of-range values with clear errors; a valid `set` upserts exactly the five
  `settings` keys and the subsequent `get` (and `CaptureConfig::from_settings`) resolves them; enabling then
  disabling round-trips; `title_mode` is never mutated by `set`. `cargo test`/`fmt`/`clippy` clean.
- **B (backend):** `capture_status_snapshot` on a seeded fixture — correct `samples_today`, retained-block
  count, `MAX(sample_ts)`, open-health (end_ts NULL) vs recent-health selection; empty DB → zero counts +
  `None` last sample (not an error). Structural test still holds: no prohibited column is read/returned.
- **C (frontend):** pure-builder `tests/*.test.mjs` for the panel, the four-field validation (bounds +
  ordering), the status/health copy (healthy / degraded-with-cause / off / no-samples-yet), the privacy
  table, and the macOS-only disabled state. `npm run test:frontend` + `npm run build` green.
- **Physical-Mac smoke (the mandated testable route):** documented checklist in `tasks.md` §Smoke — enable
  via the UI → status shows *running* and `last_sample_ts` advances, evidence blocks accrue, health surfaces
  any gap; change interval/thresholds → next-tick effect; **disable → sampling stops**; confirm **no TCC
  prompt** ever appears and titles stay `NULL`. Reuses the TASK-048 `VIRE_TASK048_LIVE_PROBE` posture,
  now driven from the UI.
- **Validation:** `openspec validate task-056-active-window-settings-ui --strict` passes.

## 8. Out of scope (guard against creep)

Any capture / macOS / TCC / AX / Screen-Recording change or new native code; window titles or a
`title_mode='stored'` toggle; a Swift sidecar / `externalBin` / codesign / notarization; the suggestion/
review pipeline, app→project mapping, and Suggestions view (TASK-055 owns those); writing `time_entries`;
CSV/report/summary changes; any new table/column/migration; any network client or CSP change; changes to
the manual `time_entries` form or the Langfuse settings surface.
