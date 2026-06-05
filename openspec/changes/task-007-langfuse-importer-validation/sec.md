# SW-5 Security Review Gate — TASK-007 (Langfuse importer validation)

- **Change:** `task-007-langfuse-importer-validation`
- **Branch:** `feat/task-007-langfuse-importer-validation`
- **PR:** https://github.com/kaikkone4/vire/pull/8
- **Tier:** L2 (secrets + CVE≥7 + Trivy + Sonar quality gate)
- **Verdict:** **PASS**
- **Reviewer:** Security Agent (SW-5), 2026-06-05

## Scope reviewed

Phase-A spike. Diff `main...HEAD` adds **only** OpenSpec docs and a single
non-shipping probe: `spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs`
(+ README, `.gitignore`). No product code touched under `src/`, `src-tauri/src/`,
or `observability/`. Probe is **not** referenced by `package.json`, `Cargo.toml`,
or `tauri.conf.json` (verified). Zero third-party dependencies — `node:` builtins only.

## Tier-1 scanner stack

| Scanner | Method | Result |
|---|---|---|
| **gitleaks** (secrets) | Tool not installed; substituted manual `main...HEAD` pattern scan (`ghp_/gho_/ghu_/ghr_/github_pat_/sk-ant-/sk-proj-/sk-lf-/pk-lf-/AKIA…/xox*/BEGIN…PRIVATE KEY`) + semgrep `p/secrets` ruleset | **PASS** — no live secret. All `pk-lf-…/sk-lf-…/LANGFUSE_*=...` hits are redacted placeholders in prose/instructions; secret-prefix matches in docs are pattern *labels*, not values. |
| **OSV-scanner** (CVE≥7) | Tool not installed; dependency surface inspected directly | **PASS (N/A surface)** — spike has no `package.json`/lockfile and imports only Node builtins; no third-party dependency to score. |
| **Trivy** (container HIGH/CRIT) | Tool not installed; image surface inspected | **N/A** — change builds/modifies no container image. The pinned `langfuse/langfuse:3.63.0` stack is reference-only and **unchanged** by this diff. |
| **SonarQube CE** (quality gate) | Tool not installed; substituted semgrep `p/javascript` (104 rules) + `node --check` | **PASS (proxy)** — 0 findings, parses clean. |

> Tooling note: the Tier-1 binaries are not provisioned in this environment. Manual
> pattern scans + semgrep (`p/javascript`, `p/secrets`) + dependency/image inspection
> were used as equivalent substitutes. Recommend wiring the real Tier-1 stack into CI
> before the TASK-007 **MVP** importer (which ships product code and a real network
> client) reaches SW-5.

## Manual secure-code review

Probe is read-only (GET), shape-only output, single configured endpoint.

- **Credential handling (SEC-003): PASS.** Keys read from `observability/langfuse/.env`
  via a **data-only** parser (no shell sourcing; rejects values with `` ` ``/`$`/`;`)
  or `process.env`, restricted to a 3-key allowlist. Used only to build the `Basic`
  `Authorization` header. Never printed, logged, or persisted; `safeHostLabel()` emits
  `protocol//host` only. `printSecureConfigInstructions()` uses redacted placeholders and
  exits `needs_input` (2) — never solicits secrets in-band.
- **Raw trace / private-data leakage: PASS.** `shapeOf()`/`schemaSketch()` emit field
  names, value *types*, nullability, and counts only; strings collapse to a length bucket;
  numbers render as `int`/`float` (never the value). No prompt/response/command-body
  value is ever read or printed.
- **Prompt/response/command/env dump: PASS.** No env enumeration; only the 3 allowlisted
  vars are accessed. Error handler prints `e.name` only (no stack/URL/message) — avoids
  incidental URL/credential leakage on failure.
- **Network boundary / SSRF / open-redirect / base URL (SEC-002): PASS.**
  `normalizeBaseUrl()` requires a syntactically valid `http(s)` origin and reconstructs
  `protocol//host`; every request is `base + path`. Pagination advances by **page number**,
  never by a `next`-URL from response data — absolute URLs from data are never followed.
  Query params (`environment`, trace `id`) pass through `encodeURIComponent`.
- **Rate-limit / auth handling: PASS.** `429 → rate_limited` (pagination halts, no hammering);
  `401/403 → auth_or_config_error`; 5s `AbortController` timeout per request. Maps cleanly to
  the 9-state health model; `absence ≠ zero` preserved (empty env → `cursor=null` + `missing`).
- **DEC-017 preservation: PASS.** Probe does not import, modify, reuse, or re-implement
  `observability/pi-observe`, the local Langfuse stack, or the legacy manual tracker; builds
  no new pi/Claude adapter. No durable product schema committed.
- **APP-005 SEC-002/SEC-003: preserved** (above). **SEC-004** (human-approval invariant):
  correctly **N/A** — the spike produces no billable/profitability totals; recorded as a
  downstream MVP note in `arch-review.md`.
- **No real data committed: PASS.** Only synthetic fixtures (`session-aaaa…eeee`, zeroed
  usage/cost). No real session id, cost, or trace value in any committed artifact.

## Advisory findings (non-blocking, for the MVP importer)

- **A1 — live cursor timestamp printed verbatim.** `runLive()` prints `res.cursor` (max
  observed trace timestamp) and aggregate counts. These are low-sensitivity metadata, not
  trace content, and match the documented cursor design — acceptable for the spike. For the
  MVP, confirm timestamps/counts are acceptable in any persisted import-run log.
- **A2 — redirect handling.** `fetch` follows redirects by default. The WHATWG fetch spec
  strips the `Authorization` header on cross-origin redirects, so credential exposure risk is
  low against the trusted configured host. For the MVP client, consider `redirect: 'manual'`
  (or an explicit same-origin assertion) for defense-in-depth.

## Auto-fail check (L2)

No auto-fail condition hit: no committed secret; no dependency CVE≥7 (no deps);
no container HIGH/CRITICAL (no image change); no Sonar/SAST ERROR (semgrep 0 findings).

## Gate verdict

**PASS.** Proceed; hold release until SW-4 (code review) also passes before SW-6.
No SEC-### blocker. No design-level escalation (`feedback_to_ba[]` empty).
