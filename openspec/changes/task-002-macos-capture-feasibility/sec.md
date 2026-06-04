# SEC — Security Review (SW-5)

- **Change:** `task-002-macos-capture-feasibility` (TASK-002)
- **Branch:** `feat/task-002-macos-capture-feasibility` · **PR:** [#7](https://github.com/kaikkone4/vire/pull/7)
- **Tier:** L2 · **Gate context:** APP-005 (SEC-001 primary, SEC-007; SEC-002/003 N/A — no network, no credentials)
- **Reviewer role:** Security Agent (SW-5) · **Date:** 2026-06-04
- **Verdict:** **PASS**

This change is a feasibility spike: OpenSpec docs + a non-shipping, reference-only Swift
probe (`spikes/task-002-macos-capture/probe/main.swift`). No product runtime, schema, or
dependency manifests are modified.

---

## 1. Scanner results (Tier 1 / L2)

The Tier 1 binaries (gitleaks, OSV-scanner, Trivy, SonarQube CE) are **not installed** in
this environment. Manual equivalents were run; results below. The change surface is
docs + one isolated Swift file with no dependency manifests and no container, so the
automated stack's blocking surface is effectively empty regardless.

| Scanner | L2 active? | Method | Result |
| --- | --- | --- | --- |
| **gitleaks** (secrets) | yes | Pattern scan of full `main...HEAD` diff for `ghp_/gho_/ghu_/ghr_/github_pat_/sk-ant-/sk-proj-/xox*/AKIA…/PRIVATE KEY/password=/secret=/api_key=` | **PASS** — no secret detected. (One match in `qa.md:727` is documentation text describing a passing scan, not a credential.) |
| **OSV-scanner** (dep CVEs) | yes (L2) | `git diff --name-only` for `Cargo.*`, `package*.json`, `requirements*.txt`, `go.*` | **PASS / N-A** — no dependency manifest changed; probe imports only system frameworks (AppKit, ApplicationServices, CoreGraphics, Foundation). No CVE surface. |
| **Trivy** (image vulns) | yes (L2) | n/a | **N-A** — no container image or Dockerfile in this change. |
| **SonarQube CE** (quality gate) | yes (L2) | Manual secure-code review (§2) in lieu of scanner binary | **PASS** — no security hotspot/anti-pattern; see §2. |

Supplementary checks:
- `swiftc -typecheck spikes/task-002-macos-capture/probe/main.swift` → **PASS** (compile-clean, captures nothing).
- Tracked files under spike tree: `.gitignore`, `README.md`, `probe/main.swift` only — **no committed `*.log`, `*.o`, or `probe/probe` binary** (all gitignored).

---

## 2. Manual secure-code review (probe + isolation)

### 2.1 Sensitive-data capture / leakage — PASS

| Risk checked | Finding |
| --- | --- |
| Committed real window/app titles | **None.** `redact()` reduces every title/name to `<redacted len=N bucket=…>` (presence + length, non-reversible). Driver only prints redacted values; report uses generic app names (e.g. "Xcode") as examples, not captured private titles. |
| Screenshots / screen pixels | **None.** No `CGWindowListCreateImage`, `CGDisplayCreateImage`, `CGDisplayStream`, or `SCStream`. Quartz path reads window-list metadata only and counts name-bearing windows without reading the name strings. |
| Keystroke / input capture | **None.** No `CGEventTap`. `CGEventSource.secondsSinceLastEventType` reads event *age* only — no event contents, coordinates, or keycodes. |
| Prompt/response, command bodies, env, secrets | **None reachable.** No env reads, no shell-out, no AI-tool text. Chosen APIs structurally cannot reach these (matches report §7.2 non-collection list). |

### 2.2 Persistence / logging — PASS

- Probe writes **nothing** to disk on its own (no `FileManager.write`, `FileHandle`,
  `Data(write:)`, `UserDefaults`). Output is stdout only; ephemeral logs are user-directed
  and gitignored (`*.log`, `out/`) with a documented cleanup step in the README.
- No raw titles are routed to any durable sink.

### 2.3 Network egress — PASS

- No `URLSession`, `URLRequest`, `NWConnection`, sockets, or `http(s)://` endpoints.
  Zero egress paths. Confirms the "local-only, no raw activity egress" guardrail.

### 2.4 Permission overreach — PASS

- Probe never *requests* a TCC grant: it uses `AXIsProcessTrusted()` (read-only trust
  check) and degrades to explicit `permission_denied` rather than prompting. Screen
  Recording is only *measured as a proxy* (name-exposure count), never required, and the
  report explicitly recommends **against** adopting Quartz/Screen Recording on the default
  path (report §3.3, §5) — the privacy-conservative choice. Net required burden for full
  capture is one grant (Accessibility); zero-grant baseline still functions.

### 2.5 Isolation — PASS

- Probe lives under `spikes/task-002-macos-capture/`, outside `src/`, `src-tauri/src/`,
  `observability/`. No build-graph reference (`Cargo.toml` / `package.json` /
  `tauri.conf.json` / `*.json` grep → none). Not a member of any shipped target.
- Diff touches **no** product-runtime, schema, or legacy manual-tracker files
  (`time_entries` / manual-entry view / stopwatch CRUD untouched, reference-only preserved).

---

## 3. APP-005 SEC-001 / SEC-007 preservation — PASS

- **SEC-001 (non-collection / allowlist):** Positive field allowlist (app name/bundle,
  AX-permitted window title, coarse time, source, capture_health, idle state) and explicit
  non-collection list (screenshots, keystrokes, screen pixels, full browser contents, full
  URLs, terminal command bodies, shell history, prompt/response text, env dumps, secrets,
  clipboard) are documented and enforced structurally by the API choice (report §7). Probe
  redaction carries the non-collection posture into reproduction evidence. Preserved.
- **SEC-007 (transparency):** Report §7.4 / §8 flag the UX "App & activity" row as
  overstating burden (NSWorkspace needs no TCC grant) and route an FB-002 reconciliation to
  BA/UX so the permissions screen reflects the real burden. Transparency obligation
  preserved and forwarded, not violated.
- **SEC-002/003 (network/credentials):** Correctly scoped N/A for capture — no network
  client, no credentials in this change.

---

## 4. Findings & escalations

- **Auto-fail conditions hit:** **none** (no secret, no CVE ≥ 7.0, no HIGH/CRITICAL image
  finding, no quality-gate ERROR).
- **Advisory (non-blocking):** none beyond items the report already routes downstream
  (FB-002 permissions-screen reconciliation to BA/UX; raw-title redaction must remain
  first-class in the TASK-005 adapter and its test fixtures — already documented in §7.4).
- **Design-level escalations to BA-flow Architect:** none. Trust boundaries and permission
  model are sound and privacy-conservative.

---

## 5. Verdict

**SEC STATUS: PASS.** No L2 auto-fail conditions hit. Secrets clean, no CVE/image surface,
no network egress, no sensitive capture or persistence, isolation intact, APP-005
SEC-001/SEC-007 allowlist + non-collection + transparency guardrails preserved. Proceed —
hold for SW-4 (Code Review) before SW-6 release.
