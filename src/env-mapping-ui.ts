// Pure HTML builders for the Settings environment picker (C4) and environment→project mapping (D4).
// No DOM or IPC here so the markup logic stays unit-testable; the click handlers that actually invoke
// create_project / set_env_mapping / clear_env_mapping live in main.ts. Every surface carries only
// environment names and project references — never a credential, trace payload, or secret (SEC-010).

import { escapeHtml as esc } from './html';
import { parseEnvironmentsCsv } from './langfuse-settings';

// Mirrors the backend `DiscoveredEnvState` (env_mapping/mod.rs) — Tauri serializes snake_case.
export type DiscoveredEnvState = {
  environment: string;
  last_seen: string;
  mapped: boolean;
  project_id: string | null;
  project_name: string | null;
};

export type PickerProject = { id: string; name: string; archived: boolean };

// The built-in default environment. Always offered in the picker so the v0.1 default stays one click
// away even before discovery has run (design §4, default `vire` unchanged).
export const DEFAULT_ENVIRONMENT = 'vire';

// The set of environments the picker offers: the built-in default, every discovered environment, and
// any already-configured environment (so a hand-entered env that discovery hasn't surfaced still shows
// as a ticked box). De-duplicated, trimmed, and sorted for a deterministic render.
export function envPickerOptions(discovered: string[], selected: string[]): string[] {
  const set = new Set<string>();
  for (const v of [DEFAULT_ENVIRONMENT, ...discovered, ...selected]) {
    const t = v.trim();
    if (t) set.add(t);
  }
  return [...set].sort((a, b) => a.localeCompare(b));
}

// Checkbox group seeding `allowed_environments`. A box is ticked when its environment is currently
// configured; unticking and saving removes it. The CSV field stays empty as an advanced fallback for
// environments discovery hasn't surfaced — see `mergeSelectedEnvironments`.
export function envPickerCheckboxes(discovered: string[], selected: string[]): string {
  const selectedSet = new Set(selected.map((s) => s.trim()).filter(Boolean));
  const boxes = envPickerOptions(discovered, selected)
    .map((env) => {
      const checked = selectedSet.has(env) ? ' checked' : '';
      return `<label class="env-pick"><input type="checkbox" name="env_pick" value="${esc(env)}"${checked}><span>${esc(env)}</span></label>`;
    })
    .join('');
  return `<div class="env-picks">${boxes}</div>`;
}

// On save, union the ticked picker boxes with any advanced CSV entries (so a user can still hand-add
// an environment discovery hasn't surfaced yet). Trimmed and de-duplicated, order-preserving. An empty
// result is left empty — the Rust core applies the `vire` code default when no environment is stored.
export function mergeSelectedEnvironments(checked: string[], csv: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const v of [...checked, ...parseEnvironmentsCsv(csv)]) {
    const t = v.trim();
    if (t && !seen.has(t)) {
      seen.add(t);
      out.push(t);
    }
  }
  return out;
}

function projectOptionsHtml(projects: PickerProject[]): string {
  return projects
    .map((p) => `<option value="${esc(p.id)}">${esc(p.name)}${p.archived ? ' (archived)' : ''}</option>`)
    .join('');
}

// One mapping row per discovered environment (D4). Mapped → show the project plus a Clear action.
// Unmapped → a project picker that maps to an existing project AND an in-app "Create & map" affordance.
// The create affordance is an inline text input (pre-filled with the environment name) plus a button —
// NOT a native window.prompt dialog, which silently returns null in the macOS WKWebView (TASK-030). The
// create action is explicit only — the handler calls create_project then set_env_mapping; nothing is
// auto-created or auto-mapped here (DEC-006). This builder stays pure HTML (no DOM/IPC) so the markup is
// unit-testable; the click wiring lives in main.ts bindEnvMapping().
export function mappingRow(env: DiscoveredEnvState, projects: PickerProject[]): string {
  const e = esc(env.environment);
  if (env.mapped && env.project_id) {
    return `<tr data-env-row="${e}"><td><b>${e}</b></td><td>Mapped → <b>${esc(env.project_name ?? env.project_id)}</b></td><td><button class="danger" data-clear-map="${e}">Clear mapping</button></td></tr>`;
  }
  const picker = projects.length
    ? `<select data-map-select="${e}"><option value="">Choose a project…</option>${projectOptionsHtml(projects)}</select> <button data-map-set="${e}">Map</button>`
    : '<span class="empty">No projects yet — create one below.</span>';
  const create = `<input type="text" class="env-create-name" data-create-name="${e}" value="${e}" maxlength="120" placeholder="New project name" aria-label="New project name for environment ${e}"> <button data-create-map="${e}">Create &amp; map</button>`;
  return `<tr data-env-row="${e}"><td><b>${e}</b></td><td>Unmapped — ${picker}</td><td>${create}</td></tr>`;
}

// The mapping panel. Renders an explanatory empty state when discovery hasn't surfaced any
// environments yet (never blank — the user is told to run an import).
export function mappingPanel(envs: DiscoveredEnvState[], projects: PickerProject[]): string {
  const rows = envs.length
    ? envs.map((env) => mappingRow(env, projects)).join('')
    : '<tr><td colspan="3" class="empty">No environments discovered yet. Run an import from Langfuse to discover environments, then map each one to a Vire project.</td></tr>';
  return `<section class="panel"><h2>Environment → project mapping</h2><p>Each discovered Langfuse environment can be mapped to a Vire project. Vire never creates a project automatically — use “Create project for …” to make one and map it in a single explicit action.</p><table class="env-map"><tr><th>Environment</th><th>Mapping</th><th></th></tr>${rows}</table></section>`;
}
