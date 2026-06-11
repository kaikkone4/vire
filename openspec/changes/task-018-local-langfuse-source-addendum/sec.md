# Security Review — TASK-018 local Langfuse source supersession addendum

- **Security Agent:** SW-5
- **Change:** `task-018-local-langfuse-source-addendum`
- **PR:** #10 — base `feat/task-003-implementation-path-decision` → head `feat/task-018-local-langfuse-source-addendum`
- **Tier:** L2 (secrets + CVE ≥ 7 + Trivy HIGH/CRITICAL + semgrep ERROR)
- **Date (initial pass):** 2026-06-11
- **Date (re-check after fix commit `bdada81`):** 2026-06-11
- **Verdict:** **PASS** — no auto-fail condition hit. Both prior advisories (SEC-ADV-01, SEC-ADV-02) now **RESOLVED** by fix commit `bdada81`; re-scan clean.

This is a docs/OpenSpec realignment change. No product runtime is created or modified
(`git diff main...HEAD -- src/ src-tauri/src/ observability/` is empty). Review covers the
8 PR-diff files plus a manual cross-check against the pre-existing committed Langfuse stack the
docs describe.

---

## 1. Scope reviewed

PR #10 diff (base task-003 → head task-018), 8 files:

- `README.md` (+64/-2) — Local Langfuse stack, AI trace import, privacy sections
- `docs/langfuse-local-setup.md` (new, 151 lines)
- `docs/backup-restore.md` (new, 158 lines)
- `openspec/changes/task-018-local-langfuse-source-addendum/{arch-review,proposal,qa}.md` (new)
- `openspec/changes/task-003-implementation-path-decision/{arch-review,design}.md` (additive
  supersession banners only)

Cross-checked (not in PR diff, but the docs make security claims about it): the pre-existing
`observability/langfuse/docker-compose.yml` + `observability/langfuse/README.md` + `.gitignore`
(all already in `main`).

## 2. Scanner stack (Tier 1, L2 active)

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** v8.30.1 | Full history (67 commits) + PR range (`task-003..HEAD`, 2 commits) | **no leaks** | No |
| **semgrep** v1.165.0 | `--config=auto`, 478 rules on 43 git-tracked files | **0 findings** (0 ERROR) | No |
| **OSV-scanner** v2.3.8 | `package-lock.json` (106 packages), recursive | **no issues** (0 CVE ≥ 7) | No |
| **Trivy** v0.71.1 | `fs --scanners vuln,secret,misconfig --severity HIGH,CRITICAL` | **0 HIGH/CRITICAL** | No |

Supplementary manual scans of the PR diff:
- Personal/sensitive absolute paths (`/Users/`, `/home/`, `/Volumes/`) in added lines: **none**.
- Credential patterns (`github_pat_`/`ghp_`/`sk-ant-`/`AKIA…`/`xox?-`/PEM private-key headers/
  inline `password=`): **none**. Docs use template placeholders only
  (`${MINIO_ACCESS_KEY}`, `<pg-user>`, `<pg-db>`, `<timestamp>`, `$(pwd)/backup`).

**No auto-fail condition reached.**

## 3. Manual review against the stated security focus

