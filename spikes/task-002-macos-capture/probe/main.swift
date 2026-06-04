// TASK-002 macOS capture feasibility probe — NON-SHIPPING, REFERENCE-ONLY.
//
// This probe is NOT a member of any shipped build target. It is not imported by
// `src/`, `src-tauri/src/`, or `observability/`, and it does not touch the legacy
// manual-tracker surface. It exists only to exercise the macOS capture APIs the
// TASK-002 spike evaluates, and to make the feasibility findings reproducible by
// hand. Delete this whole `spikes/task-002-macos-capture/` tree once TASK-003
// consumes the findings.
//
// PRIVACY (SEC-001): this probe NEVER prints or persists a real window/app title.
// Titles are reduced to a category + character count only. App bundle ids/names
// are allowlisted fields, but to keep committed/console output safe by default the
// probe prints the bundle id and a coarse name-length, not arbitrary strings it did
// not originate. Nothing here writes to a file; redirect stdout to a *.log under this
// directory (gitignored) if you want an ephemeral local record, then delete it.
//
// Build (compile-only, recommended in CI / headless):  swiftc -typecheck probe/main.swift
// Build executable (manual, on a real login session):   swiftc probe/main.swift -o probe/probe
// Run (manual, interactive GUI session required):        ./probe/probe
//
// Required frameworks auto-link: AppKit, ApplicationServices, CoreGraphics.

import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

// MARK: - Redaction (SEC-001 guardrail)

/// Reduce any free-form, possibly-sensitive string to a non-reversible shape:
/// presence + length bucket. The actual characters are never emitted.
func redact(_ value: String?) -> String {
    guard let value, !value.isEmpty else { return "<none>" }
    let bucket: String
    switch value.count {
    case 0...20: bucket = "short"
    case 21...60: bucket = "medium"
    default: bucket = "long"
    }
    return "<redacted len=\(value.count) bucket=\(bucket)>"
}

// MARK: - 1. NSWorkspace / NSRunningApplication frontmost app

struct FrontmostApp {
    let bundleId: String
    let nameRedacted: String
    let pid: pid_t
}

/// Frontmost-app observation. Empirically requires NO TCC permission grant on
/// macOS 15 — this is the central FB-002 finding the spike confirms.
func sampleFrontmostApp() -> FrontmostApp? {
    guard let app = NSWorkspace.shared.frontmostApplication else { return nil }
    return FrontmostApp(
        bundleId: app.bundleIdentifier ?? "<unknown-bundle>",
        nameRedacted: redact(app.localizedName),
        pid: app.processIdentifier
    )
}

// MARK: - 2. AXUIElement focused window / title

enum AXTitleResult {
    case granted(titleRedacted: String)   // AX trusted, title read (redacted)
    case noFocusedWindow                  // AX trusted, but no focused window
    case unavailable(AXError)             // AX call failed (e.g. app not AX-scriptable)
    case permissionDenied                 // process is not AX-trusted
}

/// Read the focused-window title for a pid via the Accessibility API.
/// Requires the Accessibility (AX) TCC permission; degrades explicitly otherwise.
func focusedWindowTitle(pid: pid_t) -> AXTitleResult {
    guard AXIsProcessTrusted() else { return .permissionDenied }

    let appElement = AXUIElementCreateApplication(pid)
    var focusedWindowRef: CFTypeRef?
    let winErr = AXUIElementCopyAttributeValue(
        appElement, kAXFocusedWindowAttribute as CFString, &focusedWindowRef)

    if winErr == .noValue || winErr == .attributeUnsupported {
        return .noFocusedWindow
    }
    guard winErr == .success, let focusedWindowRef else {
        return .unavailable(winErr)
    }

    // CFTypeRef -> AXUIElement
    let window = focusedWindowRef as! AXUIElement
    var titleRef: CFTypeRef?
    let titleErr = AXUIElementCopyAttributeValue(
        window, kAXTitleAttribute as CFString, &titleRef)

    guard titleErr == .success else { return .unavailable(titleErr) }
    return .granted(titleRedacted: redact(titleRef as? String))
}

