# Architecture Review — TASK-025 (root app env example for local MVP config)

**Reviewer:** SW Architect (SW-1)
**Change:** `openspec/changes/task-025-root-app-env-example/`
**Tier:** L2
**Verdict:** **PASS** — one docs/devops slice. No split required. One design decision recorded
(DEC-025) and one BA-flow feedback item raised (decision-log entry only).

---

## 1. Scope vs. architecture plan

The task is an onboarding / secret-hygiene gap, not new behavior. It aligns with:

- **DEC-020** (`artifacts/ba/03_architecture_plan.md` / `07_decision_log.md`): local Docker self-hosted
  Langfuse on loopback is the canonical default; Cloud is an explicit non-default override. The template
  ships `VIRE_LANGFUSE_BASE_URL=http://127.0.0.1:3000` + `VIRE_LANGFUSE_SOURCE=local` as the active
  default and keeps `cloud` commented out. **Preserved.**
- **NFR-001 / privacy posture**: local-only; the change adds a gitignore guard and empty-placeholder
  secret fields — it strictly *improves* secret hygiene and introduces no network or egress.
- The architecture plan's stated assumption (line 147) that "Langfuse API credentials and local stack
  secrets are provided by local configuration/secret storage and must never be printed, exported,
  committed…". This change operationalizes that assumption for the *app* tier, which was previously only
  covered for the *Docker-stack* tier.

No component boundary is crossed. The only files touched are `/.env.example`, `/.gitignore`, `README.md`,
and the OpenSpec change. **No product runtime code** is modified.

## 2. Contract verified against the code (authoritative source)

The env-var contract was derived from the code that actually reads it, not from prose:

| Variable | Read at | Default in code | In template |
|---|---|---|---|
| `VIRE_LANGFUSE_BASE_URL` | `langfuse/config.rs:139` | `http://127.0.0.1:3000` | active default |
| `VIRE_LANGFUSE_SOURCE` | `langfuse/config.rs:140` | `local` | active default |
| `VIRE_LANGFUSE_ENVIRONMENTS` | `langfuse/config.rs:277` | `vire` | active default |
| `VIRE_LANGFUSE_PUBLIC_KEY` | `langfuse/config.rs:151` (fallback `LANGFUSE_PUBLIC_KEY`) | — | empty placeholder |
| `VIRE_LANGFUSE_SECRET_KEY` | `langfuse/config.rs:153` (fallback `LANGFUSE_SECRET_KEY`) | — | empty placeholder |
| `VIRE_RUNTIME_LOG_PATH` | `runtime_observer/config.rs:67` | pi-observe path | commented optional |
| `VIRE_RUNTIME_ENV_MAP` | `runtime_observer/config.rs:37` | empty map | commented optional |
| `VIRE_RUNTIME_MATCH_SLOP_SECS` | `runtime_observer/config.rs:39` | `300` | commented optional |

These names and defaults must match the code verbatim; the implementer should re-grep before writing to
catch any drift.

## 3. Key architectural finding (must be honored in docs)

**Vire does not auto-load a `.env` file.** No `dotenv`/`dotenvy` in `src-tauri/Cargo.toml`; no dotenv in
`package.json`. All config is `std::env::var`. Therefore the root `.env` is a **sourced convention** —
the docs MUST show `set -a; . ./.env; set +a` (or equivalent export) before `npm run tauri:dev`, and
MUST NOT claim auto-load. This is the single most important correctness constraint on the docs slice; a
template that *looks* like it auto-applies but doesn't would send testers in circles. Captured as a spec
requirement ("Documentation states the app reads process env and how to apply the file") so QA can check
it.

## 4. Boundary / scope constraints for the implementer

- **Do not add `dotenvy`** or any runtime auto-load in this slice. That is a backend runtime change with
  a new dependency and a new CWD-file read surface; it is recorded as a future option under DEC-025 and,
  if wanted, is its own task. Keeping it out keeps this an L2 docs/devops slice.
- **Use `VIRE_*` names** in the root template, not the bare `LANGFUSE_*` fallback names — that keeps the
  app-env file cleanly separate from the Docker-stack env file (the whole point of the task). Mention the
  fallback in a comment only.
- **Anchor the gitignore rules to root** (`/.env`, `/.env.*`, `!/.env.example`) so the existing
  `observability/langfuse/.env*` rules (lines 8–11) are not disturbed. Verify with `git check-ignore -v`.
- **No secrets in the template** — every credential field empty/placeholder. No real keys, even local
  ones.
- **No edits outside `code/`.** The DEC-025 entry for the canonical BA decision log
  (`artifacts/ba/07_decision_log.md`) is **out of write-scope** for SW roles — raised as feedback to BA
  below; do not attempt to edit it.

## 5. One slice vs. split

**One slice.** All edits are root-level config/docs with a single coherent purpose and no component
boundary crossing. Splitting (e.g. template vs. README vs. gitignore) would create three trivially
coupled PRs for one change — churn, not separation. **No split required.**

## 6. ADR — DEC-025 (proposed)

See `proposal.md` § ADR. Summary: app config via `VIRE_*` env vars, documented by a secret-safe root
`.env.example` separate from the Docker-stack env; root `.env` is a sourced convention (no auto-load);
local Langfuse default preserved; secrets never committed. Status: proposed; canonical log entry routed
to BA.

## 7. feedback_to_ba[]

```
feedback_to_ba:
  - id: FB-025-1
    type: decision-log-entry
    target: artifacts/ba/07_decision_log.md
    summary: >
      Record DEC-025 — Vire desktop-app runtime configuration is supplied via VIRE_* environment
      variables, documented by a tracked secret-safe root .env.example that is separate from the
      Docker-stack observability/langfuse/.env. The root app .env is a sourced convention (the app
      reads process env and does not auto-load .env). Local Docker Langfuse on loopback remains the
      default (extends DEC-020); Cloud stays an explicit, commented-out override. Auto-loading via
      dotenvy is a deliberately deferred future option.
    reason: >
      SW write-scope is limited to code/; ADRs must land in the canonical BA decision log. DEC-025 is
      proposed in the change proposal and arch-review but needs a BA-flow entry to be authoritative.
    blocking: false
```

## Verdict

**PASS.** Proceed with the single docs/devops slice exactly as scoped in `proposal.md` / `tasks.md`,
honoring the no-auto-load documentation constraint and the root-anchored gitignore rules. Route DEC-025
to BA for the canonical decision log (non-blocking).