| # | Focus | Evidence | Verdict |
|---|---|---|---|
| 1 | No secret values, credential paths, tokens, private keys, or sensitive local paths in docs/PR | Diff grep + gitleaks + Trivy-secret all clean; docs use placeholders/env-var substitution; backup paths are generic (`$(pwd)/backup`); compose default email is the placeholder `janne@example.local`, not a real address | **PASS** |
| 2 | Local Docker Langfuse default = localhost/`127.0.0.1`, no accidental LAN exposure | `docs/langfuse-local-setup.md` §Localhost binding (long+short-form `127.0.0.1` examples; internal services unpublished); README "All service ports must be bound to `127.0.0.1` … Do not expose services on LAN interfaces"; default endpoint `http://127.0.0.1:3000` throughout. Committed `observability/langfuse/docker-compose.yml` confirms it: only `langfuse-web` publishes a port (`127.0.0.1:${LANGFUSE_PORT:-3000}:3000`); Postgres/ClickHouse/Redis/MinIO publish **no** host ports | **PASS** |
| 3 | MinIO/S3: bucket stays private; creds not committed/logged/exported; PG/CH/MinIO backup consistency visible | `langfuse-local-setup.md` §MinIO ("`langfuse` bucket **must not be set to public**"; keys "never logged, committed, exported"); README §Object storage cautions; `backup-restore.md` "all three persistent stores … consistent", divergence-failure-modes table, post-restore "`langfuse` bucket exists and is not public" check | **PASS** |
| 4 | Docker/Langfuse down or missing traces never treated as zero AI usage/cost | `langfuse-local-setup.md` health-state table + "**Vire never interprets Docker down … as zero AI usage or cost**"; README §Availability and UX; `arch-review.md` §5 item 7 ("evidence gap, never a zero-AI-usage/cost signal") | **PASS** |
| 5 | Trace content boundary honest: local Langfuse may include prompt/session/metadata; stricter retention/redaction later | `langfuse-local-setup.md` §Trace content boundary; README §Privacy status; `arch-review.md` §5 item 8 — all state prompt/session/metadata is accepted within the local boundary for MVP, with redaction/retention as an explicit L2 follow-up. Honest, not overclaimed | **PASS** |
| 6 | Cloud override stays explicit/non-default; no reintroduced raw macOS activity egress | README "Langfuse Cloud is supported only as an explicit non-default override"; "Vire does not upload macOS activity, window titles, prompts, command bodies, or raw local evidence to Langfuse Cloud"; `arch-review.md` §5 item 2 and §7 SEC-002 row ("off-host egress now occurs **only** on explicit Cloud override"). Raw macOS activity stays in local SQLite, never mixed into traces | **PASS** |

## 4. Boundary / posture check (SEC-002 / SEC-003)

- **SEC-002 (network egress):** Default importer target moves cloud → loopback (`127.0.0.1:3000`).
  Importer stays read-only, base-URL-allowlisted, Langfuse-only, behind the locked webview CSP
  (`connect-src ipc:`). Off-host egress occurs **only** on explicit Cloud override. **Preserved /
  strengthened.**
- **SEC-003 (credentials):** Now also covers local-stack secrets (Postgres/ClickHouse/MinIO/Redis
  passwords, `NEXTAUTH_SECRET`, `SALT`, `ENCRYPTION_KEY`, Langfuse API keys); docs require local
  `.env`/Keychain storage and prohibit repo/log/export/support-bundle exposure. Verified against the
  committed stack: `.gitignore` excludes `observability/langfuse/.env` and `.env.*` while keeping
  `!.env.example`; the compose injects every secret via `${VAR:?…}` required-error guards with **no
  hardcoded values**; `observability/langfuse/README.md` documents `.env` chmod `600` + gitignore.
  **Preserved / broadened.**

The committed dev-observability stack (`observability/langfuse/`) is *more* restrictive than the docs
describe — MinIO and its console are not host-published at all. No insecure default is introduced or
recommended by this PR.

> Note: `observability/langfuse/.env.example` is blocked by this runner's permission settings and was
> not opened directly. Its safety is established indirectly: gitleaks (full history) and Trivy-secret
> both scanned the tree and reported zero secrets, and `.env` (the real-value file) is gitignored.

## 5. Advisory findings (non-blocking — documentation accuracy)

> **Status after fix commit `bdada81`:** both advisories below are **RESOLVED**. Verification
> recorded in §7. They are retained here for audit trail.

**SEC-ADV-01 [RESOLVED] — Docs claim no compose file exists, but a secure one already does.**
`README.md` (§Setup follow-up) and `docs/langfuse-local-setup.md:36` state a project-local
`docker-compose.yml` "does not exist in this repo yet," directing readers to hand-build one from the
upstream Langfuse guide. A loopback-bound, secret-safe Langfuse compose already exists at
`observability/langfuse/docker-compose.yml` (in `main`, commits `9ef677b`/`249158c`/`1592d9c`),
scoped in its README as Janne's local dev/observability stack. Not a security defect — the existing
file is secure — but steering readers to roll their own slightly raises the chance someone fumbles
the `127.0.0.1` binding instead of reusing the vetted stack. Recommend the docs cross-reference
`observability/langfuse/` (or clarify the "Vire-product-bundled compose vs. existing dev stack"
distinction). Owner: developer / code-reviewer.

