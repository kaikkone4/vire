# spikes/task-007-langfuse-importer

**Status:** non-shipping, reference-only spike probe for TASK-007 (Langfuse importer
validation — Phase A). **Not** a member of any shipped build target. Delete this whole
tree once the TASK-007 MVP has consumed the findings.

## What this is

A single-file read-only probe (`probe/langfuse-probe.mjs`) that proves the
schema/pagination/dedup/cursor/health findings in
[`../../openspec/changes/task-007-langfuse-importer-validation/langfuse-validation-report.md`](../../openspec/changes/task-007-langfuse-importer-validation/langfuse-validation-report.md).
It is supporting evidence for that report, not a product. It has two modes:

- **`--mock` (offline).** Proves the import-flow logic — pagination, dedup by
  `(environment, trace_id)`, per-environment cursor, and the 9-state health model
  (incl. `absence ≠ zero`) — against **synthetic, non-sensitive in-memory fixtures**. No
  network, no credentials, no container. This is the CI/headless-safe proof.
- **live (default).** Validates against the **configured Langfuse API** (cloud-first per
  DEC-018, e.g. `https://cloud.langfuse.com`, or an optional local stack). Records the
  observed trace/observation schema **shape-only** and exercises pagination/cursor over a
  real time window. Requires a configured base URL + project-scoped keys in local secure
  config; if absent it prints redacted secure-config instructions and exits `needs_input`.

Read-API surface exercised (live):

1. `GET /api/public/health` — liveness (`auth/config` vs `rate_limit` distinction).
2. `GET /api/public/traces?environment=…&fromTimestamp=…&toTimestamp=…&page=…&limit=…`
   — environment-scoped, time-windowed listing; **pagination** to window completion;
   **dedup** by trace id scoped to environment; per-environment **cursor** = max
   observed timestamp.
3. `GET /api/public/traces/{id}` — trace detail incl. `observations`.
4. observation/`usage` shape — where token usage and cost actually live (pi-observe
   traces carry timing metadata only; usage/cost come from generation observations).

## Isolation guarantees

- Lives outside `src/`, `src-tauri/src/`, and `observability/`.
- Not referenced by `Cargo.toml`, `tauri.conf.json`, `package.json`, or any build graph.
- Does **not** import, modify, reuse, or re-implement `observability/pi-observe` or the
  local Langfuse stack — those stay reference-only validation inputs (DEC-017).
- Does **not** import, migrate, reuse, or modify the legacy manual-tracker surface.

## Privacy / security (SEC-002 / SEC-003) — read before running

- **Shape-only output.** The probe prints field **names, value types, nullability, and
  counts** only. It never prints a trace value, prompt, response, command body, real
  usage/cost number, session id, secret, or environment dump. Strings are reduced to a
  length bucket; the underlying string is never emitted.
- **Configured-endpoint-only (SEC-002).** The only network path is the configured Langfuse
  base URL. Every request is built as **base + path**; absolute URLs from response data are
  **never** followed. A `LANGFUSE_HOST` that is not a valid `http(s)` origin is rejected. The
  probe is read-only (GET) — it pulls traces, never pushes macOS activity.
- **Credentials (SEC-003).** Reads `LANGFUSE_HOST`/`LANGFUSE_PUBLIC_KEY`/`LANGFUSE_SECRET_KEY`
  from `observability/langfuse/.env` via a data-only parser (no shell sourcing), or from the
  environment, and uses them only for the `Authorization` header. Keys are never printed,
  logged, or persisted.
- **Writes nothing on its own.** For an ephemeral local record, redirect stdout to a
  `*.local.log` in this directory (gitignored) and **delete it when done**.

## How to run

```sh
# Offline logic proof — no network, no credentials, no container (CI/headless safe):
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --mock

# Live shape probe against the configured Langfuse API (keys from local secure config):
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment vire
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment default --hours 720

# Optional ephemeral record (gitignored), then delete:
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment default > out.local.log
rm -f out.local.log
```

If the configured base URL / project keys are absent, the live mode prints secure
local-config instructions using **redacted placeholders** and exits `needs_input` (exit 2)
— it never asks for secrets in-band and never probes or prints a secret.

### Optional local Docker fixture (NOT required per DEC-018)

```sh
./scripts/setup-local-observability.sh   # interactive: checks Docker/Compose, creates .env
./scripts/langfuse-up.sh                  # brings up the pinned stack on 127.0.0.1:3000
# create project API keys in the Langfuse UI, paste only into observability/langfuse/.env
./scripts/langfuse-smoke-test.sh          # emits non-sensitive vire + default traces
./scripts/langfuse-down.sh
```

## Parse-only check (CI / headless safe)

```sh
node --check spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs
```

Validates the probe parses without contacting any endpoint or reading any secret. The
`--mock` proof is also headless-safe; the live shape probe is the only step that needs a
configured base URL and project keys (see the report's validation-status matrix).
