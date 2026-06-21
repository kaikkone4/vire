# Spec delta — app-configuration

## ADDED Requirements

### Requirement: A secret-safe root env template documents the Vire app's runtime configuration

The repository SHALL provide a tracked root `.env.example` that lists the Vire **desktop-app** runtime
environment variables a local tester must set, distinct from the Docker-stack env at
`observability/langfuse/.env.example`. The template SHALL include the variables the app actually reads —
`VIRE_LANGFUSE_BASE_URL`, `VIRE_LANGFUSE_SOURCE`, `VIRE_LANGFUSE_ENVIRONMENTS`,
`VIRE_LANGFUSE_PUBLIC_KEY`, `VIRE_LANGFUSE_SECRET_KEY`, and (optional, commented) `VIRE_RUNTIME_LOG_PATH`
/ `VIRE_RUNTIME_ENV_MAP` / `VIRE_RUNTIME_MATCH_SLOP_SECS` — with the same defaults the code applies. The
template SHALL contain **no real secrets**: every credential field SHALL be empty or a placeholder.

#### Scenario: Template lists app vars with safe defaults

- **WHEN** a local tester opens the root `.env.example`
- **THEN** it lists the `VIRE_*` app runtime variables with their code defaults
  (`VIRE_LANGFUSE_BASE_URL=http://127.0.0.1:3000`, `VIRE_LANGFUSE_SOURCE=local`,
  `VIRE_LANGFUSE_ENVIRONMENTS=vire`)
- **AND** the credential fields (`VIRE_LANGFUSE_PUBLIC_KEY`, `VIRE_LANGFUSE_SECRET_KEY`) are empty
  placeholders with guidance to fill them locally and never commit them.

#### Scenario: No secret is committable at the repo root

- **WHEN** the repository is checked for a committed root `.env`
- **THEN** `.gitignore` ignores a root `.env` and root `.env.*` variants
- **AND** the root `.env.example` remains tracked
- **AND** the Docker-stack `observability/langfuse/.env.example` remains tracked and unaffected.

### Requirement: Documentation states the app reads process env and how to apply the file

The documentation SHALL state that the app loads configuration from process environment variables and
does **not** auto-load a `.env` file, and SHALL give the apply step (`set -a; . ./.env; set +a`) to be
run in the shell that launches the app. The documentation SHALL NOT claim the app reads the `.env` file
automatically.

#### Scenario: Apply step is documented, no false auto-load claim

- **WHEN** a local tester reads the README app-configuration section
- **THEN** it states the app reads process environment variables
- **AND** it shows how to export the root `.env` into the launching shell before running the app
- **AND** it does not claim the app loads `.env` automatically.

### Requirement: App env and Docker-stack env are documented as separate concerns

The documentation SHALL distinguish the **root app `.env`** (configures the Vire desktop app via
`VIRE_*` variables) from the **`observability/langfuse/.env`** (configures the Langfuse Docker server and
its backing services), so a tester knows which file to edit for which purpose.

#### Scenario: Reader can tell the two env files apart

- **WHEN** a local tester reads the README
- **THEN** it explains that the root `.env` holds Vire app settings (Langfuse base URL/source/keys for
  the app to query)
- **AND** that `observability/langfuse/.env` holds the Langfuse server stack's secrets and bootstrap
- **AND** the two are not merged or interchangeable.

### Requirement: Local Docker Langfuse stays the default; Cloud is an explicit override

The template and documentation SHALL preserve DEC-020: local Docker self-hosted Langfuse on loopback
(`http://127.0.0.1:3000`, `source=local`) is the default, and Langfuse Cloud is presented only as an
explicit, non-default, commented-out override that the operator must deliberately enable.

#### Scenario: Default points at local loopback, Cloud is opt-in only

- **WHEN** the root `.env.example` is copied to `.env` without edits to the source/base-url lines
- **THEN** the configured target is the local loopback Langfuse with `source=local`
- **AND** the Cloud override (`source=cloud` with an off-host base URL) is present only as a commented
  example that is inactive until uncommented.
