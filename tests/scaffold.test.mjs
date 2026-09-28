import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { SCAFFOLD_STATUS } from '../packages/protocol/dist/index.js';
import { exampleProvider } from '../packages/provider-example/dist/index.js';

function cargo(...args) {
  return spawnSync('cargo', ['run', '--locked', '--quiet', ...args], { encoding: 'utf8', timeout: 120000 });
}
test('Rust status exactly matches the generated TypeScript contract', () => {
  const result = cargo('-p', 'runweft-cli', '--', 'status', '--json');
  assert.equal(result.status, 0, result.stderr || String(result.error));
  assert.deepEqual(JSON.parse(result.stdout), SCAFFOLD_STATUS);
});
test('the scaffold rejects execution and daemon startup', () => {
  for (const args of [['-p', 'runweft-cli', '--', 'run', 'example'], ['-p', 'runweft-daemon']]) {
    const result = cargo(...args);
    assert.equal(result.status, 2, result.stderr || String(result.error));
    assert.equal(result.stdout, '');
  }
  assert.equal(exampleProvider.supportsExecution, false);
});
