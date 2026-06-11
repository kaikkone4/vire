# Backup and Restore — Local Langfuse Docker Stack

This document covers backup scope, consistency requirements, restore procedure, divergence failure modes, and the post-restore verification checklist for the local Langfuse Docker stack that Vire uses as its primary AI trace source.

This is an L2 operations document. Completing a restore drill before broad use is strongly recommended.

## What must be backed up

A valid Langfuse backup requires **all three persistent stores to be consistent with each other**. Backing up only one is insufficient and will cause divergence on restore.

| Component | Docker volume | What it holds |
|---|---|---|
| PostgreSQL | `langfuse_postgres_data` | Projects, users, auth tokens, Langfuse application state, trace metadata |
| ClickHouse | `langfuse_clickhouse_data`, `langfuse_clickhouse_logs` | Trace events, observation spans, usage/cost records, analytics data |
| MinIO | `langfuse_minio_data` | Ingested event objects, media files, batch export objects (`events/`, `media/`, `exports/` prefixes in the `langfuse` bucket) |
| Redis / Valkey | `langfuse_redis_data` | Worker queue state and cache |

Redis/Valkey holds transient queue state. Full point-in-time consistency with the other three stores is not always achievable for the queue/cache tier. The practical approach is to stop the Langfuse worker before backup so the queue is drained, then restore Redis last. Incomplete queue recovery may cause some in-flight ingestion jobs to be lost on restore; those trace events would need to be re-ingested or accepted as a gap.

## Backup procedure

### Recommended approach: stop containers first

The safest backup method is a cold backup after stopping the stack. This guarantees consistency between PostgreSQL, ClickHouse, and MinIO.

```sh
# Stop the stack (drains in-flight workers)
docker compose down

# Back up PostgreSQL
docker run --rm -v langfuse_postgres_data:/data -v "$(pwd)/backup":/backup \
  alpine tar czf /backup/postgres_$(date +%Y%m%d_%H%M%S).tar.gz -C /data .

# Back up ClickHouse data and logs
docker run --rm -v langfuse_clickhouse_data:/data -v "$(pwd)/backup":/backup \
  alpine tar czf /backup/clickhouse_data_$(date +%Y%m%d_%H%M%S).tar.gz -C /data .

docker run --rm -v langfuse_clickhouse_logs:/logs -v "$(pwd)/backup":/backup \
  alpine tar czf /backup/clickhouse_logs_$(date +%Y%m%d_%H%M%S).tar.gz -C /logs .

# Back up MinIO object storage
docker run --rm -v langfuse_minio_data:/data -v "$(pwd)/backup":/backup \
  alpine tar czf /backup/minio_$(date +%Y%m%d_%H%M%S).tar.gz -C /data .

# Optionally back up Redis/Valkey
docker run --rm -v langfuse_redis_data:/data -v "$(pwd)/backup":/backup \
  alpine tar czf /backup/redis_$(date +%Y%m%d_%H%M%S).tar.gz -C /data .

# Restart the stack
docker compose up -d
```

Replace `"$(pwd)/backup"` with a stable local backup destination. All backup archives should be stored in a location accessible outside the Docker volume set, such as an external drive or Time Machine path.

### Alternative: online PostgreSQL dump

If stopping the stack is not acceptable, you can take a `pg_dump` while the stack is running:

```sh
docker compose exec postgres pg_dump -U <pg-user> <pg-db> > backup/langfuse_pg_$(date +%Y%m%d_%H%M%S).sql
```

However, an online PostgreSQL dump taken while ClickHouse and MinIO are actively writing will not be consistent with those stores. Use this approach only for PostgreSQL-only recovery scenarios, not for a full consistent restore.

## Backup security

- Backup archives contain Langfuse application data including API key hashes, user records, and trace metadata. Store them securely and apply the same access controls as the live stack.
- Do not include MinIO access keys, PostgreSQL passwords, or any other credentials in backup file names, metadata, or README notes.
- Do not upload backup archives to untrusted storage or commit them to the repo.

## Restore procedure

### Pre-restore checklist

- [ ] Identify the backup timestamp you are restoring from. All three archives (PostgreSQL, ClickHouse, MinIO) must come from the **same backup set** (same timestamp).
- [ ] Stop the running stack: `docker compose down`.
- [ ] Remove existing volumes if performing a full restore (this is destructive — confirm before proceeding):
  ```sh
  docker volume rm langfuse_postgres_data langfuse_clickhouse_data \
    langfuse_clickhouse_logs langfuse_minio_data langfuse_redis_data
  ```
- [ ] Mark Vire's AI trace source status as `unknown` in Vire settings before starting the stack. Do not trust AI totals until the post-restore verification checklist passes.

