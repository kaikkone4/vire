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
```

Project resolution order: `--project`, `PI_OBSERVE_PROJECT`, nearest `.pi-project`/`.vire-project`, `~/.config/pi-observe/projects.json`, then cwd basename (low confidence).

## Time model

Each billable observed run increments a per-project active counter. Idle countdown starts only when the counter returns to zero. Concurrent projects are tracked independently. Editor launch wrappers are context signals; for GUI tools prefer manual active/inactive markers unless a real task/agent process is wrapped.

## Privacy defaults

Captured: project key, tool/role, timing, exit status, cwd basename, safe git branch, git remote hash, command label, optional sanitized summary.

Not captured by default: raw prompts, full command args, terminal streams, file contents, diffs, environment variables, screenshots, browser history, or hidden editor/CLI logs.

Disable wrapper telemetry while still running commands:

```sh
PI_OBSERVE_ENABLED=false pi-observe run -- echo hello
```

## Integration examples

- Shell aliases/functions: `observability/pi-observe/examples/shell/aliases.sh`
- VS Code tasks: `observability/pi-observe/examples/vscode/tasks.json`
- Cursor/VS Code/Antigravity: use observed launchers for context or manual markers for billable active work. Do not scrape private app storage/logs without explicit review.
