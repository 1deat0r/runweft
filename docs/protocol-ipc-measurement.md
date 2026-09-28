# Protocol IPC measurement

This is a local prototype measurement for framed request/response plus protocol
validation. It uses a temporary Unix domain socket and a single sequential client.
It is not a production IPC implementation, coordinator, throughput test, or
performance claim for a future runtime.

Run it after building the TypeScript workspace:

```sh
npm run build
node scripts/measure-ipc.mjs
```

The probe validates `run.submit` messages with the TypeScript validator, measures
request/response round-trip latency, and checks that every response is accepted.
It invokes no provider or tool and uses no credentials or external service. Defaults
are 100 warmups and 500 measured round trips per payload. Override them with
`RUNWEFT_IPC_WARMUPS` and `RUNWEFT_IPC_ITERATIONS` when repeating the experiment.

## Result recorded 2026-09-29

Environment: Linux `7.0.0-34-generic`, Node `v26.8.1`, x64, AMD Ryzen 7 5800X
8-Core Processor. Command: `node scripts/measure-ipc.mjs`; 100 warmups and 500
repetitions per payload.

| Payload | Request bytes | Response bytes | p50 | p95 |
|---|---:|---:|---:|---:|
| Base `run.submit` | 446 | 478 | 0.1087 ms | 0.1530 ms |
| `run.submit` with 3,500-byte extension string | 3,985 | 4,017 | 0.0964 ms | 0.1626 ms |

These numbers include local socket framing, JSON serialization/parsing, schema and
semantic validation, and the response round trip. They are one host snapshot; use
the script to reproduce measurements on a target deployment platform.
