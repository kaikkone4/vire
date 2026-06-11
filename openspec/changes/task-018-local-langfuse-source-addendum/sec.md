# Security Review — TASK-018 local Langfuse source supersession addendum

- **Security Agent:** SW-5
- **Change:** `task-018-local-langfuse-source-addendum`
- **PR:** #10 — base `main` → head `feat/task-018-local-langfuse-source-addendum` (retargeted to `main` after TASK-003 landed via PR #9; previously stacked on `feat/task-003-implementation-path-decision`)
- **Tier:** L2 (secrets + CVE ≥ 7 + Trivy HIGH/CRITICAL + semgrep ERROR)
- **Date (initial pass):** 2026-06-11
- **Date (re-check after fix commit `bdada81`):** 2026-06-11
- **Date (final re-check after metadata-hygiene commit `70f677e` + PR-body edit):** 2026-06-11
- **Date (final re-check after OpenSpec validation/spec-delta commit `1955eb4`):** 2026-06-11
- **Verdict:** **PASS** — no auto-fail condition hit. Both prior advisories (SEC-ADV-01, SEC-ADV-02) **RESOLVED** by fix commit `bdada81`; re-scan clean. Final re-check (§9) confirms the OpenSpec validation fix + new `specs/langfuse-trace-source/spec.md` ADDED spec delta are docs/OpenSpec-only and **strengthen, not weaken**, the loopback-default posture.

This is a docs/OpenSpec realignment change. No product runtime is created or modified
(`git diff main...HEAD -- src/ src-tauri/src/ observability/` is empty). Review covers the
10 PR-diff files plus a manual cross-check against the pre-existing committed Langfuse stack the
docs describe.

---

## 1. Scope reviewed

PR #10 diff (base `main` → head task-018), 10 files:

- `README.md` (+64/-2) — Local Langfuse stack, AI trace import, privacy sections
- `docs/langfuse-local-setup.md` (new, 153 lines)
- `docs/backup-restore.md` (new, 158 lines)
- `openspec/changes/task-018-local-langfuse-source-addendum/{arch-review,proposal,qa,sec}.md` (new)
- `openspec/changes/task-018-local-langfuse-source-addendum/specs/langfuse-trace-source/spec.md`
  (new — minimal ADDED spec delta recording the downstream default-source requirement; docs/OpenSpec
  text only, no runtime)
- `openspec/changes/task-003-implementation-path-decision/{arch-review,design}.md` (additive
  supersession banners only)

Cross-checked (not in PR diff, but the docs make security claims about it): the pre-existing
`observability/langfuse/docker-compose.yml` + `observability/langfuse/README.md` + `.gitignore`
(all already in `main`).

## 2. Scanner stack (Tier 1, L2 active)

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** v8.30.1 | Full history + PR range (`origin/main..HEAD`, task-018 docs commits — base retargeted to `main` after TASK-003 merged via PR #9; current set enumerated in §6 post-merge note) | **no leaks** | No |
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
| **gitleaks** v8.30.1 | Full history + PR range (`origin/main..HEAD`, all task-018 docs commits — see post-merge note below for the current set) | **no leaks found** | No |
| **semgrep** v1.165.0 | `--config=auto`, 44 files scanned | **0 findings (0 ERROR)** | No |
| **OSV-scanner** v2.3.8 | `package-lock.json` (106 packages) | **no issues** (0 CVE ≥ 7) | No |
| **Trivy** v0.71.1 | `fs --scanners vuln,secret,misconfig --severity HIGH,CRITICAL` | **0 vuln / 0 secret / 0 misconfig** | No |

> **Post-merge re-verification (2026-06-11):** TASK-003 has since landed on `main` via PR #9, so PR #10
> was retargeted from the former `feat/task-003-implementation-path-decision` base to `main`. The
> reviewable PR range is therefore `origin/main..HEAD` — the task-018 docs/OpenSpec commits only;
> TASK-003's commits are now part of `main`. That set has grown as gate evidence and metadata fixes
> landed: it is **6 task-018 commits** as of this validity/metadata-fix commit — `7c7a663` (package),
> `a0d08b0` (QA gate), `bdada81` (SW-2 fix), `7526a0e` (QA re-run + SEC re-audit), `70f677e`
> (metadata hygiene), and this fix (spec delta + stale PR-range/scanner-scope sync). gitleaks was
> re-run on the current tree post-merge: **no leaks** (the gitleaks row above reflects this). The
> semgrep / OSV / Trivy figures are the SW-5 re-audit results on the same docs-only tree and are
> unaffected by the merge or by these docs-only commits. The PASS verdict is unaffected.

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
honest trace-content boundary all intact and re-confirmed). The SW-3 re-run report
(`task-018/qa.md`) is committed (`7526a0e`) and the working tree is clean; it is docs-only and
carries no secrets or posture change.

