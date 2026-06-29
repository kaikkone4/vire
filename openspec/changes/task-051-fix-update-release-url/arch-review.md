# Architecture Review — TASK-051 (hotfix: update-check repo owner)

**Verdict:** PASS — tiny, single-component, in-scope. No split, no BA escalation.
**Tier:** L2. **Reviewed against:** TASK-050 spec + code-to-spec, `03_architecture_plan.md` (SEC-002).

## Scope assessment

- **One component** (the Vire desktop app: Rust core + one capability file + tests). No component
  boundary is crossed → **no split required** (Working rule: split only if scope crosses a
  `03_architecture_plan.md` boundary).
- **Correction, not redesign.** The behaviour was already specified correctly in TASK-050
  (`specs/in-app-update-check/spec.md:9,45,53` → `kaikkone4/vire`); the merged code diverged. This
  change re-aligns the implementation to the existing spec and adds an invariant guard. No new
  design decision (no ADR / `DEC-###`) is warranted.

## Architectural consistency

| Check | Result | Notes |
|---|---|---|
| Matches existing spec owner | PASS | Spec already mandates `kaikkone4/vire`; this fixes code to spec. |
| Repository identity | PASS | `remote.origin.url = https://github.com/kaikkone4/vire.git`; merge `a0c8cff` is PR #38 from `kaikkone4/…`. |
| Component boundary | PASS | Desktop-app-only; no schema/migration/IPC-storage/capture/classifier/Langfuse touch. |
| Generated artifacts | PASS | `gen/schemas/capabilities.json` + `target/**` are gitignored; regenerate from `capabilities/default.json` at build — must not be hand-edited or committed. |
| Version contract | PASS | No bump. `CARGO_PKG_VERSION = 0.8.0` (TASK-049) is the single running-version source and is independent of the target URL string. |

## NFR / security implications (SEC-002)

- **No network-boundary change.** The egress host set is unchanged — one read-only GitHub host
  (`api.github.com`) plus the user-initiated browser open of `github.com`. Only the owner/name path
  segment is corrected. Client posture (HTTPS-only, GET-only, `redirect::Policy::none()`, bounded
  timeout, no credentials, no app data) is untouched. The renderer stays loopback-only; **CSP
  unchanged**. SEC-002 amendment from TASK-050 still holds as written.
- **Capability scope unchanged in kind.** Still the host/path-scoped `opener:allow-open-url`; only
  the allowed URL's owner segment is corrected. No `opener:default` / `allow-default-urls`.
  Architectural constraint added: the allowlist URL MUST equal the compile-time `RELEASES_URL`
  constant (enforced by the regression guard) so capability and code cannot drift apart again.
- **Fail-soft posture preserved.** No change to error mapping; rate-limit/403/429 still map to
  `Unknown` with no detail leak.

## Constraints handed to the developer

1. Correct exactly the three targets + fixtures named in `tasks.md`; change nothing else.
2. Add the owner-typo regression guard (the one new, testable invariant) — static assertion or unit
   test asserting `kaikkone4/vire` present and `kaikkonen4` absent, and allowlist URL == `RELEASES_URL`.
3. Do not bump the version. Do not introduce updater/download/install. Do not edit generated files.
4. Re-run the TASK-050 check suite + the new guard, then repeat the browser/API smoke before release.

## Escalation / feedback to BA

None. No boundary or decision diverges from the BA architecture plan; the spec was already correct.
No `feedback_to_ba[]` entry required.
