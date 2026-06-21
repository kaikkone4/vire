# TASK-025 — Root app env example for local MVP config (secret-safe)

## Why

Janne finished a local Mac MVP smoke through CSV export (post-TASK-024) and asked the obvious next
question: **which environment variables does the Vire app itself need, and where do I put them without
committing secrets?** Today the answer is undiscoverable from the repo:

- The repo has a tracked `observability/langfuse/.env.example` — but that is the **Docker-stack** env
  (Postgres/ClickHouse/Redis/MinIO/NextAuth secrets, `LANGFUSE_INIT_*` bootstrap, pi-observe wrapper
  defaults). It configures the *Langfuse server container*, not the *Vire desktop app*.
- The **Vire app** reads its own runtime configuration from process environment variables
  (`src-tauri/src/langfuse/config.rs`, `src-tauri/src/runtime_observer/config.rs`):
  `VIRE_LANGFUSE_BASE_URL`, `VIRE_LANGFUSE_SOURCE`, `VIRE_LANGFUSE_ENVIRONMENTS`,
  `VIRE_LANGFUSE_PUBLIC_KEY`, `VIRE_LANGFUSE_SECRET_KEY`, and optional `VIRE_RUNTIME_LOG_PATH` /
  `VIRE_RUNTIME_ENV_MAP` / `VIRE_RUNTIME_MATCH_SLOP_SECS`. There is **no root template** listing them,
  and `.gitignore` does not protect a root `.env` from being committed by accident.
- README line 69 currently says "Configure Vire's Langfuse settings to point to `http://127.0.0.1:3000`
  with your local API key/secret" but never says *how* — there is no settings UI for keys; the keys are
  env vars. A new local tester cannot connect the app to their local Langfuse without reading Rust
  source.

This is a **local-MVP onboarding / secret-hygiene gap**, not a code defect. It directly supports the
local-only posture (NFR-001) and DEC-020 (local Docker self-hosted Langfuse is the default source,
Cloud is an explicit non-default override). No FR behavior changes.

### One non-obvious architectural fact this change must encode accurately

**Vire does not auto-load a `.env` file.** There is no `dotenv`/`dotenvy` dependency in Rust and none in
Node (`src-tauri/Cargo.toml`, `package.json`); every value is read with `std::env::var`. A root `.env`
is therefore a **sourced convention**, not magic — the tester must export the variables into the shell
that launches the app (e.g. `set -a; . ./.env; set +a` before `npm run tauri:dev`). The docs in this
change MUST state this plainly; promising auto-load would be false. (Adding `dotenvy` so the app loads
a root `.env` automatically is a deliberate **out-of-scope** runtime change — see Impact.)

## What Changes

A single **docs/devops slice**. No product runtime code is touched (the env vars are already read by
existing code).

- **Add a tracked, secret-safe root `.env.example`** at the repo root listing the Vire **app** runtime
  variables with safe defaults and empty secret placeholders:
  - `VIRE_LANGFUSE_BASE_URL=http://127.0.0.1:3000` — local Docker loopback default (DEC-020). Loopback
    host required when source is `local`.
  - `VIRE_LANGFUSE_SOURCE=local` — default; `cloud` is the **explicit, non-default** override and the
    only setting that produces off-host egress. A commented `# VIRE_LANGFUSE_SOURCE=cloud` +
    `# VIRE_LANGFUSE_BASE_URL=https://cloud.langfuse.com` line shows the override without enabling it.
  - `VIRE_LANGFUSE_ENVIRONMENTS=vire` — default environment list (CSV; DEC-020 environment-first).
  - `VIRE_LANGFUSE_PUBLIC_KEY=` and `VIRE_LANGFUSE_SECRET_KEY=` — **empty placeholders only**, with a
    comment to create them in the Langfuse UI (Project settings → API keys) and never commit them.
  - Optional, commented-out by default: `# VIRE_RUNTIME_LOG_PATH=`, `# VIRE_RUNTIME_ENV_MAP=proj=env`,
    `# VIRE_RUNTIME_MATCH_SLOP_SECS=300` — all have safe code defaults, so they stay commented.
  - A header comment: copy to `.env`, fill locally, do **not** commit, and **how to apply it**
    (`set -a; . ./.env; set +a`) since the app does not auto-load it.
  - The file uses the `VIRE_LANGFUSE_*` names (not the bare `LANGFUSE_*` fallback names) to keep the
    app-env file cleanly distinct from the Docker-stack env. The bare-name fallback is mentioned in a
    comment only.
