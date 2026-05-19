import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const composePath = new URL('../observability/langfuse/docker-compose.yml', import.meta.url).pathname;

function extractServiceBlock(compose, serviceName) {
  const serviceHeader = new RegExp(`^  ${serviceName}:\\n`, 'm');
  const match = serviceHeader.exec(compose);
  assert.ok(match, `${serviceName} service should exist`);
  const start = match.index;
  const next = /^  [a-zA-Z0-9_-]+:\n/gm;
  next.lastIndex = start + match[0].length;
  const nextMatch = next.exec(compose);
  return compose.slice(start, nextMatch?.index ?? compose.length);
}

test('ClickHouse healthcheck is scoped to the clickhouse service and uses bundled authenticated client query', () => {
  const compose = readFileSync(composePath, 'utf8');
  const clickhouse = extractServiceBlock(compose, 'clickhouse');

  assert.match(clickhouse, /healthcheck:/, 'clickhouse service should define a healthcheck');
  assert.match(clickhouse, /test:\s*\["CMD-SHELL",\s*"clickhouse-client --host localhost/, 'healthcheck should use clickhouse-client from the image');
  assert.match(clickhouse, /--user \\\"\$\$\{CLICKHOUSE_USER\}\\\"/, 'healthcheck should use the configured ClickHouse user');
  assert.match(clickhouse, /--password \\\"\$\$\{CLICKHOUSE_PASSWORD\}\\\"/, 'healthcheck should authenticate with the configured ClickHouse password');
  assert.match(clickhouse, /--query 'SELECT 1' >\/dev\/null/, 'healthcheck should verify a query succeeds, not only that a port responds');
  const healthcheckLine = clickhouse.split('\n').find((line) => line.includes('test:')) ?? '';
  assert.doesNotMatch(healthcheckLine, /\b(wget|curl|nc)\b/, 'clickhouse healthcheck command must not depend on network tools missing from Alpine images');
});

test('Langfuse services wait for ClickHouse to become healthy', () => {
  const compose = readFileSync(composePath, 'utf8');
  for (const serviceName of ['langfuse-worker', 'langfuse-web']) {
    const block = extractServiceBlock(compose, serviceName);
    assert.match(block, /clickhouse:\n\s+condition: service_healthy/, `${serviceName} should wait for ClickHouse health`);
  }
});
