# spikes/task-007-langfuse-importer

**Status:** non-shipping, reference-only spike probe for TASK-007 (Langfuse importer
validation — Phase A). **Not** a member of any shipped build target. Delete this whole
tree once the TASK-007 MVP has consumed the findings.

## What this is

A single-file read-only probe (`probe/langfuse-probe.mjs`) that exercises the Langfuse
**public read API** served by the pinned local stack (`langfuse/langfuse:3.63.0`,
loopback `http://localhost:3000`), so the schema/pagination/dedup/cursor findings in
[`../../openspec/changes/task-007-langfuse-importer-validation/langfuse-validation-report.md`](../../openspec/changes/task-007-langfuse-importer-validation/langfuse-validation-report.md)
are reproducible by hand once the stack is up. It is supporting evidence for that
report, not a product.

Read-API surface exercised:

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
- **Loopback-only (SEC-002).** Refuses any non-loopback `LANGFUSE_HOST`. The only
  network path is the configured local Langfuse read API. No macOS activity egress.
- **Credentials (SEC-003).** Reads `LANGFUSE_PUBLIC_KEY`/`LANGFUSE_SECRET_KEY` from
  `observability/langfuse/.env` via a data-only parser (no shell sourcing) and uses them
  only for the `Authorization` header. Keys are never printed, logged, or persisted.
- **Writes nothing on its own.** For an ephemeral local record, redirect stdout to a
  `*.local.log` in this directory (gitignored) and **delete it when done**.

## How to run (only when the local stack is up)

```sh
./scripts/setup-local-observability.sh   # interactive: checks Docker/Compose, creates .env
./scripts/langfuse-up.sh                  # brings up the pinned stack on 127.0.0.1:3000
# create project API keys in the Langfuse UI, paste only into observability/langfuse/.env
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment vire
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment default
# optional ephemeral record:
node spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs --environment vire > out.local.log
# inspect, then:
rm -f out.local.log
./scripts/langfuse-down.sh
```

## Parse-only check (CI / headless safe)

```sh
node --check spikes/task-007-langfuse-importer/probe/langfuse-probe.mjs
```

Validates the probe parses without contacting any stack or reading any secret. Running
the probe against live traces is a **manual** step requiring the running local stack and
locally-configured project keys (see the report's validation-status matrix).
