// Pure scroll-position decision for the full-innerHTML re-render in main.ts `shell()` (TASK-031).
// `shell()` rebuilds the whole tree on every render, which destroys and recreates the `<main>` scroll
// container and resets its scrollTop to 0 — so pressing any Settings control jumped the viewport to the
// top. The fix captures the outgoing scrollTop and restores it on a same-view re-render; a view change
// (or the first render, when there is no previous view) starts at the top.
//
// Only this decision is pure and unit-tested here; the live-DOM capture/restore wiring stays in
// `shell()`, which cannot run outside a webview.
export function nextScrollTop(sameView: boolean, prevScroll: number): number {
  return sameView ? prevScroll : 0;
}
