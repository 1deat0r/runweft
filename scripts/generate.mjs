import { spawnSync } from 'node:child_process';

const check = process.argv.includes('--check');
const scripts = [
  'scripts/generate-status.mjs',
  'scripts/generate-protocol.mjs',
  'scripts/generate-typescript-types.mjs',
];
for (const script of scripts) {
  const result = spawnSync(process.execPath, [script, ...(check ? ['--check'] : [])], { stdio: 'inherit' });
  if (result.error || result.status !== 0) {
    console.error(result.error ?? `${script} exited with status ${result.status}`);
    process.exit(1);
  }
}
