# TASK-048 — Active-window zero-permission app + idle capture seam

- **Change:** `task-048-active-window-app-idle-capture`
- **Capability:** `active-window-capture` (macOS Evidence Capture boundary — the *zero-permission* slice)
- **Kind:** First capture slice. Adds a **Rust-core capture loop** that records the **frontmost
  application identity** and **idle/away state** into the already-built TASK-046 store. **No window
  titles, no Accessibility, no Screen Recording, no permissions UI, no suggestion wiring, no network,
  no renderer/IPC surface.**
- **Tier / gate:** L2 · SEC-001 (field-allowlist preserved at the capture boundary), SEC-007 input
  (permission burden = **zero grants**), APP-005 input.
- **Smallest unblocked slice after storage.** This realizes the *"TASK-036 + TASK-037a"* first slice
  named in `task-035-active-window-tracking/design.md` §4 — *"a zero-permission, locally-stored
  active-app + idle timeline (no titles)"* — now that the storage foundation (TASK-035's TASK-036,
  shipped as **TASK-046**, merged) exists as the sink. The AX-title path, the permissions/privacy UI,
  the suggestion integration, and the L2 security-acceptance gate remain separate downstream tasks and
  are referenced descriptively, not by stale IDs.

## Why

TASK-046 built the **Local SQLite Store** — `active_window_raw_evidence` / `active_window_evidence` /
`active_window_capture_health` plus the typed, allowlist-enforcing write/read/prune API
(`src-tauri/src/active_window/store.rs`) — but nothing writes to it. The tables ship empty and the app
is still in *"Manual Mode / Capture deferred"* (`lib.rs:680`). The smallest, lowest-risk way to make
the timeline real is to fill the store with the **two signals that need no macOS permission at all**:

- **NSWorkspace `frontmostApplication`** → active-app name + bundle id (no TCC grant — verified, §refs
  in `design.md`).
- **CGEventSource last-event age** → `active` / `idle_candidate` / `away` (no TCC grant — reads event
  *age* only, creates no event tap, reads no event content).

This is exactly the slice TASK-035 sized as shippable-on-its-own: useful and reviewable immediately,
with **titles, UI, and integration layered after**. It takes on **none** of the high-risk native
packaging work (`externalBin` / codesign / notarization / TCC) that TASK-035 deliberately isolated to
the AX-title task — see the build-shape decision in `design.md` §3 and the routed `feedback_to_ba[]`
item **FB-048** in `arch-review.md`.

## What changes

- Adds a **capture loop** to the existing `src-tauri/src/active_window/` module (new
  `capture.rs` + a `config` flag), started from the Tauri `.setup()` hook on a **dedicated background
  OS thread** — the same idiomatic pattern the Langfuse auto-import scheduler already uses
  (`lib.rs:1189`). Each tick:
  - reads `NSWorkspace.frontmostApplication` (`app_name` = `localizedName`, `app_bundle_id` =
    `bundleIdentifier`, `source = nsworkspace`);
  - reads CGEventSource idle age and maps it to `idle_state` via the TASK-002 §3.4 thresholds
    (`idle_candidate` ≥ 60 s, `away` ≥ 300 s — configurable);
  - sets `window_title = NULL` and `title_state = absent_no_permission` **always** (no AX in this
    slice);
  - writes via the **existing TASK-046 store API** (`insert_raw_observation`, `upsert_evidence_block`,
    `record_capture_health`) — no new table, no generic writer, redaction gate untouched.
- Records degraded states as first-class `capture_health` rows (C4) reachable **without** any grant:
  `no_gui_session` (no frontmost app) and `sampling_gap` (wall-clock gap > 2× the tick interval,
  which covers sleep/suspend without needing notification observers). `detail` carries a coarse code
  only, never a title.
- Adds a **capture-enable switch** — settings key `active_window_capture_enabled` /
  env `VIRE_ACTIVE_WINDOW_CAPTURE_ENABLED`, **default OFF** — so capture never runs silently before
  the transparency/consent UI (the downstream permissions task) exists. Janne enables it via the
  settings key / env for real-world dogfooding now.
- Drives the existing `prune_expired` retention primitive on the capture cadence (C6) so raw evidence
  stays bounded; it touches **only** `active_window_*` tables and never `time_entries`.
- Adds the **two Apple-binding crates** the in-process calls require (`objc2-app-kit` for NSWorkspace,
  `objc2-core-graphics` for CGEventSource — the maintained `madsmtm/objc2` ecosystem). These are the
  only new dependencies; the security gate (SW-5) scans them.

## Impact

- **Affected specs:** introduces the `active-window-capture` capability (zero-permission capture loop,
  store-only write path, opt-in switch, degraded-state health, structural no-title/no-prohibited-data).
- **Affected code:** `src-tauri/src/active_window/capture.rs` (+ `config.rs` flag, `mod.rs` wiring),
  one capture-loop spawn in `lib.rs` `.setup()`, and two new Cargo dependencies. **No** change to
  `tauri.conf.json` (CSP and the absent `externalBin` stay as-is), `src/main.ts`, the renderer, or the
  `invoke_handler!` surface. `capture_status` is reconsidered only insofar as the loop now writes when
  enabled; the renderer string is **not** changed in this slice (no renderer exposure — deferred to
  the permissions/privacy UI task).
- **Downstream:** unblocks the **AX-title task** (adds `kAXTitle` under one Accessibility grant — and
  is where the Swift sidecar / `externalBin` / codesign / TCC-trust work belongs, per FB-048), the
  **permissions/privacy-UI task** (user-facing capture toggle + transparency, owns FB-002), and the
  **suggestion-integration task** (reads normalized blocks into the task-032 pipeline).
- **Guardrails preserved:** zero TCC grants requested; no AX, no Screen Recording, no event tap;
  local-only evidence; no SaaS/cloud sync; no raw activity egress; locked `connect-src ipc:` CSP
  unchanged; no new network client; no `externalBin`/sidecar in this slice; positive field allowlist +
  structural non-collection intact; legacy manual `time_entries` surface untouched.