- **Update `.gitignore`** to keep a root `.env` (and variants) out of version control while keeping the
  example tracked, scoped to the repo root so it does not disturb the existing
  `observability/langfuse/` rules:
  ```
  # Local Vire app runtime env (root) — real values never committed
  /.env
  /.env.*
  !/.env.example
  ```
- **Update `README.md`** with a short "App runtime configuration (env)" subsection that: (a) lists the
  `VIRE_*` app vars and their defaults; (b) states the app reads **process env** and shows the
  `set -a; . ./.env; set +a` apply step; (c) explains the **root app `.env` vs the Docker-stack
  `observability/langfuse/.env`** distinction (one configures the desktop app, the other the Langfuse
  server); (d) reiterates local default + Cloud-as-explicit-override. The vague line 69 is tightened to
  point at the new `.env.example`.
- **Add the OpenSpec change** (this proposal, `tasks.md`, `specs/app-configuration/spec.md`).

## Impact

- **Affected files (no product runtime):** new root `/.env.example`; `/.gitignore` (3 anchored lines);
  `README.md` (one subsection + one tightened line). **No** `.rs`, `.ts`, `tauri.conf.json`,
  `Cargo.toml`, or `package.json` change. No schema, migration, network, or export change.
- **Affected specs:** new capability `app-configuration` — ADDED requirements for the secret-safe root
  app env template, the documented apply mechanism, and the app-env vs stack-env separation. No existing
  capability (`csv-export`, `langfuse-importer`, `runtime-reconciliation`) is modified.
- **Security:** strictly improves posture — adds a gitignore guard against committing a root `.env`, and
  the template ships **empty** secret fields only. Consistent with the existing rule that Langfuse keys
  and stack secrets live in local config only and are never committed/logged/exported.
- **DEC-020 / NFR-001 preserved:** local Docker Langfuse on loopback stays the default; `cloud` remains
  commented-out and explicit; no localhost binding is loosened.
- **Out of scope (clean boundaries):**
  - **Auto-loading a root `.env` via `dotenvy`** — that is a runtime behavior change (new dependency,
    reads a file from CWD at startup, a new minor attack surface) and is not needed for the onboarding
    fix. Recorded as a future option in DEC-025; if Janne wants it, it is its own backend task.
  - Any settings **UI** for entering keys (no GUI key entry exists; not introduced here).
  - The Docker-stack `observability/langfuse/.env.example` — left exactly as-is; this change does not
    merge or duplicate it.
  - Trace-emission vars for pi/Claude Code (`LANGFUSE_TRACING_ENVIRONMENT`,
    `OTEL_RESOURCE_ATTRIBUTES`) — these instrument the *agents*, not Vire; the README already covers
    them and they are referenced, not relocated.

## ADR — DEC-025 (proposed)

**Decision.** Vire's **desktop-app** runtime configuration is supplied via `VIRE_*` environment
variables, documented by a tracked, secret-safe **root `.env.example`** that is separate from the
Docker-stack `observability/langfuse/.env`. The root app `.env` is a **sourced convention** — the app
reads process env and does **not** auto-load the file. Local Docker self-hosted Langfuse on loopback
remains the default (DEC-020); Cloud stays an explicit, commented-out non-default override. Real secrets
are never committed (gitignore guard + empty placeholders).

**Status.** Proposed (this change). Routed to BA-flow Architect for the canonical decision log via
`feedback_to_ba[]`; Vire's `code/` write-scope cannot edit `artifacts/ba/07_decision_log.md`.

**Alternatives considered.** (1) Add `dotenvy` to auto-load `.env` — rejected for this slice as an
unnecessary runtime/dependency change; left as a future option. (2) Reuse the Docker-stack
`.env.example` for app config — rejected; it conflates server-container secrets with app config and
would blur the boundary this task exists to clarify.
