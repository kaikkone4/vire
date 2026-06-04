# spikes/task-002-macos-capture

**Status:** non-shipping, reference-only spike probe for TASK-002 (macOS capture
feasibility). **Not** a member of any shipped build target. Delete this whole tree
once TASK-003 has consumed the feasibility findings.

## What this is

A single-file Swift probe (`probe/main.swift`) that exercises the macOS capture APIs
the spike evaluates, so the findings in
[`../../openspec/changes/task-002-macos-capture-feasibility/capture-feasibility-report.md`](../../openspec/changes/task-002-macos-capture-feasibility/capture-feasibility-report.md)
are reproducible by hand. It is the supporting evidence for that report, not a product.

Signals exercised:

1. **NSWorkspace / NSRunningApplication** — frontmost app bundle id + (redacted) name + pid.
2. **AXUIElement** — focused-window title via the Accessibility API, with explicit
   `permission_denied` / `no_focused_window` / `unavailable` capture-health states.
3. **Quartz Window Services** — on-screen window count and whether `kCGWindowName` is
   exposed (a Screen Recording permission proxy) — never reads the name strings.
4. **CGEventSource** — seconds since last input → `active` / `idle_candidate` / `away`.

## Isolation guarantees

- Lives outside `src/`, `src-tauri/src/`, and `observability/`.
- Not referenced by `Cargo.toml`, `tauri.conf.json`, `package.json`, or any build graph.
- Does not import, migrate, reuse, or modify the legacy manual-tracker surface
  (`time_entries`, manual-entry view, stopwatch CRUD) — that stays reference-only and
  its fate is TASK-003's decision, not this spike's.

## Privacy (SEC-001) — read before running

- The probe **never** prints or persists a real window/app title. Titles are reduced to
  `<redacted len=N bucket=...>` — presence + length only, non-reversible.
- It writes **nothing** to disk on its own. If you want an ephemeral local record,
  redirect stdout to a `*.log` in this directory (gitignored) and **delete it when done**:

  ```sh
  swiftc probe/main.swift -o probe/probe        # build (real login session)
  ./probe/probe > out.local.log                 # run once; out.local.log is gitignored
  # inspect, then:
  rm -f out.local.log probe/probe
  ```

- No screenshots, keystrokes, screen pixels, browser contents, URLs, command bodies,
  shell history, prompt/response text, env dumps, or secrets are read or written.
- No network egress.

## Compile-only check (CI / headless safe)

```sh
swiftc -typecheck probe/main.swift   # validates the API surface; captures nothing
```

This is how the spike validated API availability without sampling real activity.
Running the executable requires an interactive GUI login session and (for AX titles)
the Accessibility permission; that is a **manual** validation step, see the manual
validation matrix in the feasibility report.
