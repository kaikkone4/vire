// Pure HTML builders for the U-lite update-check surface (TASK-050). No DOM or IPC here;
// the click handlers that invoke check_for_update / open_releases_page live in main.ts.
// All caller-supplied text is escaped. Three states mirror the Rust `UpdateCheck` enum:
// up_to_date, update_available, unknown (fail-soft — never an error modal).

import { escapeHtml as esc } from './html';

// Mirrors the Rust `UpdateCheck` serde output (tag = "status", snake_case).
export type UpdateCheckResult =
  | { status: 'up_to_date'; current: string }
  | { status: 'update_available'; current: string; latest: string; release_url: string }
  | { status: 'unknown'; reason: string };

// Idle state — no check has been run yet this session.
export function updateCheckIdle(): string {
  return (
    `<div id="updateCheckResult" class="update-check-idle">` +
    `<button id="checkForUpdates">Check for updates</button>` +
    ` <button id="openReleasesPage">Open GitHub Releases</button>` +
    `</div>`
  );
}

// Checking state — spinner/disabled to prevent double-click storms.
export function updateCheckPending(): string {
  return (
    `<div id="updateCheckResult" class="update-check-pending">` +
    `<button id="checkForUpdates" disabled>Checking…</button>` +
    ` <button id="openReleasesPage">Open GitHub Releases</button>` +
    `</div>`
  );
}

// Result state — renders one of the three enum arms.
export function updateCheckResult(r: UpdateCheckResult): string {
  let status = '';
  if (r.status === 'up_to_date') {
    status = `<span class="update-up-to-date">Vire ${esc(r.current)} is up to date.</span>`;
  } else if (r.status === 'update_available') {
    status =
      `<span class="update-available">Update available — v${esc(r.latest)}</span>` +
      ` <button id="openReleasesSpecific" data-release-url="${esc(r.release_url)}">` +
      `Open release page</button>`;
  } else {
    // unknown — quiet fail-soft copy; never implies the app is broken.
    status = `<span class="update-unknown">Couldn't check — try again later.</span>`;
  }
  return (
    `<div id="updateCheckResult" class="update-check-result">` +
    `${status}` +
    ` <button id="checkForUpdates">Check again</button>` +
    ` <button id="openReleasesPage">Open GitHub Releases</button>` +
    `</div>`
  );
}

// Full update-check panel rendered inside the Settings view.
// `result` is null before any check is run this session.
export function updateCheckPanel(result: UpdateCheckResult | null, pending = false): string {
  const inner = pending
    ? updateCheckPending()
    : result != null
      ? updateCheckResult(result)
      : updateCheckIdle();
  return (
    `<section class="panel" id="updateCheckPanel"><h2>App updates</h2>` +
    `<p>Vire does not check for updates automatically. Click below to see if a newer version ` +
    `is available on GitHub. No app data is sent — the check is a read-only request to ` +
    `the GitHub public API.</p>` +
    inner +
    `</section>`
  );
}
