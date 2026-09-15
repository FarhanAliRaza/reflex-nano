# WebSocket performance — Reflex Nano 0.2

This report preserves the original **0.2.0** warmed-backend experiment.
The 0.3.0 rerun is in [BACKEND-OPTIMIZED.md](BACKEND-OPTIMIZED.md).

Experiment date: 2026-09-15. These are **measured backend event-delivery results** on one machine, not projected Rust speedups.

Nano’s authoritative state, event programs, computed expressions and scheduling are Rust implementations. The Python-hosted case only enters PyO3 to construct/start that same native runtime. The standalone case does not load Python.

## One connected client

Median event-to-observed-state latency in milliseconds. Each value is the median of five independent process-run medians. Smaller is better.

| Rows | Workload | Reflex + Granian | Nano / Rust | Nano / PyO3 | Rust latency ratio | PyO3 latency ratio |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 0 | Increment + computed | 0.452 | 0.093 | 0.085 | 4.86× | 5.33× |
| 0 | Deferred increment | 0.369 | 0.114 | 0.111 | 3.22× | 3.31× |
| 100 | Increment + computed | 0.388 | 0.112 | 0.107 | 3.47× | 3.63× |
| 100 | Deferred increment | 0.424 | 0.115 | 0.131 | 3.68× | 3.24× |
| 100 | Toggle row + computed | 0.806 | 0.291 | 0.314 | 2.77× | 2.57× |
| 1,000 | Increment + computed | 0.361 | 0.096 | 0.098 | 3.78× | 3.67× |
| 1,000 | Deferred increment | 0.404 | 0.120 | 0.104 | 3.36× | 3.88× |
| 1,000 | Toggle row + computed | 4.022 | 1.551 | 1.447 | 2.59× | 2.78× |

Ratios divide Reflex latency by Nano latency. For example, a ratio of 3 means one-third the latency in this workload.

## Eight concurrent clients

Aggregate completed events per second, median across five runs. Higher is better. Each client sends its next event after observing the expected state; Nano also waits for its receipt before the next iteration.

| Rows | Workload | Reflex + Granian | Nano / Rust | Nano / PyO3 | Rust throughput ratio |
| ---: | --- | ---: | ---: | ---: | ---: |
| 0 | Increment + computed | 3,511 | 18,861 | 15,758 | 5.37× |
| 0 | Deferred increment | 3,453 | 17,930 | 15,223 | 5.19× |
| 100 | Increment + computed | 3,693 | 16,532 | 16,146 | 4.48× |
| 100 | Deferred increment | 3,613 | 16,789 | 17,165 | 4.65× |
| 100 | Toggle row + computed | 1,445 | 8,561 | 9,081 | 5.93× |
| 1,000 | Increment + computed | 3,671 | 22,372 | 21,222 | 6.09× |
| 1,000 | Deferred increment | 3,519 | 17,708 | 18,525 | 5.03× |
| 1,000 | Toggle row + computed | 278 | 1,564 | 1,583 | 5.62× |

## Tail latency with 1,000 rows

Median of per-run p95 latencies, milliseconds. These are not pooled percentiles or production service-level guarantees.

| Clients | Workload | Reflex + Granian | Nano / Rust | Nano / PyO3 |
| ---: | --- | ---: | ---: | ---: |
| 1 | Increment + computed | 0.454 | 0.133 | 0.137 |
| 8 | Increment + computed | 3.127 | 0.476 | 0.540 |
| 1 | Deferred increment | 0.555 | 0.169 | 0.168 |
| 8 | Deferred increment | 3.443 | 0.613 | 0.544 |
| 1 | Toggle row + computed | 4.398 | 1.955 | 1.727 |
| 8 | Toggle row + computed | 41.246 | 7.635 | 8.522 |

## Method

