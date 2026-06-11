# Spec delta — langfuse-trace-source

## ADDED Requirements

### Requirement: The default AI trace source is local Docker self-hosted Langfuse

The TASK-007 Langfuse importer SHALL treat local Docker self-hosted Langfuse as Vire's
canonical/default AI trace source, with Langfuse Cloud demoted to an explicit, non-default override.
This realigns the downstream importer with DEC-020 (technical-plan alignment DEC-022), which
supersedes the earlier DEC-018 cloud-first posture. The importer SHALL default to a loopback
(`127.0.0.1`) endpoint, SHALL never interpret a down/unreachable local stack as zero AI usage or
cost, and SHALL keep the local multi-component stack's MinIO/S3 and backup-consistency risks
documented for downstream owners.

#### Scenario: Local Docker Langfuse is the default and Cloud is an explicit override

- **WHEN** the TASK-007 importer resolves its trace source
- **THEN** local Docker self-hosted Langfuse is selected as the default source with no extra
  configuration
- **AND** Langfuse Cloud is reachable only as an explicit, operator-set non-default override and is
  never the implicit default.

#### Scenario: The importer targets loopback by default

- **WHEN** the default local source is used
- **THEN** the importer's base URL defaults to a loopback/localhost endpoint (`http://127.0.0.1:3000`)
- **AND** the local stack is documented as bound to `127.0.0.1` with no LAN/host exposure recommended
  for its internal services.

#### Scenario: A down or unreachable stack is never read as zero usage or cost

- **WHEN** Docker or the local Langfuse stack is down, unreachable, or returns no traces
- **THEN** the importer treats the result as unavailable / stale / unknown — an evidence gap
- **AND** it SHALL NOT interpret the absence of traces as zero AI usage or zero cost.

#### Scenario: Local-stack MinIO/S3 and backup risks stay documented

- **WHEN** the local Docker stack is the default source
- **THEN** its multi-component nature (Postgres + ClickHouse + MinIO/S3 object storage + Redis) and
  the associated MinIO/S3 object-storage and three-store backup/restore-consistency risks are
  documented for the TASK-007 MVP owner
- **AND** the MinIO bucket is documented as internal/private (not host-published, not public) so the
  default posture is not silently weakened downstream.
