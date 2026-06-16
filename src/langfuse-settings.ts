// Pure helpers for the in-app Langfuse settings panel (TASK-026 A4). No DOM or IPC here so the
// logic stays unit-testable; the secret value is never an input or output of any of these.

export function environmentsToCsv(environments: string[]): string {
  return environments.join(', ');
}

export function parseEnvironmentsCsv(csv: string): string[] {
  return csv
    .split(',')
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
}

// Presence flag → label. We only ever know whether a key is set, never its value (SEC-009).
export function secretStateLabel(present: boolean): string {
  return present ? 'set' : 'not set';
}
