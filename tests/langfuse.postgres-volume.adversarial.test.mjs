import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const upScript = readFileSync(new URL('../scripts/langfuse-up.sh', import.meta.url), 'utf8');

test('Postgres credential guard validates .env password, not stale container environment', () => {
  assert.doesNotMatch(
    upScript,
    /exec\s+-T\s+postgres\s+sh\s+-c\s+'PGPASSWORD="\$POSTGRES_PASSWORD"/,
    'docker compose exec runs inside an existing postgres container; using $POSTGRES_PASSWORD there can validate the stale container env instead of the current .env password'
  );
  assert.match(
    upScript,
    /pg_password="\$\(env_value POSTGRES_PASSWORD\)"[\s\S]*PGPASSWORD="\$pg_password"[\s\\\n]*docker compose --env-file \.env exec -T -e PGUSER -e PGDATABASE -e PGPASSWORD postgres/,
    'guard should pass the POSTGRES_PASSWORD read from observability/langfuse/.env into psql, so already-running containers with old env cannot mask password drift'
  );
});
