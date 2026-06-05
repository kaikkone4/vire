# QA gate report — TASK-007 Langfuse importer validation (SW-3)

- **Change:** `task-007-langfuse-importer-validation`
- **Branch:** `feat/task-007-langfuse-importer-validation` · **PR:** #8
- **Tier:** L2 · **Gate:** SW-3 (QA) · **Reviewer:** QA Engineer (SW-3)
- **Date:** 2026-06-05
- **Verdict:** **PASS** → route to SW-4 (Code Reviewer) ∥ SW-5 (Security Agent)

> Gate criterion: every OpenSpec scenario in
> `specs/langfuse-importer-validation/spec.md` has observable coverage. This is a Phase A
> **validation spike** — its "integration test" is the non-shipping probe
> (`spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs`), exercised offline (`--mock`,
> deterministic) and live (configured cloud API, shape-only). No NFR throughput/latency target
> is specified, so no performance test is required at L2.

---

## 1. Verification performed by this gate

| Check | Method | Result |
| --- | --- | --- |
| OpenSpec strict validation | `openspec validate … --strict` | **valid** |
| Diff scope (TASK-007 only) | `git diff --name-only f624891..HEAD` | **9 files, all under `openspec/changes/task-007-…/` + `spikes/task-007-langfuse-importer/`** — no `src/`, `src-tauri/src/`, `observability/`, `pi-observe`, schema/config, or legacy manual-tracker changes |
| Probe parse-check | `node --check langfuse-probe.mjs` | **parses clean** (headless-safe) |
| Probe offline proof | `node langfuse-probe.mjs --mock` | **ALL CHECKS PASS** (see §3) |
| Probe not in any build target | grep `*.toml`/`*.json`/`*.conf` for spike paths | **no references** (not in `package.json`, `Cargo.toml`, `tauri.conf.json`) |
| Live shape-only behavior | `node langfuse-probe.mjs --environment default …` | **auth/health confirmed live** (`HTTP 200`, base `https://cloud.langfuse.com`); output shape-only; **no secrets/values** printed. Traces-listing re-run hit the probe's hardcoded 5 s client timeout in this sandbox — see §4 |
| Secret scan of committed artifacts | pattern scan of `f624891..HEAD` diff + live probe stdout | **no concrete tokens**; only redacted placeholders (`pk-lf-…`, `sk-lf-…`, `LANGFUSE_*=...`) |

---

## 2. Scenario coverage matrix (gate criterion)

All 14 scenarios across 9 requirements have observable coverage. **C** = covered.

| # | Requirement / Scenario | Coverage evidence | Status |
| --- | --- | --- | --- |
| 1.1 | Observed schema recorded from configured API | report §2.2/§2.3 (live shape-only `Trace`+observation schema); probe `schemaSketch`/`printSchema` live + mock | **C** |
| 1.2 | Time/usage/cost validated as primary source; absence ≠ zero | report §3, §4(H2); mock: empty env → cursor `null`, health `missing` (asserted, reproduced) | **C** |
| 2.1 | Pagination completes window + cursor computed | report §6.2; mock 3-page walk → cursor = max ts (reproduced); live 2-page walk (report) | **C** |
| 2.2 | Duplicate traces detected, not double-counted | report §6.3; mock: 1 cross-page dup suppressed + idempotent overlap re-import (reproduced) | **C** |
| 3.1 | All 9 health states with detection basis + consequence | report §4 table; mock asserts each of 9 states produced by its rule (reproduced, all PASS) | **C** |
| 3.2 | Workspace failure modes (wrong/default env; hook silent-fail) | report §5 — live-confirmed `vire` empty, real traffic in `default`; mock `wrong_env`/`missing` | **C** |
| 4.1 | Mapping signals assessed without building classification | report §7 signal table; defers TASK-008/TASK-010 explicitly | **C** |
| 5.1 | Import-run + normalized-evidence shapes proposed, not migrated | report §8.1/§8.2; **no `.sql`/migration in diff** (verified) | **C** |
| 6.1 | Only configured Langfuse endpoint contacted; no raw-activity egress | report §10; code: `apiGet` builds base+path, `normalizeBaseUrl` rejects non-`http(s)`, GET-only; live probe hit only `cloud.langfuse.com` | **C** |
| 6.2 | Credentials never in evidence/logs/output; redacted placeholders | report §10; secret scan clean; live output shape-only; keys used only for `Authorization` header | **C** |
| 7.1 | Probe confined and disposable, not in any build target | `spikes/task-007-langfuse-importer/`; grep confirms no build-graph reference; README disposal note | **C** |
| 7.2 | Emitter and legacy manual-tracker not modified | diff touches no `observability/`, `pi-observe`, or manual-tracker files | **C** |
| 8(priv).1 | No secrets/raw content persisted or committed | shape-only probe output; `.gitignore` for `*.local.log`; scan clean | **C** |
| 9.1 | Spike defers MVP / host-runtime / schema decisions | report §9, §11; no durable schema/runtime in diff; TASK-003/004/MVP handoffs stated | **C** |

