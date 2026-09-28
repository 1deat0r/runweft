import { spawnSync } from 'node:child_process';
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const steps = [
  ['node', ['scripts/generate-protocol.mjs', '--check']],
  ['cargo', ['fmt', '--all', '--check']],
  ['cargo', ['clippy', '--locked', '--workspace', '--all-targets', '--', '-D', 'warnings']],
  ['cargo', ['test', '--locked', '--workspace']],
  [npm, ['run', 'build']],
  [npm, ['test']],
  ['python3', ['-B', 'evals/analyze_evaluation.py', '--self-test']],
];
for (const [command, args] of steps) {
  console.log(`\n> ${command} ${args.join(' ')}`);
  const result = spawnSync(command, args, { stdio: 'inherit', timeout: 300000 });
  if (result.error || result.status !== 0) { console.error(result.error ?? `exit ${result.status}`); process.exit(1); }
}