## 7. Verdict

**PASS.** All four L2 scanners clean on re-check (no secrets, no CVE ≥ 7, no semgrep ERROR, no Trivy
HIGH/CRITICAL). All six stated security focus points hold. SEC-002/SEC-003 are preserved and
strengthened by the loopback-default posture. Both prior documentation-accuracy advisories
(SEC-ADV-01, SEC-ADV-02) are now **RESOLVED** by fix commit `bdada81` — no open advisories remain.

Handoff: wait for SW-4 (Code Review); on both PASS, route to SW-6 (Release Manager).

## 8. Final re-check after metadata-hygiene commit `70f677e` + PR-body edit (SW-5)

Trigger: SW-2 metadata-hygiene commit `70f677e` ("sync stale PR-state after TASK-003 merged to
main") and a PR-body edit removing stale merge-order text. Confirming the security posture still
holds with no file-content change since the SW-3 QA re-run.

**Changeset character.** PR #10 diff is still docs/OpenSpec-only — `git diff --stat origin/main...HEAD`
= `README.md`, `docs/{backup-restore,langfuse-local-setup}.md`, and the task-003 supersession banners
+ task-018 `{proposal,arch-review,qa,sec}.md` (9 files, 935 insertions). No runtime source, no
dependency manifest, no container/Compose file, no lockfile in the diff
(`git diff --name-only origin/main...HEAD | grep -E 'package.*json|cargo|Dockerfile|docker-compose|\.lock'`
→ empty). `70f677e` touches documentation/metadata only.

**PR body re-verified.** The updated body's "Merge status" now reads "TASK-003 has already landed on
`main` via PR #9 … No merge-order dependency remains" — the former stale stacked-branch / merge-order
text is gone. Base `main`, head `feat/task-018-local-langfuse-source-addendum`, draft. No security
claim (loopback default, no-LAN-exposure, MinIO private, Cloud explicit-override, Docker-down ≠ zero
cost) is weakened by the edit; the body still states "No product runtime source changed. No
credentials introduced (secret-pattern scan clean)."

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** | Full history (70 commits, incl. `70f677e`) + working tree | **no leaks found** | No |
| **semgrep** | `--config=auto --severity=ERROR` on `README.md`, `docs/`, task-018 change dir | **0 ERROR findings** | No |
| **OSV-scanner** (CVE ≥ 7) | n/a — no dependency manifest changed in PR | unchanged from §6 (0 CVE ≥ 7) | No |
| **Trivy** (HIGH/CRITICAL) | n/a — no container image / Compose / IaC changed in PR | unchanged from §6 (0 HIGH/CRITICAL) | No |

Supplementary diff grep for literal credential values (`github_pat_`/`ghp_`/`sk-ant-`/`AKIA…`/`xox?-`/
inline `password=`/`secret=`/`api_key=`) over added lines: **none** — the only match is sec.md
describing its own scan patterns, not a real value. Docs continue to use placeholders / env-var
substitution only.

**Seven-point posture re-confirmation (all hold):**

1. No secrets / credential values / sensitive paths — gitleaks + semgrep + diff-grep clean. **PASS**
2. Loopback default, no LAN-exposure recommendation — `langfuse-local-setup.md:42,63`, README "All
   service ports must be bound to `127.0.0.1` … Do not expose services on LAN interfaces". **PASS**
3. MinIO internal/private + backup consistency visible — `langfuse-local-setup.md:61,94,100`
   (internal-only, "must not be set to public"); `backup-restore.md:9` three-store consistency +
   divergence table. **PASS**
4. Docker/Langfuse down never zero cost — `langfuse-local-setup.md:119` "**Vire never interprets
   Docker down … as zero AI usage or cost**"; README §Availability and UX. **PASS**
5. Cloud override explicit-only — `langfuse-local-setup.md:153`, README "Langfuse Cloud is supported
   only as an explicit non-default override". **PASS**
6. Trace-content boundary honest — `langfuse-local-setup.md:123-129`, README §Privacy status; local
   boundary accepted for MVP, redaction/retention scoped as follow-up, raw macOS activity kept in
   local SQLite and never mixed into traces. **PASS**
7. Docs/OpenSpec only, no runtime source changes — diff `--stat` confirms 9 docs/spec files only.
   **PASS**

**Verdict unchanged: PASS.** No auto-fail condition reached. No open advisories. The metadata-hygiene
commit and PR-body edit are non-code, non-posture changes; the loopback-default security posture is
intact and re-confirmed. No design-level issue → no BA-flow Architect escalation.

> **Scoping note (SW-2, post-§8 validity/metadata-fix commit).** The §8 figures above (9 docs/OpenSpec
> files, full-history commit count, "diff `--stat` confirms 9 files") describe the tree **as of
> `70f677e`**, the commit SW-5's final re-check ran against. A subsequent SW-2 commit then closed the
> SW-4 final-review blockers: it adds the minimal `specs/langfuse-trace-source/spec.md` ADDED spec
> delta (so `openspec validate --strict` passes) and syncs the stale §2/§6 PR-range and scanner-scope
> text. That makes the PR **10 docs/OpenSpec files** across the task-018 commits on `origin/main..HEAD`
> (enumerated in the §6 post-merge note). The fix commit is documentation/OpenSpec text only — no
> runtime source, dependency manifest, container/Compose, or lockfile touched; the added spec delta
> uses prose and placeholders only, introduces no credential values, and changes no security claim.
> SW-5's PASS verdict and the seven-point posture re-confirmation above are unaffected.

## 9. Final re-check after OpenSpec validation / spec-delta commit `1955eb4` (SW-5)

Trigger: SW-2 fix commit `1955eb4` ("add OpenSpec spec delta + sync stale PR-range/scanner-scope
metadata"), which closed the last SW-4 blocker by adding the minimal
`specs/langfuse-trace-source/spec.md` ADDED delta so `openspec validate --strict` passes, and synced
the stale §2/§6 PR-range/scanner-scope text. This is the final SW-5 check before SW-6.

**Changeset character.** PR #10 diff is now **10 docs/OpenSpec files** (`git diff --name-status
main...HEAD`): `README.md`, `docs/{backup-restore,langfuse-local-setup}.md`, the task-003
`{arch-review,design}.md` supersession banners, and task-018
`{arch-review,proposal,qa,sec}.md` + `specs/langfuse-trace-source/spec.md`. **No runtime source**
(`git diff --name-only main...HEAD -- src/ src-tauri/ observability/` → empty), no dependency
manifest, no container/Compose, no lockfile.

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** v8.30.1 | Full history (71 commits, incl. `1955eb4`) + working tree | **no leaks found** | No |
| **semgrep** v1.165.0 | `--config=auto --severity=ERROR` on tracked tree | **0 ERROR findings** | No |
| **OSV-scanner** v2.3.8 | `package-lock.json` (106 packages), recursive | **no issues** (0 CVE ≥ 7) | No |
| **Trivy** v0.71.1 | `fs --scanners vuln,misconfig,secret --severity HIGH,CRITICAL` | **0 HIGH/CRITICAL** | No |

Supplementary diff grep for literal credential values (`ghp_`/`github_pat_`/`sk-ant-`/`AKIA…`/`xox?-`/
PEM private-key headers/inline `password=`/`secret=`) over added lines: the only matches are sec.md
text describing its own scan patterns — **no real value**. Docs use placeholders / `${VAR}`
substitution only.

**New spec delta — control-weakening check.** `specs/langfuse-trace-source/spec.md` is an **ADDED**
requirement (no existing requirement modified or removed). It *codifies* the controls rather than
relaxing them: loopback (`127.0.0.1:3000`) default, Cloud as explicit non-default override only,
a down/unreachable stack never read as zero usage/cost (evidence gap), and MinIO/S3 documented as
internal/private (not host-published, not public) with three-store backup-consistency risk surfaced.
Each scenario in the delta matches — and binds — the prose posture in the README and the two `docs/`
files. **The delta strengthens controls; it weakens none.**

**Seven-point posture re-confirmation (all hold):** (1) no secrets/paths — scanners + grep clean;
(2) loopback default / no LAN exposure — `langfuse-local-setup.md:42,63`, spec-delta loopback
scenario; (3) MinIO internal/private + backup consistency — `langfuse-local-setup.md:61,94`,
`backup-restore.md:9` + divergence table, spec-delta MinIO scenario; (4) Docker-down ≠ zero cost —
`langfuse-local-setup.md:119`, spec-delta down-stack scenario; (5) Cloud explicit-only —
`langfuse-local-setup.md:153`, spec-delta default/override scenario; (6) trace-content boundary
honest — `langfuse-local-setup.md:123-129`, README §Privacy status; (7) docs/OpenSpec only —
runtime-source diff empty. All **PASS**.

**Verdict unchanged: PASS.** No auto-fail condition reached, no open advisories, no design-level
boundary issue → no BA-flow Architect escalation. Handoff: wait for SW-4 (Code Review); on both
PASS, route to SW-6 (Release Manager).

## 10. Final re-check after SW-4 blocker-close commit `21f5fdd` + SW-3 QA re-run `dbb9c6f` (SW-5)

Trigger: two commits landed after the §9 check — `21f5fdd` ("SW-2 fix — close SW-4 final blockers:
arch §8 spec-delta truth + active 10-file scope") and `dbb9c6f` ("SW-3 final QA re-run after fix
commit `21f5fdd` — PASS"). This is the final SW-5 re-check before SW-6.

**Changeset character.** The two new commits touch documentation only — `arch-review.md` §8
(replacing a stale "no spec delta" item with the current truth), `sec.md` (active header 9→10 files +
committing the §9 re-check), and `qa.md` (F-01–F-06 verification matrix). No product runtime, no
dependency manifest, no container/Compose, no lockfile:

- `git diff --name-only main...HEAD -- src/ src-tauri/ observability/` → **empty**
- `git diff --name-only main...HEAD | grep -E 'package.*json|cargo|Cargo|Dockerfile|docker-compose|\.lock'` → **empty**
- `git diff --name-only main...HEAD | grep -vE '\.md$'` → **empty** (all 18 PR-diff files are `.md`)
- Added-line credential grep over `1955eb4..HEAD` (`ghp_`/`github_pat_`/`sk-ant-`/`AKIA…`/`xox?-`/
  PEM headers/`password=`/`secret=`/`api_key=`): the only match is sec.md text describing its own
  scan patterns — **no real value**. No `/Users/`-`/home/`-`/Volumes/` paths in added lines.

| Scanner | Scope | Result | Auto-fail? |
|---|---|---|---|
| **gitleaks** | Full history (73 commits, incl. `21f5fdd` + `dbb9c6f`) + working tree | **no leaks found** | No |
| **semgrep** | `--config=auto --severity=ERROR` on tracked tree | **0 ERROR findings** | No |
| **OSV-scanner** | `package-lock.json` (106 packages) | **no issues** (0 CVE ≥ 7) | No |
| **Trivy** | `fs --scanners vuln,secret,misconfig --severity HIGH,CRITICAL` | **0 HIGH/CRITICAL** | No |

**Spec delta — control-weakening re-check.** `specs/langfuse-trace-source/spec.md` is unchanged by
these two commits and remains an **ADDED** requirement that codifies (does not relax) the loopback
default, Cloud explicit-override-only, down-stack ≠ zero usage/cost, and MinIO internal/private +
three-store backup-consistency controls. **Strengthens; weakens nothing.**

**Seven-point posture re-confirmation (all hold):** (1) no secrets/paths — scanners + diff grep
clean; (2) loopback default / no LAN — `langfuse-local-setup.md:42` ("All service ports must be bound
to `127.0.0.1` … not reachable from the local network"), README §91; (3) MinIO internal/private +
backup consistency — `langfuse-local-setup.md` (MinIO listed `none (internal)`, "must not be set to
public"), `backup-restore.md:9` three-store-consistency + divergence table; (4) Docker-down ≠ zero
cost — `langfuse-local-setup.md:119`; (5) Cloud explicit-only — README:91,
`langfuse-local-setup.md:153`; (6) trace-content boundary honest — `langfuse-local-setup.md:123`,
README §Privacy; (7) docs/OpenSpec only — runtime-source diff empty. All **PASS**.

**Verdict unchanged: PASS.** No auto-fail condition reached, no open advisories, no design-level
boundary issue → no BA-flow Architect escalation. The SW-4 blocker-close and SW-3 QA-re-run commits
are documentation-only and change no security claim. Handoff: on SW-4 PASS, route to SW-6 (Release
Manager).
