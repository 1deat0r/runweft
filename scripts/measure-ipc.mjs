// Local-only framing/validation probe. This is a test prototype, not a production listener.
import { createServer, connect } from 'node:net';
import { unlink } from 'node:fs/promises';
import { cpus, platform, release, tmpdir, arch } from 'node:os';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { validateWireMessage } from '../packages/protocol/dist/index.js';

const socketPath = path.join(tmpdir(), `runweft-ipc-${process.pid}-${randomUUID().slice(0, 8)}.sock`);
const warmups = Number(process.env.RUNWEFT_IPC_WARMUPS ?? 100);
const iterations = Number(process.env.RUNWEFT_IPC_ITERATIONS ?? 500);
if (!Number.isSafeInteger(warmups) || warmups < 1 || !Number.isSafeInteger(iterations) || iterations < 10) {
  throw new Error('RUNWEFT_IPC_WARMUPS must be >= 1 and RUNWEFT_IPC_ITERATIONS must be >= 10');
}

const authority = {
  peer: { role: 'user', id: 'ipc-measurement-client' },
  profile_id: 'profile-measurement',
  project_id: 'project-measurement',
  profile_incarnation: 'incarnation-measurement',
  coordinator_generation: '1',
};
const baseMessage = {
  protocol_version: '1.0',
  kind: 'command',
  variant: 'run.submit',
  command_id: 'ipc-measurement-command',
  profile_id: authority.profile_id,
  project_id: authority.project_id,
  profile_incarnation: authority.profile_incarnation,
  coordinator_generation: authority.coordinator_generation,
  payload: {
    run_id: 'run-measurement',
    graph_revision_id: 'graph-measurement',
    graph_digest: 'b'.repeat(64),
    requested_budget_micro_usd: '0',
  },
};

const frame = (bytes) => {
  const result = Buffer.allocUnsafe(4 + bytes.byteLength);
  result.writeUInt32BE(bytes.byteLength, 0);
  result.set(bytes, 4);
  return result;
};

function createProbeServer() {
  const server = createServer((socket) => {
    let incoming = Buffer.alloc(0);
    let work = Promise.resolve();
    socket.on('data', (chunk) => {
      incoming = Buffer.concat([incoming, chunk]);
      while (incoming.byteLength >= 4) {
        const length = incoming.readUInt32BE(0);
        if (length > 1_048_576) { socket.destroy(new Error('probe frame exceeded contract bound')); return; }
        if (incoming.byteLength < 4 + length) break;
        const request = incoming.subarray(4, 4 + length);
        incoming = incoming.subarray(4 + length);
        work = work.then(async () => {
          const result = await validateWireMessage(request, authority);
          socket.write(frame(Buffer.from(JSON.stringify(result), 'utf8')));
        }).catch((error) => socket.destroy(error));
      }
    });
  });
  return server;
}

async function connectLocal(server) {
  const socket = connect(socketPath);
  await new Promise((resolve, reject) => {
    socket.once('connect', resolve);
    socket.once('error', reject);
  });
  return socket;
}

async function readFrame(socket) {
  let incoming = Buffer.alloc(0);
  return new Promise((resolve, reject) => {
    const cleanup = () => {
      socket.off('data', onData);
      socket.off('error', onError);
    };
    const onError = (error) => { cleanup(); reject(error); };
    const onData = (chunk) => {
      incoming = Buffer.concat([incoming, chunk]);
      if (incoming.byteLength < 4) return;
      const length = incoming.readUInt32BE(0);
      if (incoming.byteLength < 4 + length) return;
      const response = incoming.subarray(4, 4 + length);
      cleanup();
      resolve(JSON.parse(response.toString('utf8')));
    };
    socket.on('data', onData);
    socket.once('error', onError);
  });
}

async function roundTrip(socket, requestBytes) {
  const responsePromise = readFrame(socket);
  socket.write(frame(requestBytes));
  const response = await responsePromise;
  if (response.status !== 'accepted') throw new Error(`probe validation rejected: ${response.code}`);
}

function percentile(sorted, value) {
  return sorted[Math.max(0, Math.ceil(sorted.length * value) - 1)];
}

async function measureCase(server, name, message) {
  const socket = await connectLocal(server);
  const request = Buffer.from(JSON.stringify(message), 'utf8');
  for (let index = 0; index < warmups; index += 1) await roundTrip(socket, request);
  const latencies = [];
  for (let index = 0; index < iterations; index += 1) {
    const started = performance.now();
    await roundTrip(socket, request);
    latencies.push(performance.now() - started);
  }
  socket.end();
  const sorted = [...latencies].sort((a, b) => a - b);
  return {
    name,
    request_bytes: request.byteLength,
    response_bytes: Buffer.byteLength(JSON.stringify({ status: 'accepted', message }), 'utf8'),
    warmups,
    repetitions: iterations,
    p50_ms: Number(percentile(sorted, 0.5).toFixed(4)),
    p95_ms: Number(percentile(sorted, 0.95).toFixed(4)),
  };
}

const server = createProbeServer();
try {
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(socketPath, resolve);
  });
  const small = await measureCase(server, 'base-command', baseMessage);
  const medium = await measureCase(server, 'command-with-3500-byte-extension', {
    ...baseMessage,
    command_id: 'ipc-measurement-medium',
    extensions: { 'runweft.measurement': 'x'.repeat(3500) },
  });
  console.log(JSON.stringify({
    probe: 'local-unix-domain-socket-framing-and-schema-validation',
    execution_enabled: false,
    platform: `${platform()} ${release()} ${arch()}`,
    node: process.version,
    cpu: cpus()[0]?.model ?? 'unknown',
    results: [small, medium],
  }, null, 2));
} finally {
  await new Promise((resolve) => server.close(resolve));
  await unlink(socketPath).catch(() => {});
}
