"""Render the recorded benchmark without rerunning workloads."""
import csv
import json
from pathlib import Path

HERE=Path(__file__).resolve().parent
r=json.loads((HERE/'results.json').read_text())
assert len(r['runs'])==240 and len(r['contracts'])==9
names={'increment':'Increment + computed','increment_async':'Deferred increment','toggle':'Toggle row + computed'}
lines=['# WebSocket performance — Reflex Nano 0.2', '',
'Historical 0.2.0 results; see BACKEND-OPTIMIZED.md for 0.3.0.', '',
'Experiment date: 2026-09-15. These are **measured backend event-delivery results** on one machine, not projected Rust speedups.', '',
'Nano’s authoritative state, event programs, computed expressions and scheduling are Rust implementations. The Python-hosted case only enters PyO3 to construct/start that same native runtime. The standalone case does not load Python.', '',
'## One connected client', '',
'Median event-to-observed-state latency in milliseconds. Each value is the median of five independent process-run medians. Smaller is better.', '',
'| Rows | Workload | Reflex + Granian | Nano / Rust | Nano / PyO3 | Rust latency ratio | PyO3 latency ratio |',
'| ---: | --- | ---: | ---: | ---: | ---: | ---: |']
for x in r['summary']:
 if x['clients']==1:
  lines.append(f"| {x['rows']:,} | {names[x['operation']]} | {x['reflex']['median_ms']:.3f} | {x['nano']['median_ms']:.3f} | {x['nano_python']['median_ms']:.3f} | {x['latency_ratio_reflex_over_nano']:.2f}× | {x['latency_ratio_reflex_over_nano_python']:.2f}× |")
lines+=['','Ratios divide Reflex latency by Nano latency. For example, a ratio of 3 means one-third the latency in this workload.','',
'## Eight concurrent clients','','Aggregate completed events per second, median across five runs. Higher is better. Each client sends its next event after observing the expected state; Nano also waits for its receipt before the next iteration.','',
'| Rows | Workload | Reflex + Granian | Nano / Rust | Nano / PyO3 | Rust throughput ratio |',
'| ---: | --- | ---: | ---: | ---: | ---: |']
for x in r['summary']:
 if x['clients']==8:
  lines.append(f"| {x['rows']:,} | {names[x['operation']]} | {x['reflex']['events_per_second']:,.0f} | {x['nano']['events_per_second']:,.0f} | {x['nano_python']['events_per_second']:,.0f} | {x['throughput_ratio_nano_over_reflex']:.2f}× |")
lines+=['','## Tail latency with 1,000 rows','','Median of per-run p95 latencies, milliseconds. These are not pooled percentiles or production service-level guarantees.','',
'| Clients | Workload | Reflex + Granian | Nano / Rust | Nano / PyO3 |','| ---: | --- | ---: | ---: | ---: |']
for x in r['summary']:
 if x['rows']==1000:
  lines.append(f"| {x['clients']} | {names[x['operation']]} | {x['reflex']['p95_ms']:.3f} | {x['nano']['p95_ms']:.3f} | {x['nano_python']['p95_ms']:.3f} |")