- 240 workload groups, **162,000 measured events**, 20 warm-up events per client per group, and 150 measured events per client. The 1,000-row list is approximately 42 KiB as compact JSON; the exact payload depends on values and protocol framing.
- Five process runs per framework and state size. Framework order is randomized with recorded seed 20260915. Each framework runs alone; concurrent clients within a workload are intentional. No compilers or browser tests ran during the final timing run.
- One backend process per case. Nano uses its standard Tokio runtime and blocking worker pool; the Reflex app uses Granian 2.8.3 embedded ASGI with its default single runtime thread and an asyncio Python loop. Both have the same machine CPU allocation. This is not a single-core or equal-thread-count experiment.
- Reflex 0.9.11 runs its actual application, state manager, lifecycle, event processor, middleware, computed tracking, Engine.IO and Socket.IO stack. Its Python page/state definitions are compiled once before serving. A skip-compile environment setting prevents redundant startup compilation. No event dispatch, state manager or transport is mocked.
- Reflex is configured for **in-memory state**, WebSocket transport, no development reload and telemetry disabled. Nano also uses in-memory state. The result does not compare Nano against disk-backed Reflex state.
- The same aiohttp client implementation handles both protocols over loopback WebSockets with compression not requested. The client accounts for the actual Socket.IO/Engine.IO framing in Reflex. The server is a separate process from the load generator.
- Timing begins before serializing/sending an event and ends when the client has decoded and observed the expected changed state and computed fields. Nano’s subsequent ack wait is excluded from the latency metric but included in workload throughput. Connection establishment, hydration and warm-up are untimed.
- Concurrent workers synchronize after warm-up. Throughput uses all measured events divided by the wall-clock span from the earliest measured start to the latest measured finish.
- Every timed workload checks its complete final public state. The shared contract also verifies forms, input, streaming checkpoints, background pushes alongside foreground events, chains, reconnects and session isolation for all three configurations. These extra features are correctness checks, not additional timing workloads.
- The baseline behavior is defined in `shared.py`; its Rust port is `crates/nano-core/src/demo.rs`. The two implementations have matching tested behavior, rather than identical source code.
- Raw per-event samples, all five run medians, bytes received/sent, startup metadata, package versions and hashes of measured sources are retained in `results.json`. `summary.csv` includes all aggregate metrics.

## What the result means

The Rust implementation removes Python event execution and avoids copying unchanged state collections. Rust stores each field behind a shared pointer and copies a field when it changes. Computed values and wire deltas reuse unchanged fields. This makes the scalar-event path largely insensitive to the untouched list size in these tests.

Toggling a row still copies and sends its containing list and recomputes the remaining-item count. It therefore scales with collection size. The Python binding host and native executable are in similar performance ranges; differences between their measured runs should not be interpreted as a precise binding overhead, because no Python callback participates in either event path.

## Limits

These ratios include differences in implementation, state layout, schedulers, server stacks, protocol framing and the remaining framework work. They do not isolate the Rust language as a causal factor. In particular, the Reflex state hierarchy, routing metadata, proxies and ecosystem support are more extensive; see [PARITY.md](PARITY.md).

This measures warmed backend event delivery. It excludes React/browser rendering, page compilation/builds, startup, initial synchronization, TLS, remote latency, database work, uploads and application-specific computation. It does not establish a whole-application speedup or complete Reflex feature parity.

Runs are short and use a shared execution environment. The Python load generator, OS scheduling, allocator effects and machine noise can limit throughput or move microsecond-scale medians. Five repetitions provide observed variation, not a universal performance guarantee. Raw per-run medians are included so that variation is visible.

## Environment

- CPU: AMD EPYC 9V74 80-Core Processor.
- CPU affinity exposes 9 logical IDs; cgroup quota/period: `800000 100000` (8 CPU equivalents).
- rustc 1.98.1 (48a229cea 2026-09-01); Python 3.12.14.
- Linux-6.18.44-x86_64-with-glibc2.39.
- Exact package versions and source hashes are in the raw results.

## Reproduce

From the project root after building/installing Nano:

```bash
uv pip install --python .venv/bin/python -r benchmarks/requirements.txt
cargo build --release --locked -p reflex-nano --examples
.venv/bin/python benchmarks/driver.py
.venv/bin/python benchmarks/run.py --output benchmarks/results.json
.venv/bin/python benchmarks/report.py
```

`benchmarks/requirements.lock` records the complete Python package environment used here, excluding local development extensions. For a quick smoke run use `--repeats 1 --events 20 --rows 100 --output benchmarks/smoke.json`. Keep the main result file separate from smoke runs.
