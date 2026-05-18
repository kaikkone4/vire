import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const tasksPath = new URL('../observability/pi-observe/examples/vscode/tasks.json', import.meta.url).pathname;
const aliasesPath = new URL('../observability/pi-observe/examples/shell/aliases.sh', import.meta.url).pathname;

test('VS Code task examples wrap commands through pi-observe with explicit project', () => {
  const tasks = JSON.parse(readFileSync(tasksPath, 'utf8'));
  assert.ok(tasks.tasks.length >= 1);
  for (const task of tasks.tasks) {
    assert.match(task.command, /^pi-observe run /);
    assert.match(task.command, /--project vire/);
    assert.match(task.command, / -- /);
  }
});

test('editor launcher aliases are nonbillable context by default', () => {
  const aliases = readFileSync(aliasesPath, 'utf8');
  assert.match(aliases, /cursor-observed=.*--nonbillable/);
  assert.match(aliases, /code-observed=.*--nonbillable/);
  assert.match(aliases, /antigravity-observed=.*--nonbillable/);
});