**SEC-ADV-02 [RESOLVED] — MinIO port table inaccurate.**
README and `langfuse-local-setup.md` list MinIO API at `127.0.0.1:9090` and console at `9001`. The
committed compose publishes **neither** to the host (MinIO is internal-bridge-only; console is
`--console-address ":9001"` inside the container, unpublished). Reality is stricter than documented,
so there is no exposure risk — but the table is wrong and should be corrected to avoid a future
implementer "fixing" the docs by exposing the port. Owner: developer / code-reviewer.

Both advisories are documentation-correctness items, not security auto-fails. They do not block this
gate. No design-level boundary issue → no BA-flow Architect escalation.

## 6. Re-check after fix commit `bdada81` (SW-5 re-audit)

Re-ran the full L2 scanner stack and re-verified the two advisories against the post-fix tree.

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** v8.30.1 | Full history (68 commits) + PR range (`main..HEAD`, 9 commits) | **no leaks found** | No |
| **semgrep** v1.165.0 | `--config=auto`, 44 files scanned | **0 findings (0 ERROR)** | No |
| **OSV-scanner** v2.3.8 | `package-lock.json` (106 packages) | **no issues** (0 CVE ≥ 7) | No |
| **Trivy** v0.71.1 | `fs --scanners vuln,secret,misconfig --severity HIGH,CRITICAL` | **0 vuln / 0 secret / 0 misconfig** | No |

Advisory resolution verified (fix commit `bdada81`):

- **SEC-ADV-01 [RESOLVED]** — The false "compose file does not exist in this repo yet" claim is gone
  from both `README.md` and `docs/langfuse-local-setup.md` (`grep "does not exist" README.md docs/` →
  empty). Both now carry an "Existing local stack" callout pointing to
  [`observability/langfuse/docker-compose.yml`](../../../observability/langfuse/docker-compose.yml)
  and steer readers to reuse the vetted loopback-bound stack rather than hand-roll one (lowering the
  earlier mis-binding risk). The product-bundled-compose question is correctly scoped as an open
  TASK-007 follow-up with the same localhost-only / env-var-reference constraints.
- **SEC-ADV-02 [RESOLVED]** — The inaccurate `127.0.0.1:9090` MinIO host-port claim is gone
  (`grep "9090" README.md docs/` → empty). MinIO is now documented as **internal-only**: README
  table row reads `internal (not host-published)`; `langfuse-local-setup.md` reads
  `none (internal); API minio:9000, console :9001 inside the container — not host-published`, with an
  explicit "do not add `ports:` entries for them" note. The lone remaining `9001` reference is the
  negative statement "There is no `127.0.0.1:9001` mapping by default" — confirming absence, not
  exposure. Cross-checked the committed `observability/langfuse/docker-compose.yml`: the only host
  publication is `127.0.0.1:${LANGFUSE_PORT:-3000}:3000` (langfuse-web); MinIO has no `ports:` entry
  and runs `--console-address ":9001"` in-container. **Docs now match the stricter reality.**

No new security issue introduced by the fix commit; the doc changes only **strengthen** the stated
posture (loopback default, MinIO internal-only, Cloud explicit-override, Docker-down ≠ zero cost,
honest trace-content boundary all intact and re-confirmed). The lone uncommitted working-tree change
(`task-018/qa.md`, the SW-3 re-run report) is docs-only and carries no secrets or posture change.

## 7. Verdict

**PASS.** All four L2 scanners clean on re-check (no secrets, no CVE ≥ 7, no semgrep ERROR, no Trivy
HIGH/CRITICAL). All six stated security focus points hold. SEC-002/SEC-003 are preserved and
strengthened by the loopback-default posture. Both prior documentation-accuracy advisories
(SEC-ADV-01, SEC-ADV-02) are now **RESOLVED** by fix commit `bdada81` — no open advisories remain.

Handoff: wait for SW-4 (Code Review); on both PASS, route to SW-6 (Release Manager).