lines+=['','## Method','','- 240 workload groups, **162,000 measured events**, 20 warm-up events per client per group, and 150 measured events per client. The 1,000-row list is approximately 42 KiB as compact JSON; the exact payload depends on values and protocol framing.',
'- Five process runs per framework and state size. Framework order is randomized with recorded seed 20260915. Each framework runs alone; concurrent clients within a workload are intentional. No compilers or browser tests ran during the final timing run.',
'- One backend process per case. Nano uses its standard Tokio runtime and blocking worker pool; the Reflex app uses Granian 2.8.3 embedded ASGI with its default single runtime thread and an asyncio Python loop. Both have the same machine CPU allocation. This is not a single-core or equal-thread-count experiment.',
'- Reflex 0.9.11 runs its actual application, state manager, lifecycle, event processor, middleware, computed tracking, Engine.IO and Socket.IO stack. Its Python page/state definitions are compiled once before serving. A skip-compile environment setting prevents redundant startup compilation. No event dispatch, state manager or transport is mocked.',
'- Reflex is configured for **in-memory state**, WebSocket transport, no development reload and telemetry disabled. Nano also uses in-memory state. The result does not compare Nano against disk-backed Reflex state.',
'- The same aiohttp client implementation handles both protocols over loopback WebSockets with compression not requested. The client accounts for the actual Socket.IO/Engine.IO framing in Reflex. The server is a separate process from the load generator.',
'- Timing begins before serializing/sending an event and ends when the client has decoded and observed the expected changed state and computed fields. Nano’s subsequent ack wait is excluded from the latency metric but included in workload throughput. Connection establishment, hydration and warm-up are untimed.',
'- Concurrent workers synchronize after warm-up. Throughput uses all measured events divided by the wall-clock span from the earliest measured start to the latest measured finish.',
'- Every timed workload checks its complete final public state. The shared contract also verifies forms, input, streaming checkpoints, background pushes alongside foreground events, chains, reconnects and session isolation for all three configurations. These extra features are correctness checks, not additional timing workloads.',
'- The baseline behavior is defined in `shared.py`; its Rust port is `crates/nano-core/src/demo.rs`. The two implementations have matching tested behavior, rather than identical source code.',
'- Raw per-event samples, all five run medians, bytes received/sent, startup metadata, package versions and hashes of measured sources are retained in `results.json`. `summary.csv` includes all aggregate metrics.', '',
'## What the result means','','The Rust implementation removes Python event execution and avoids copying unchanged state collections. Rust stores each field behind a shared pointer and copies a field when it changes. Computed values and wire deltas reuse unchanged fields. This makes the scalar-event path largely insensitive to the untouched list size in these tests.', '',
'Toggling a row still copies and sends its containing list and recomputes the remaining-item count. It therefore scales with collection size. The Python binding host and native executable are in similar performance ranges; differences between their measured runs should not be interpreted as a precise binding overhead, because no Python callback participates in either event path.', '',
'## Limits','','These ratios include differences in implementation, state layout, schedulers, server stacks, protocol framing and the remaining framework work. They do not isolate the Rust language as a causal factor. In particular, the Reflex state hierarchy, routing metadata, proxies and ecosystem support are more extensive; see [PARITY.md](PARITY.md).', '',
'This measures warmed backend event delivery. It excludes React/browser rendering, page compilation/builds, startup, initial synchronization, TLS, remote latency, database work, uploads and application-specific computation. It does not establish a whole-application speedup or complete Reflex feature parity.', '',
'Runs are short and use a shared execution environment. The Python load generator, OS scheduling, allocator effects and machine noise can limit throughput or move microsecond-scale medians. Five repetitions provide observed variation, not a universal performance guarantee. Raw per-run medians are included so that variation is visible.', '',
'## Environment','',
f"- CPU: {r['environment']['cpu']}.",
f"- CPU affinity exposes {len(r['environment']['affinity'])} logical IDs; cgroup quota/period: `{r['environment']['cpu_max']}` (8 CPU equivalents).",
f"- {r['environment']['rustc']}; Python {r['environment']['python']}.",
f"- {r['environment']['platform']}.",
'- Exact package versions and source hashes are in the raw results.', '',
'## Reproduce','',
'From the project root after building/installing Nano:', '',
'```bash',
'uv pip install --python .venv/bin/python -r benchmarks/requirements.txt',
'cargo build --release --locked -p reflex-nano --examples',
'.venv/bin/python benchmarks/driver.py',
'.venv/bin/python benchmarks/run.py --output benchmarks/results.json',
'.venv/bin/python benchmarks/report.py',
'```','',
'`benchmarks/requirements.lock` records the complete Python package environment used here, excluding local development extensions. For a quick smoke run use `--repeats 1 --events 20 --rows 100 --output benchmarks/smoke.json`. Keep the main result file separate from smoke runs.']
(HERE.parent/'docs/BENCHMARKS.md').write_text('\n'.join(lines)+'\n')
with (HERE/'summary.csv').open('w',newline='') as file:
 writer=csv.writer(file);writer.writerow(['rows','workload','clients','framework','median_ms','p95_ms','events_per_second','received_bytes_per_event','run_medians_ms'])
 for x in r['summary']:
  for framework in ('reflex','nano','nano_python'):
   v=x[framework];writer.writerow([x['rows'],x['operation'],x['clients'],framework,v['median_ms'],v['p95_ms'],v['events_per_second'],v['received_bytes_per_event'],json.dumps(v['run_medians_ms'])])
print('Wrote docs/BENCHMARKS.md and benchmarks/summary.csv')
