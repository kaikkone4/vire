# pi-observe

`pi-observe` is a metadata-only wrapper for local development/delegate commands. It records local time-state events and best-effort Langfuse traces. Telemetry failures fail open: the wrapped command still runs.

## Usage

```sh
pi-observe run --tool pi-team --role delegate-developer --project vire -- delegate-developer ...
pi-observe run --tool claude-code --project vire -- claude
pi-observe run --tool copilot-cli --role suggest --project vire -- gh copilot suggest "..."
pi-observe mark-active --tool cursor --project vire --summary "Cursor coding session"
pi-observe mark-inactive --tool cursor --project vire
pi-observe status
pi-observe reconcile
```

Project resolution order: `--project`, `PI_OBSERVE_PROJECT`, nearest `.pi-project`/`.vire-project`, `~/.config/pi-observe/projects.json`, then cwd basename (low confidence).

## Time model

Each billable observed run increments a per-project active counter. Idle countdown starts only when the counter returns to zero. A new billable run cancels/resets any existing idle countdown for that project. Concurrent projects are tracked independently. Editor launch wrappers are nonbillable context signals by default; for GUI tools prefer manual active/inactive markers or observed tasks for billable work.

Idle completion and orphan reconciliation are explicit maintenance operations in phase 1. `pi-observe status` runs reconciliation before printing state, and `pi-observe reconcile` can be called from a cron/LaunchAgent or smoke/setup flow. Reconciliation appends `idle_started` after `PI_OBSERVE_IDLE_THRESHOLD_MS` elapses and marks active runs older than `PI_OBSERVE_ORPHAN_TIMEOUT_MS` as `tool_orphaned`, then starts idle countdown if no active runs remain.

## Privacy defaults

Captured: project key, tool/role, timing, exit status, cwd basename, safe git branch, git remote hash, command label, optional sanitized summary.

Not captured by default: raw prompts, full command args, terminal streams, file contents, diffs, environment variables, screenshots, browser history, or hidden editor/CLI logs.

`pi-observe` loads only the needed Langfuse API variables from `observability/langfuse/.env` using a data-only parser; do not source the full `.env`. Wrapped child commands receive a scrubbed environment with Langfuse/backing-service secrets removed. Langfuse ingestion is restricted to loopback hosts (`localhost`, `127.0.0.1`, `::1`) by default; set `PI_OBSERVE_ALLOW_REMOTE_LANGFUSE=true` only if you intentionally point telemetry at a remote Langfuse endpoint. Session IDs supplied via `--session` or `PI_OBSERVE_SESSION` are hashed before storage/transmission.

Disable wrapper telemetry while still running commands:

```sh
PI_OBSERVE_ENABLED=false pi-observe run -- echo hello
```

## Integration examples

- Shell aliases/functions: `observability/pi-observe/examples/shell/aliases.sh`
- VS Code tasks: `observability/pi-observe/examples/vscode/tasks.json`
- Cursor/VS Code/Antigravity: use observed launchers for context or manual markers for billable active work. Do not scrape private app storage/logs without explicit review.
