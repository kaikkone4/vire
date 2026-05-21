import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const upScript = readFileSync(new URL('../scripts/langfuse-up.sh', import.meta.url), 'utf8');
const setupScript = readFileSync(new URL('../scripts/setup-local-observability.sh', import.meta.url), 'utf8');
const readme = readFileSync(new URL('../observability/langfuse/README.md', import.meta.url), 'utf8');

test('langfuse-up verifies existing Postgres volume credentials before starting full stack', () => {
  assert.match(upScript, /postgres_volume_exists\(\)/, 'up script should detect an existing Postgres named volume');
  assert.match(upScript, /vire-local-langfuse_langfuse_postgres/, 'up script should check the Compose-prefixed volume name');
  assert.match(upScript, /PGPASSWORD="\$POSTGRES_PASSWORD" psql -U "\$POSTGRES_USER" -d "\$POSTGRES_DB"/, 'up script should test real password authentication without printing secrets');
  assert.match(upScript, /Prisma P1000|Postgres rejected the credentials/, 'up script should explain the auth failure');
  assert.match(upScript, /\.\/scripts\/langfuse-down\.sh -v/, 'up script should provide the local reset command');
});

test('setup warns when it generates POSTGRES_PASSWORD after a Postgres volume already exists', () => {
  assert.match(setupScript, /generated_postgres_password=false/, 'setup should track whether it generated a new Postgres password');
  assert.match(setupScript, /generated_postgres_password" == "true".*postgres_volume_exists/s, 'setup should warn only when a new password coincides with an existing volume');
  assert.match(setupScript, /Postgres only applies POSTGRES_PASSWORD when its data directory is first initialized/, 'setup warning should state why the mismatch happens');
});

test('README documents Prisma P1000 password and volume mismatch remediation', () => {
  assert.match(readme, /Prisma `P1000` \/ Postgres authentication failed/, 'README should have a targeted troubleshooting section');
  assert.match(readme, /`pg_isready` checks readiness, not password authentication/, 'README should explain why healthcheck can pass');
  assert.match(readme, /\.\/scripts\/langfuse-down\.sh -v/, 'README should document destructive reset command');
  assert.match(readme, /do \*\*not\*\* delete volumes/i, 'README should warn users with important data not to delete volumes');
});
