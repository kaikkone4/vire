# SW-5 Security Review — TASK-045 settings-mapping-completeness

- **Change**: `openspec/changes/task-045-settings-mapping-completeness/`
- **Branch / PR**: `feat/task-045-settings-mapping-completeness` → PR #33
- **Tier**: L2 (secrets + CVE ≥ 7 + Trivy HIGH/CRITICAL + semgrep ERROR)
- **Primary control**: **SEC-010** — mapping/discovery surfaces carry only environment names,
  project references, and mapping state; no credential, raw payload, or trace content.
- **Verdict**: **PASS** — no auto-fail condition hit.

## 1. Scope reviewed

Backend-only diff (no renderer/IPC/manifest change):

- `src-tauri/src/env_mapping/mod.rs` — `list_discovered_environments_repo` now returns the
  **union** of discovered ∪ distinct `langfuse_ai_evidence.environment` ∪ distinct
  `langfuse_env_project_map.environment` (BTreeMap → sorted, de-duped).
- `src-tauri/src/langfuse/mod.rs` — discovery look-back uses `discovery_window(range_floor, now)`
  instead of the fixed 7-day `recent_window`. Removed dead `recent_window`/`DISCOVERY_WINDOW_DAYS`.
- `src-tauri/src/langfuse/discovery.rs` — `MAX_PAGES` visibility raised to `pub` (value unchanged).
- `src-tauri/src/env_mapping/tests.rs`, `src-tauri/src/langfuse/tests.rs` — new tests + SEC-010
  false-positive hardening.

## 2. Scanner results (Tier 1 stack)

| Scanner | Scope | Result | Auto-fail? |
|---------|-------|--------|------------|
| **gitleaks** 8.30.1 | full history (191 commits, 3.42 MB) | **0 leaks** | No |
| **semgrep** 1.166.0 | `--config auto --severity ERROR` over `src-tauri/src/` (53 rules, 26 files) | **0 findings** | No |
| **OSV-scanner** 2.3.8 | `src-tauri/Cargo.lock` (492 pkgs) + `package-lock.json` (106 pkgs) | npm 0; Cargo advisories all **unmaintained/low** — **none CVE ≥ 7.0** | No |
| **Trivy** 0.71.1 | `fs --scanners vuln,secret --severity HIGH,CRITICAL` | **0 HIGH/CRITICAL**, **0 secrets** | No |

### OSV detail (all pre-existing — no lockfile changed in this PR)

The 18 Cargo advisories are the standard Tauri-v1 / GTK transitive set, all **informational**:

- GTK/GDK/atk bindings unmaintained — RUSTSEC-2024-0411…0420, -0429 (no CVSS; the one scored entry,
  `glib` GHSA-wrw7-89jp-8q8g, is CVSS 4.0 `VI:L` = **low**).
- `proc-macro-error` unmaintained — RUSTSEC-2024-0370 (no CVSS).
- `unic-*` unmaintained — RUSTSEC-2025-0075/0080/0081/0098/0100 (no CVSS).

None meets the L1/L2 CVE ≥ 7.0 auto-fail threshold. `git diff main...HEAD` touches **no**
`Cargo.lock` / `Cargo.toml` / `package.json` / `package-lock.json` — this change introduces no new
dependency, so it neither adds nor worsens any advisory.

## 3. Manual review — SEC-010 + trust boundary (all PASS)

1. **No token / cost / session / payload leak.** The new evidence query selects only
   `environment, COALESCE(MAX(ai_end_ts), MAX(ai_start_ts), '')`; the map query selects only
   `DISTINCT environment`. The token/cost/session columns of `langfuse_ai_evidence` are never
   touched. `DiscoveredEnvState` is unchanged (`environment`, `last_seen`, `mapped`, `project_id`,
   `project_name`) — no new field crosses the IPC boundary.
2. **Environment names are not a new data class.** Names from evidence/maps were already persisted
   and already exposed via the discovery surface; the union only changes *which rows* render, not
   *what kind of data* leaves the backend. `last_seen` is a timestamp or empty string — display-only.
3. **No SQL injection.** Both new statements are static SQL with no interpolation; the per-env
   mapping join stays parameterized (`params![environment]`).
4. **Egress / allowlist / loopback unchanged.** The wider discovery window changes only the
   `from`/`to` RFC3339 timestamps passed to the existing `get_traces_any_env` path. URL construction,
   the `/api/public/` allowlist, and the `is_loopback_host` gate live in `config.rs::build_url` —
   **not in this diff**. Same host, same GET-only read, no new egress destination.
5. **Resource bound preserved.** A wider (up to `all`) floor scans more pages but stays capped by
   `discovery::MAX_PAGES = 1000`; the loop condition `page >= MAX_PAGES` is unchanged. Test
   `discovery_is_bounded_by_max_pages_so_an_all_floor_cannot_spin` proves an effectively-infinite
   source terminates at exactly `MAX_PAGES`. Discovery remains read-only and best-effort (a failure
   never fails the import). The `pub` change on `MAX_PAGES` is visibility-only.
6. **No credential / keychain path change.** The `KeyringSecretStore` import is untouched; the
   removed code was the `recent_window`/`DISCOVERY_WINDOW_DAYS` helper, not credential handling.
7. **No new capability.** No Tauri capability, command surface, FS, or network capability added.
8. **SEC-010 test strengthened.** `mapping_surfaces_carry_no_secrets` now neutralizes the legitimate
   random `project_id` UUID (whose hex could coincidentally contain the `579` needle) before
   asserting absence of `session-`/`Bearer`/`leak`/`sk-`/`pk-`/`9.99`/`579`. This removes a false
   positive without weakening coverage — the needle set is unchanged.

## 4. Escalations

None. No design-level (trust-boundary / missing-auth) issue. DEC-038 (surface = discovered ∪
has-evidence ∪ mapped) is a BA-owned decision already recorded in `design.md` §7 — non-blocking,
no security implication beyond what is reviewed above.

## 5. Verdict

**PASS.** No auto-fail condition hit: 0 secrets (gitleaks), 0 semgrep ERROR, no CVE ≥ 7.0 (OSV),
0 Trivy HIGH/CRITICAL, SEC-010 verified secret-free by code review and test. Proceed; release to
SW-6 once SW-4 Code Review also passes.

Raw scanner reports: `/tmp/sec045-gitleaks.json`, `/tmp/sec045-semgrep.json`,
`/tmp/sec045-osv.json`, `/tmp/sec045-osv-npm.json`, `/tmp/sec045-trivy.json`.
