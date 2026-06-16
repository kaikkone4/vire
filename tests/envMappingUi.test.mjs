import test from 'node:test';
import assert from 'node:assert/strict';
import {
  DEFAULT_ENVIRONMENT,
  envPickerOptions,
  envPickerCheckboxes,
  mergeSelectedEnvironments,
  mappingRow,
  mappingPanel,
} from '../src/env-mapping-ui.ts';

// ---- C4: environment picker -------------------------------------------------

test('picker always offers the default environment, dedupes, trims, and sorts', () => {
  assert.deepEqual(envPickerOptions([], []), [DEFAULT_ENVIRONMENT]);
  assert.deepEqual(
    envPickerOptions(['staging', ' prod ', 'vire'], ['prod', 'dev']),
    ['dev', 'prod', 'staging', 'vire'],
  );
});

test('a configured-but-undiscovered environment still appears as a ticked box', () => {
  const html = envPickerCheckboxes(['vire'], ['vire', 'staging']);
  // staging is configured (selected) though not discovered → shown and checked.
  assert.match(html, /value="staging"[^>]*checked/);
  assert.match(html, /value="vire"[^>]*checked/);
});

test('an unselected discovered environment renders unchecked', () => {
  const html = envPickerCheckboxes(['prod'], ['vire']);
  assert.match(html, /value="prod"(?![^>]*checked)/);
  assert.match(html, /value="vire"[^>]*checked/);
});

test('saving unions ticked boxes with advanced CSV entries, deduped and order-preserving', () => {
  assert.deepEqual(mergeSelectedEnvironments(['vire', 'staging'], ''), ['vire', 'staging']);
  assert.deepEqual(mergeSelectedEnvironments(['vire'], 'prod, qa'), ['vire', 'prod', 'qa']);
  // CSV duplicate of a ticked box collapses; blanks dropped.
  assert.deepEqual(mergeSelectedEnvironments(['vire'], ' vire , , prod '), ['vire', 'prod']);
});

test('unticking everything yields an empty list — the core applies the vire default, not the UI', () => {
  assert.deepEqual(mergeSelectedEnvironments([], ''), []);
});

// ---- D4: environment → project mapping --------------------------------------

const mappedEnv = {
  environment: 'vire',
  last_seen: '2026-06-16 10:00:00',
  mapped: true,
  project_id: 'p1',
  project_name: 'Vire Core',
};
const unmappedEnv = {
  environment: 'staging',
  last_seen: '2026-06-16 10:00:00',
  mapped: false,
  project_id: null,
  project_name: null,
};
const projects = [
  { id: 'p1', name: 'Vire Core', archived: false },
  { id: 'p2', name: 'Old Work', archived: true },
];

test('a mapped environment shows its project and a clear action, no project picker', () => {
  const html = mappingRow(mappedEnv, projects);
  assert.match(html, /Mapped → <b>Vire Core<\/b>/);
  assert.match(html, /data-clear-map="vire"/);
  assert.doesNotMatch(html, /data-map-set/);
  assert.doesNotMatch(html, /data-create-map/);
});

test('an unmapped environment offers a project picker AND an explicit create-and-map action', () => {
  const html = mappingRow(unmappedEnv, projects);
  assert.match(html, /data-map-select="staging"/);
  assert.match(html, /data-map-set="staging"/);
  assert.match(html, /data-create-map="staging"/);
  assert.match(html, /Create project for staging/);
  // existing projects are selectable, archived flagged
  assert.match(html, /value="p1"/);
  assert.match(html, /Old Work \(archived\)/);
});

test('unmapped with no projects still offers create-and-map but no empty picker', () => {
  const html = mappingRow(unmappedEnv, []);
  assert.match(html, /No projects yet/);
  assert.match(html, /data-create-map="staging"/);
  assert.doesNotMatch(html, /data-map-set/);
});

test('panel explains the empty state instead of rendering blank', () => {
  const html = mappingPanel([], projects);
  assert.match(html, /No environments discovered yet/);
  assert.match(html, /Run an import/i);
});

test('mapping surfaces never leak a secret-shaped token', () => {
  const html = mappingPanel([mappedEnv, unmappedEnv], projects);
  for (const needle of ['sk-', 'pk-', 'Bearer', 'Authorization']) {
    assert.ok(!html.includes(needle), `mapping panel must not contain ${needle}`);
  }
});