### Restore steps

Restore in this order: PostgreSQL → ClickHouse → MinIO → Redis/Valkey.

```sh
# Restore PostgreSQL
docker run --rm -v langfuse_postgres_data:/data -v "$(pwd)/backup":/backup \
  alpine sh -c "cd /data && tar xzf /backup/postgres_<timestamp>.tar.gz"

# Restore ClickHouse
docker run --rm -v langfuse_clickhouse_data:/data -v "$(pwd)/backup":/backup \
  alpine sh -c "cd /data && tar xzf /backup/clickhouse_data_<timestamp>.tar.gz"

docker run --rm -v langfuse_clickhouse_logs:/logs -v "$(pwd)/backup":/backup \
  alpine sh -c "cd /logs && tar xzf /backup/clickhouse_logs_<timestamp>.tar.gz"

# Restore MinIO
docker run --rm -v langfuse_minio_data:/data -v "$(pwd)/backup":/backup \
  alpine sh -c "cd /data && tar xzf /backup/minio_<timestamp>.tar.gz"

# Restore Redis/Valkey (optional — can be left empty for a fresh queue state)
docker run --rm -v langfuse_redis_data:/data -v "$(pwd)/backup":/backup \
  alpine sh -c "cd /data && tar xzf /backup/redis_<timestamp>.tar.gz"

# Start the stack
docker compose up -d
```

Replace `<timestamp>` with the actual backup timestamp from the matching set.

## Divergence failure modes

If PostgreSQL, ClickHouse, and MinIO are not restored from the same consistent backup, the following failures can occur:

| Scenario | Symptoms | Impact |
|---|---|---|
| PostgreSQL restored from an older backup than ClickHouse | ClickHouse contains trace events for projects or generations that no longer exist in PostgreSQL auth/project tables | Vire may show trace events that cannot be mapped to a project; API may return 404 or auth errors for some traces |
| ClickHouse restored from an older backup than PostgreSQL | PostgreSQL references trace IDs or generations not present in ClickHouse | Trace queries return empty or incomplete results; AI usage/cost totals are understated |
| MinIO restored from an older backup | PostgreSQL/ClickHouse reference media/event objects that do not exist in object storage | Media display fails; batch export requests fail; some trace events may be unreachable |
| MinIO newer than PostgreSQL/ClickHouse | Object storage contains objects with no corresponding metadata records | Orphaned objects; no direct failure but wasted storage and confusing audit trail |
| Stores restored from different points in time | Mixed state across all three | Inconsistent totals across the review UI; some traces fully visible, some partially visible, some missing; cannot be diagnosed without re-backup |
| Incomplete queue/cache recovery | Redis/Valkey restored to state mid-queue-drain | Worker may replay or drop in-flight ingestion jobs; some trace events may be duplicated or lost |

### Signs of divergence after restore

- Vire shows trace events for environments or projects that do not exist.
- Trace counts in Langfuse UI differ significantly from pre-backup counts.
- ClickHouse queries succeed but return no data for date ranges that were populated before backup.
- MinIO/S3 errors appear in Langfuse worker logs (`No such key`, `Bucket not found`, etc.).
- Vire import health reports schema or mapping errors that were not present before.

## Post-restore verification checklist

Complete all checks before removing the `unknown` AI trace source status in Vire.

- [ ] Stack is fully up: `docker compose ps` shows all services healthy.
- [ ] Langfuse UI is reachable at `http://127.0.0.1:3000`.
- [ ] Can log in to Langfuse UI and see existing projects and environments.
- [ ] Trace count for a known date range matches pre-backup expectation (or is acceptably close to the backup timestamp).
- [ ] Langfuse worker logs show no critical errors after first startup post-restore.
- [ ] MinIO API is reachable (via MinIO console at `http://127.0.0.1:<console-port>` or `mc admin info`); `langfuse` bucket exists and is not public.
- [ ] Vire Langfuse import runs successfully for at least one environment and returns traces.
- [ ] Vire AI trace health status transitions from `unknown` to `healthy` or `stale` (not `unavailable`).
- [ ] Spot-check two or three known trace IDs: confirm they are visible in Langfuse UI and their usage/cost fields are populated.

If any check fails, do not rely on AI totals in Vire. Investigate the failed component and, if needed, perform a full restore from a known-good backup set.

## Backup frequency guidance

For active use, take consistent cold backups at least:
- Before upgrading the Langfuse Docker images.
- Before any significant Vire schema or project-mapping changes.
- On a regular schedule (weekly at minimum for active daily use).

Store at least two backup sets at any time so that if the most recent set is corrupt, a fallback is available.