// MARK: - 3. Quartz Window Services fallback

struct QuartzSummary {
    let onScreenWindowCount: Int
    let windowsExposingNameCount: Int   // count only; the name strings are never read out
    let screenRecordingLikelyGranted: Bool
}

/// Quartz on-screen window list. The `kCGWindowName` field is only populated when
/// Screen Recording permission is granted (macOS 10.15+). We measure *whether* names
/// are exposed (a permission proxy) without ever emitting a name value.
func sampleQuartz() -> QuartzSummary {
    let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
    let info = (CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]]) ?? []
    let named = info.filter { entry in
        if let name = entry[kCGWindowName as String] as? String { return !name.isEmpty }
        return false
    }.count
    return QuartzSummary(
        onScreenWindowCount: info.count,
        windowsExposingNameCount: named,
        // Heuristic only: if any other-app window exposes a name, Screen Recording is on.
        screenRecordingLikelyGranted: named > 0
    )
}

// MARK: - 4. Idle / away via CGEventSource last-event age

enum ActivityState: String {
    case active
    case idleCandidate = "idle_candidate"
    case away
}

/// Directional thresholds (seconds). Final values are tuned with Janne in TASK-005.
let idleCandidateThreshold: TimeInterval = 60      // 1 min of no input
let awayThreshold: TimeInterval = 300              // 5 min of no input

/// Seconds since the last user input event. Uses the minimum across input event
/// types (robust and avoids relying on the `kCGAnyInputEventType` sentinel).
func secondsSinceLastInput() -> TimeInterval {
    let types: [CGEventType] = [
        .keyDown, .mouseMoved, .leftMouseDown, .rightMouseDown, .scrollWheel,
    ]
    let ages = types.map {
        CGEventSource.secondsSinceLastEventType(.combinedSessionState, eventType: $0)
    }
    return ages.min() ?? 0
}

func activityState(forIdleSeconds seconds: TimeInterval) -> ActivityState {
    if seconds >= awayThreshold { return .away }
    if seconds >= idleCandidateThreshold { return .idleCandidate }
    return .active
}

// MARK: - Driver (single sample; manual run only)

func runOnce() {
    let iso = ISO8601DateFormatter()
    let stamp = iso.string(from: Date())
    print("# vire TASK-002 capture probe — single sample @ \(stamp)")
    print("# (redacted output; no real titles emitted)")

    // 1. Frontmost app (no permission expected)
    if let front = sampleFrontmostApp() {
        print("nsworkspace.frontmost: bundle=\(front.bundleId) name=\(front.nameRedacted) pid=\(front.pid)")
    } else {
        print("nsworkspace.frontmost: <none> (no GUI session?)")
    }

    // 2. AX focused-window title (needs Accessibility)
    print("ax.trusted: \(AXIsProcessTrusted())")
    if let front = sampleFrontmostApp() {
        switch focusedWindowTitle(pid: front.pid) {
        case .granted(let t): print("ax.focusedTitle: granted \(t)")
        case .noFocusedWindow: print("ax.focusedTitle: capture_health=no_focused_window")
        case .unavailable(let e): print("ax.focusedTitle: capture_health=unavailable axerr=\(e.rawValue)")
        case .permissionDenied: print("ax.focusedTitle: capture_health=permission_denied")
        }
    }

    // 3. Quartz fallback (kCGWindowName needs Screen Recording)
    let q = sampleQuartz()
    print("quartz.onScreenWindows: \(q.onScreenWindowCount)")
    print("quartz.windowsExposingName: \(q.windowsExposingNameCount)")
    print("quartz.screenRecordingLikelyGranted: \(q.screenRecordingLikelyGranted)")

    // 4. Idle / away
    let idle = secondsSinceLastInput()
    print(String(format: "idle.secondsSinceLastInput: %.1f", idle))
    print("idle.state: \(activityState(forIdleSeconds: idle).rawValue)")
}

runOnce()
