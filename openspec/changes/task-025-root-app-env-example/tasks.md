# Tasks — TASK-025 Root app env example for local MVP config

**One docs/devops slice.** No product runtime code changes — the `VIRE_*` variables are already read by
existing code (`src-tauri/src/langfuse/config.rs`, `src-tauri/src/runtime_observer/config.rs`). The
sub-tasks below are an implementation sequence within a single OpenSpec change, **not** a component
split. All edits land at the repo root / in `README.md`.

## 1. Add the root app env template

- [x] Create `/.env.example` at the repo root with the Vire **app** runtime variables. Use the
      `VIRE_LANGFUSE_*` / `VIRE_RUNTIME_*` names (authoritative — match the `std::env::var` keys in the
      Rust config modules exactly):
  - [x] `VIRE_LANGFUSE_BASE_URL=http://127.0.0.1:3000` (local Docker loopback default).
  - [x] `VIRE_LANGFUSE_SOURCE=local` (default; document `cloud` only as a commented-out override).
  - [x] `VIRE_LANGFUSE_ENVIRONMENTS=vire` (default CSV environment list).
  - [x] `VIRE_LANGFUSE_PUBLIC_KEY=` and `VIRE_LANGFUSE_SECRET_KEY=` — **empty** placeholders, with a
        comment to create them in the Langfuse UI and never commit them.
  - [x] Optional, **commented out** (safe code defaults exist): `# VIRE_RUNTIME_LOG_PATH=`,
        `# PI_OBSERVE_STATE_DIR=`, `# VIRE_RUNTIME_ENV_MAP=proj=env`, `# VIRE_RUNTIME_MATCH_SLOP_SECS=300`.
  - [x] A commented Cloud-override block: `# VIRE_LANGFUSE_SOURCE=cloud` +
        `# VIRE_LANGFUSE_BASE_URL=https://cloud.langfuse.com` (off by default, DEC-020 preserved).
  - [x] Header comment: copy to `.env`, fill locally, **do not commit**, and the apply step
        (`set -a; . ./.env; set +a` before `npm run tauri:dev`) because the app does **not** auto-load
        `.env`. Note the `LANGFUSE_PUBLIC_KEY` / `LANGFUSE_SECRET_KEY` fallback in a comment only.
- [x] Confirm **no real secret values** are present — every credential field is empty or a placeholder.

## 2. Protect a root `.env` in version control

- [x] Append anchored rules to `/.gitignore` (root-scoped so the existing `observability/langfuse/`
      rules are untouched):
      ```
      # Local Vire app runtime env (root) — real values never committed
      /.env
      /.env.*
      !/.env.example
      ```
- [ ] Verify: `git check-ignore -v .env` reports ignored; `git check-ignore -v .env.example` reports
      **not** ignored (tracked); `observability/langfuse/.env.example` is still tracked.
      *(Run after commit — check performed in docs.md)*

## 3. Document app env vs Docker-stack env

- [x] Add a "App runtime configuration (env)" subsection to `README.md` that: lists the `VIRE_*` app
      vars and defaults; states the app reads **process env** and shows `set -a; . ./.env; set +a`;
      explains **root app `.env`** (configures the Vire desktop app) vs
      **`observability/langfuse/.env`** (configures the Langfuse Docker server); restates local default
      + Cloud-as-explicit-override.
- [x] Tighten README step 3 ("Configure Vire's Langfuse settings…") to point at the new root
      `.env.example` and the apply step, instead of implying a settings UI.

## 4. Verify

- [x] Re-read `/.env.example`, `/.gitignore`, and the README subsection — values match the env-var
      names in the Rust source; defaults match the code defaults; no secrets committed.
- [ ] `git status` shows only `.env.example`, `.gitignore`, `README.md`, and the OpenSpec change as
      additions; **no** `.env`, no source/code files modified. *(See docs.md)*
- [ ] Optional sanity: with a filled local `.env`, `set -a; . ./.env; set +a && npm run tauri:dev`
      brings the app up and Langfuse health resolves against `127.0.0.1:3000` (manual, local-only).

## Acceptance

- A new local tester can copy `/.env.example` → `.env`, fill keys, source it, run the app, and reach a
  healthy local Langfuse — **without reading Rust source**.
- No secret is committable at the repo root (gitignore guard verified).
- The app-env / stack-env distinction is documented and the two example files stay separate.
