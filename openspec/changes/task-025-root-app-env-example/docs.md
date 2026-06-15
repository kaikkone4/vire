# Documentation Review — TASK-025 Root app env example for local MVP config

- **Documentation Engineer:** SW-6
- **Change:** `task-025-root-app-env-example`
- **Branch:** `docs/task-025-root-app-env-example` · **Base:** `origin/main`
- **Tier:** L2 · **Gate context:** SW-6 docs-only slice (no product runtime code)
- **Date:** 2026-06-15
- **Verdict:** **PASS — docs complete**

---

## Scope produced

| File | Action | Notes |
|---|---|---|
| `/.env.example` | Created | Root app runtime env template — safe defaults, empty secrets |
| `/.gitignore` | Updated | Added 3 root-anchored lines: `/.env`, `/.env.*`, `!/.env.example` |
| `README.md` | Updated | Added "App runtime configuration (env)" subsection; tightened Local Langfuse setup step 3 |
| `openspec/changes/task-025-root-app-env-example/tasks.md` | Updated | All deliverable tasks checked |
| `openspec/changes/task-025-root-app-env-example/docs.md` | Created | This file |

No product runtime code was modified (`*.rs`, `*.ts`, `Cargo.toml`, `package.json`,
`tauri.conf.json` untouched). No schema, migration, network, or export change.

---

## 1. Env-var names verified against Rust source

Authoritative source: `std::env::var` calls in the Rust config modules. All names in
`.env.example` match the code exactly:

| Variable | Rust source | Default in code | In template |
|---|---|---|---|
| `VIRE_LANGFUSE_BASE_URL` | `langfuse/config.rs:139` | `http://127.0.0.1:3000` | active default |
| `VIRE_LANGFUSE_SOURCE` | `langfuse/config.rs:140` | `local` | active default |
| `VIRE_LANGFUSE_ENVIRONMENTS` | `langfuse/config.rs:277` | `vire` | active default |
| `VIRE_LANGFUSE_PUBLIC_KEY` | `langfuse/config.rs:151` | — | empty placeholder |
| `VIRE_LANGFUSE_SECRET_KEY` | `langfuse/config.rs:153` | — | empty placeholder |
| `VIRE_RUNTIME_LOG_PATH` | `runtime_observer/config.rs:67` | derived from state dir | commented optional |
| `PI_OBSERVE_STATE_DIR` | `runtime_observer/config.rs:70` | `~/.local/state/pi-observe` | commented optional |
| `VIRE_RUNTIME_ENV_MAP` | `runtime_observer/config.rs:37` | empty map | commented optional |
| `VIRE_RUNTIME_MATCH_SLOP_SECS` | `runtime_observer/config.rs:39` | `300` | commented optional |

---

## 2. Secret hygiene

- All credential fields (`VIRE_LANGFUSE_PUBLIC_KEY=`, `VIRE_LANGFUSE_SECRET_KEY=`) are explicitly
  empty.
- No token pattern (`github_pat_*`, `ghp_*`, `sk-ant-*`, `AKIA*`, etc.) present in the template.
- Cloud override block is commented out — inactive by default.

---

## 3. Gitignore verification

`git check-ignore -v` results:

| Path | Rule matched | Outcome |
|---|---|---|
| `.env` | `.gitignore:9` — `/.env` | **Ignored** (root-anchored) ✓ |
| `.env.example` | `.gitignore:11` — `!/.env.example` | **Not ignored** (negation, tracked) ✓ |
| `observability/langfuse/.env.example` | No rule matched | **Not ignored** (existing tracked file, unaffected) ✓ |

---

## 4. DEC-020 / NFR-001 posture preserved

| Criterion | Location | Status |
|---|---|---|
| Local Docker Langfuse on loopback is the default | `.env.example` active `VIRE_LANGFUSE_BASE_URL=http://127.0.0.1:3000` + `VIRE_LANGFUSE_SOURCE=local` | **PASS** |
| Cloud is commented-out non-default override only | `.env.example` Cloud block commented; README note restates this | **PASS** |
| No secrets committed | Credential fields empty; gitignore guards root `.env` | **PASS** |
| App-env / Docker-stack-env distinction documented | README "Two env files" table | **PASS** |

---

## 5. No-auto-load constraint honored (arch-review §3)

The README "App runtime configuration (env)" section states: "Vire reads its runtime configuration
from **process environment variables**. It does **not** auto-load a `.env` file." The apply step
(`set -a; . ./.env; set +a`) is shown prominently before `npm run tauri:dev`. No false auto-load
claim is made anywhere in the documentation.

---

## 6. OpenSpec validation

```
openspec validate task-025-root-app-env-example --strict
→ Change 'task-025-root-app-env-example' is valid
```

---

## 7. git status check

`git status --short` after all edits:

```
M .gitignore
 M README.md
?? .env.example
?? openspec/changes/task-025-root-app-env-example/
```

No `.env` (real) file present; no Rust/TypeScript/config source files modified.

---

## Verdict

**PASS.** All three deliverables are in place (`/.env.example`, updated `/.gitignore`, updated
`README.md`). Env-var names match Rust source verbatim. No secrets in the template. Gitignore
guards verified. DEC-020 posture preserved. No-auto-load constraint documented correctly. OpenSpec
validates clean.

Non-blocking: DEC-025 canonical log entry is routed to BA flow (see `arch-review.md` §7 `FB-025-1`);
SW write-scope does not cover `artifacts/ba/07_decision_log.md`.