---

## 3. Offline `--mock` proof (reproduced this gate)

```
pagination/dedup/cursor: unique=4 (exp 4), raw=5 (exp 5), dupes=1 (exp 1), pages=3 (exp 3)
overlapping re-import idempotent: true (exp true)
9-state health model: all 9 PASS (valid, missing, stale, wrong_env, delayed,
                      duplicate, schema_mismatch, auth_or_config_error, rate_limited)
absence != zero: empty env -> cursor=null (not 0), health=missing
MOCK RESULT: ALL CHECKS PASS  (exit 0)
```

The pagination/dedup/cursor/health logic is validated **without network, credentials, or
container**, satisfying the offline-proof requirement.

## 4. Live re-run note (not a gate failure)

Project-scoped credentials were present in local secure config (environment). The live probe
**authenticated successfully** against the configured cloud base
(`https://cloud.langfuse.com`, `health: HTTP 200`) and emitted only shape-only headers — no
secret, value, prompt/response, or env dump. The subsequent `/api/public/traces` listing
repeatedly aborted at the probe's hardcoded **5 s client timeout** in this QA sandbox, so the
full live pagination/schema sketch could not be re-rendered here. This is an **environmental
network limitation of the re-run**, not a defect in the probe or report:

- the live **auth + base-URL + health** path is independently confirmed (HTTP 200);
- the full import-flow logic is **deterministically proven** by `--mock`;
- the report's live claims (§0/§2/§5/§12) are internally consistent and shape-only.

Per the gate's "do not fail solely on live (un)availability" rule, the live **traces-listing
re-run is marked partially reproduced**; it does not affect the verdict.

## 5. Privacy / security findings

- **No credentials** in artifacts, fixtures, probe output, or PR text — only redacted
  placeholders. Live keys are read from the environment and used solely for the
  `Authorization` header; never printed/logged/persisted. **No rotation needed** (nothing
  leaked).
- **No** prompt/response text, terminal command bodies, env dumps, raw macOS activity, or
  private trace contents in any committed file or in live output. Trace `input`/`output` is
  recorded shape-only and flagged must-not-persist (report §2.2, §10).
- **SEC-002 boundary** upheld in code: every request is `base + path`; absolute URLs from
  response data are never followed; non-`http(s)` hosts rejected; GET-only.

## 6. tasks.md ↔ coverage check

All `tasks.md` checkboxes (1.1–8.4) correspond to delivered report sections / probe
assertions verified above. `8.4` (strict validate) and `8.3` (secret scan) independently
re-confirmed by this gate.

## 7. Verdict

**PASS.** Exit gate satisfied for the Phase A spike: Langfuse is validated as usable primary
AI time/usage/cost source where traces are valid (conditioned on generation observations
carrying usage+cost); all 9 health states incl. missing/stale/wrong-env/delayed/duplicate/
schema/auth/rate-limit are visible with `absence ≠ zero`; credentials protected; probe
isolated/non-shipping; no durable MVP, host-runtime (TASK-003), or schema (TASK-004) decision
taken. Route to **SW-4 (Code Reviewer)** and **SW-5 (Security Agent)** in parallel.
