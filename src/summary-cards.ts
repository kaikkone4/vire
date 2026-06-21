// Pure HTML builder for the project summary cards (Today + Reports). Kept DOM/IPC-free so the card
// markup — including the AI-cost sub-line (TASK-034 B5) — stays unit-testable; main.ts only supplies
// the data and binds nothing here. Human (origin='manual') time is the headline figure; accepted
// AI-suggested time and its cost are shown on a separate sub-line and never folded into the human
// total, so prior numbers never shift (DEC-003 / TASK-032 B4). AI cost is NULL when unknown or
// mixed-currency → "—", never "0" (absence ≠ zero — DEC-004).
import { escapeHtml as esc } from './html';
import { costLabel } from './suggestions-ui';

// Mirrors the backend `SummaryRow` (lib.rs) — the fields summaryCards renders. `ai_cost_*` are
// optional/nullable: NULL when no accepted AI entry in range carries a cost.
export type SummaryCard = {
  project_id: string;
  project_name: string;
  duration_minutes: number;
  ai_minutes: number;
  ai_cost_total?: number | null;
  ai_cost_currency?: string | null;
};

const hm = (m: number): string => `${Math.floor(m / 60)}h ${m % 60}m`;

// Aggregate AI cost across cards for the lead "Total tracked" card. The sum is NULL when no card
// carries a cost (absence ≠ zero → "—"). A sum across differing currencies is not a meaningful single
// figure, so a mixed-currency set is also reported as NULL → "—" (never a bare cross-currency total);
// the normal case is a single currency (usually NULL — see design §4), which sums cleanly.
function aggregateAiCost(cards: SummaryCard[]): { total: number | null; currency: string | null } {
  const withCost = cards.filter((c) => c.ai_cost_total != null);
  if (!withCost.length) return { total: null, currency: null };
  const currencies = new Set(withCost.map((c) => c.ai_cost_currency ?? null));
  if (currencies.size > 1) return { total: null, currency: null };
  const total = withCost.reduce((a, c) => a + (c.ai_cost_total as number), 0);
  return { total, currency: withCost[0].ai_cost_currency ?? null };
}

// Project summary cards. `lead` adds the "Total tracked" human-total card with an aggregate AI
// sub-line (Today only). `emptyMsg` renders when there are no cards. The AI sub-line appears only
// when there is AI-suggested time, and always carries the cost (or "—" when absent).
export function summaryCards(summaries: SummaryCard[], lead: boolean, emptyMsg: string): string {
  const human = summaries.reduce((a, s) => a + s.duration_minutes, 0);
  const ai = summaries.reduce((a, s) => a + s.ai_minutes, 0);
  const agg = aggregateAiCost(summaries);
  const total = lead
    ? `<div class="card"><span>Total tracked</span><strong>${esc(hm(human))}</strong>${
        ai > 0
          ? `<small>AI-suggested ${esc(hm(ai))} · ${esc(costLabel(agg.total, agg.currency))}, reported separately</small>`
          : ''
      }</div>`
    : '';
  const cards = summaries
    .map(
      (s) =>
        `<div class="card"><span>${esc(s.project_name)}</span><strong>${esc(hm(s.duration_minutes))}</strong>${
          s.ai_minutes > 0
            ? `<small>AI-suggested ${esc(hm(s.ai_minutes))} · ${esc(costLabel(s.ai_cost_total ?? null, s.ai_cost_currency ?? null))}</small>`
            : ''
        }</div>`,
    )
    .join('');
  return `<div class="cards">${total}${cards || `<div class="empty">${esc(emptyMsg)}</div>`}</div>`;
}
