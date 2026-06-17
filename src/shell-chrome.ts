// Pure builder for the window titlebar markup (E1/E2). Extracted from main.ts's `shell()` so the
// chrome can be snapshot-tested without a DOM. TASK-027 removed the fake macOS traffic-light cluster
// the app used to draw itself — the native window decorations provide the real controls — so this
// markup must never re-introduce a `.traffic` element.

import { escapeHtml as esc } from './html';

export function titlebar(brand: string, version: string): string {
  return `<div class="titlebar"><b>${esc(brand)}</b><code>${esc(version)}</code></div>`;
}
