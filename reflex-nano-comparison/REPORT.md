# Reflex Nano 0.4 versus Reflex: full comparison

**Nano wins most clearly on app preparation, server startup, server memory and backend events. Direct HTML also gives the lightest browser path. Nano React is not uniformly faster or smaller: large list updates are slower than Reflex, and retained JavaScript heap is higher.**

This report compares the exact **Nano 0.4.0** release with **Reflex 0.9.11**. It combines the preserved **144-navigation** release experiment with **148,500 fresh backend events** and **90 additional browser navigations** for repeated updates and memory. No Nano 0.3 performance numbers are attributed to 0.4.

Nano's schema, authoritative state, event programs, computed values and render-plan compiler are Rust. PyO3 exposes native objects with no Python event callbacks. Exported applications run without Python. Native Rust closures extend the bounded declarative programs; those programs execute through the Rust interpreter, rather than being JIT-compiled to separate machine code per application. Browser DOM operations, React, public snapshots, input drafts and local hooks run in JavaScript.

Reflex executes state and application handlers in Python and compiles UI definitions to React. The tested version uses React Router/Vite and Granian ASGI; some public architecture documentation retains descriptions of older stack layers. The installed version and recorded dependencies govern this experiment. [Reflex architecture](https://reflex.dev/docs/advanced-onboarding/how-reflex-works/).

## How to read the report

The release section retains all compilation, startup, load, reload, first-event and transfer measurements. The next sections add current-version backend throughput, repeated browser mutations, memory and feature coverage. These datasets are not pooled: the memory sweep forces garbage collection and runs additional events between initial load and reload.

The roughly 29 ms Nano app-preparation figure uses its **prebuilt engine through Python bindings and exported native definitions**. Editing native Rust application source requires Cargo compilation. The roughly 63 ms cached Cargo figure means **no source changed**, not an incremental edit. Custom Rust app rebuilds and source-edit HMR were not measured.

All tests are local loopback experiments in a shared container. Results are specific to these fixtures, versions and configurations; they do not establish universal speedups or complete Reflex compatibility.

## Release lifecycle measurements

Measured 2026-09-15T11:34:07Z using Nano 0.4.0 and production Reflex
0.9.11. All table values are milliseconds
unless marked otherwise. Cells report **median [minimum–maximum]**.

For 1,000 rows, Nano React/Rust reached complete state + hydrated DOM in
**130.5 ms initially** and
**112.8 ms on first reload**.
Reflex took 159.8 and 121.6 ms:
ratios of 1.22× and
1.08×, respectively.
Direct HTML remains the lighter option: 89.5 and
85.7 ms for the same conditions.
At 10,000 rows the React difference narrows: Nano React/Rust was
752.3 ms initially and 731.5 ms on reload,
versus Reflex's 799.2 and 708.7 ms.
The reload median is slightly worse, with overlapping observed ranges. React
mode does not deliver a consistent large-load hydration speedup in this test.
These are local fixture results, not universal framework or language speedups.

### What was timed and verified

All **144 navigations passed** across 100, 1,000 and 10,000 rows.
Each row has the same ID, label, done value and working toggle button. There is
no list virtualization. The complete public state must arrive over a real
WebSocket; the probe validates every item and every rendered row. Nano 0.4 must
also report `ready` after its renderer commits, and be connected. React mode
must use real `hydrateRoot`, with no recorded recoverable hydration errors.
Reflex must report its framework hydration condition. The initial click changes
both the counter and computed double. Reload retains that state; another click
and the last row toggle must work. Zero recorded page errors.

One workload runs at a time, with no compiler overlapping browser timings.
There are 3 fresh server/browser processes per framework/size,
2 fresh contexts per process: 6 initial
loads and six first reloads per cell. Trials use deterministic randomized order.
Initial loads have fresh browser HTTP caches and sessions. Reload retains them.
Reload means document reload, not source-edit HMR or restarting the server.
Small sample counts and shared-container variability limit fine comparisons.

The Rust and PyO3 React hosts run the same native manifest/backend and bundled
React adapter. PyO3 releases the GIL while Rust serves. All schemas, authoritative
state, computed values and event programs are native in both hosts.

### Complete state and hydrated DOM, from navigation start

| Rows | Phase | Nano HTML / Rust | Nano React / Rust | Nano React / PyO3 | Reflex |
| --- | --- | --- | --- | --- | --- |
| 100 | initial | 40.7 [32.7–51.2] | 60.6 [56.5–69.2] | 60.7 [51.7–64.6] | 97.7 [80.0–131.3] |
| 100 | first_reload | 34.1 [30.9–36.2] | 42.9 [38.8–92.1] | 38.8 [36.7–43.7] | 59.4 [54.7–77.2] |
| 1,000 | initial | 89.5 [83.9–103.1] | 130.5 [116.1–146.1] | 127.2 [114.1–142.9] | 159.8 [137.9–213.5] |
| 1,000 | first_reload | 85.7 [84.1–91.8] | 112.8 [103.3–118.2] | 107.9 [103.9–123.4] | 121.6 [108.7–157.8] |
| 10,000 | initial | 589.6 [573.9–783.9] | 752.3 [696.8–771.8] | 724.1 [699.4–786.4] | 799.2 [733.1–854.2] |
| 10,000 | first_reload | 617.2 [582.9–1022.0] | 731.5 [689.7–846.9] | 739.5 [675.7–1013.2] | 708.7 [634.8–745.3] |

This condition includes document delivery, parsing, JavaScript startup, WebSocket
state transfer and hydration. It is not an isolated React hydration function
timer. In particular, Nano also does Rust SSR/compression on the request path.

The 1,000-row median breakdown below separates these milestones. Paint-ready
waits two animation frames after the validated ready condition.

| Framework | Phase | TTFB | Response end | Full WS state | Ready | Paint-ready |
| --- | --- | --- | --- | --- | --- | --- |
| Nano HTML / Rust | initial | 2.4 | 4.1 | 87.3 | 89.5 | 109.4 |
| Nano HTML / Rust | first_reload | 1.8 | 4.0 | 84.1 | 85.7 | 106.5 |
| Nano React / Rust | initial | 2.5 | 3.9 | 127.9 | 130.5 | 158.8 |
| Nano React / Rust | first_reload | 1.8 | 3.2 | 110.9 | 112.8 | 135.3 |
| Nano React / PyO3 | initial | 2.5 | 3.9 | 125.2 | 127.2 | 153.6 |
| Nano React / PyO3 | first_reload | 1.8 | 3.5 | 106.3 | 107.9 | 134.6 |
| Reflex | initial | 5.1 | 6.6 | 148.3 | 159.8 | 183.4 |
| Reflex | first_reload | 2.6 | 3.9 | 108.0 | 121.6 | 140.4 |

### First event: click to observed DOM update

Programmatic DOM click, with both count and computed double validated. This
includes WebSocket transit, Rust/Python backend execution and browser update.

| Rows | Phase | Nano HTML / Rust | Nano React / Rust | Nano React / PyO3 | Reflex |
| --- | --- | --- | --- | --- | --- |
| 100 | initial | 1.7 [1.3–6.0] | 4.5 [4.1–5.3] | 4.5 [3.9–6.4] | 4.7 [4.1–7.1] |
| 100 | first_reload | 1.2 [1.0–1.2] | 1.9 [1.9–2.8] | 1.9 [1.6–2.8] | 3.4 [3.2–7.1] |
| 1,000 | initial | 1.8 [1.7–2.0] | 5.1 [4.3–6.1] | 4.4 [4.0–5.5] | 10.6 [9.6–15.2] |
| 1,000 | first_reload | 1.2 [1.1–4.8] | 3.3 [2.7–4.6] | 2.9 [2.6–5.1] | 7.1 [6.4–8.5] |
| 10,000 | initial | 1.9 [1.8–2.5] | 7.8 [7.5–9.8] | 8.0 [7.6–8.8] | 48.1 [34.4–56.5] |
| 10,000 | first_reload | 1.1 [1.0–5.8] | 8.6 [8.5–96.3] | 8.1 [6.7–9.3] | 36.3 [32.6–39.2] |

### Per-application preparation / compilation

3 fresh process trials per framework/size, with dependencies
installed. Process startup/shutdown is included. Nano imports bindings,
constructs/exports native definitions, validates/builds the app and renders all
rows using the prebuilt engine. Both Nano modes use a prebuilt generic adapter;
there is no per-app JavaScript bundle build. Reflex runs its real Python
compiler, production React Router/Vite build, route prerendering and compression.
Reflex's stock frontend initializer normalizes package.json outside the timer
to preserve its existing dependency-installation cache.

| Rows | Nano HTML | Nano React | Reflex total |
| --- | --- | --- | --- |
| 100 | 24.3 [23.0–29.8] | 25.0 [23.8–26.0] | 2611.7 [2329.7–2666.0] |
| 1,000 | 28.5 [28.1–34.2] | 28.8 [26.6–30.3] | 2746.8 [2595.4–2896.8] |
| 10,000 | 56.9 [53.5–60.5] | 59.6 [54.6–59.9] | 3020.7 [2812.7–3219.4] |

This compares app preparation with those architectures; it is not a Rust versus
Python compiler benchmark. Changing the registered React package set or adapter
requires a separate frontend and Rust build. Those costs are measured below.

### Framework build costs (seconds)

| Stage | Seconds |
| --- | --- |
| clean-native | 83.371 |
| cached-native | 0.063 |
| python-wheel-after-native | 41.465 |
| React frontend build, median of 3 | 0.617 |

The native source build starts from an empty target directory with cached Cargo
sources, offline, one job, release ThinLTO. The wheel build follows that native
build. React bundle timings use installed pinned npm dependencies and reproduce
the exact tested asset bytes. Toolchain/dependency installation and operating
system filesystem cache flushing are excluded. These build costs are not hidden
inside the much smaller per-app preparation numbers.

### Cold server process startup

Spawn to successful HTTP probe; no browser launch or application compilation.
Native startup includes manifest loading, plan construction and relevant asset
compression; Reflex includes backend imports and its production static server.
Probes use the native static asset or Reflex `/ping` endpoint. A 5 ms probe
interval and Node/fetch overhead make short timings coarse.

| Rows | Nano HTML / Rust | Nano React / Rust | Nano React / PyO3 | Reflex |
| --- | --- | --- | --- | --- |
| 100 | 27.6 [22.8–28.2] | 65.1 [62.6–71.5] | 90.6 [80.8–106.8] | 484.2 [471.5–540.6] |
| 1,000 | 20.9 [19.8–22.6] | 62.0 [61.3–69.5] | 87.8 [82.3–90.8] | 487.3 [472.6–792.2] |
| 10,000 | 36.1 [35.8–46.2] | 102.9 [85.1–105.8] | 119.2 [110.8–119.7] | 679.4 [603.2–686.2] |

### Transfer sizes and React component costs

Median initial-load KiB. HTTP transfer uses Resource Timing (including its header
estimate); decoded bytes are decompressed bodies. WebSocket bytes are received
application payloads through readiness, excluding framing.

| Rows | Framework | HTTP transferred KiB | HTTP decoded KiB | WS received KiB |
| --- | --- | --- | --- | --- |
| 100 | Nano HTML / Rust | 10.4 | 45.2 | 4.2 |
| 100 | Nano React / Rust | 69.7 | 233.1 | 4.2 |
| 100 | Nano React / PyO3 | 69.7 | 233.1 | 4.2 |
| 100 | Reflex | 162.4 | 514.4 | 6.4 |
| 1,000 | Nano HTML / Rust | 21.8 | 231.5 | 41.9 |
| 1,000 | Nano React / Rust | 81.2 | 419.4 | 41.9 |
| 1,000 | Nano React / PyO3 | 81.2 | 419.4 | 41.9 |
| 1,000 | Reflex | 174.0 | 692.8 | 44.2 |
| 10,000 | Nano HTML / Rust | 123.5 | 2129.9 | 437.5 |
| 10,000 | Nano React / Rust | 182.6 | 2317.9 | 437.5 |
| 10,000 | Nano React / PyO3 | 182.6 | 2317.9 | 437.5 |
| 10,000 | Reflex | 266.9 | 2512.2 | 439.7 |

The timed fixture uses native HTML elements in both React frontends. Radix is
tested functionally in a separate fixture; its component/CSS chunk is lazy and
is not downloaded by this benchmark. Direct HTML pages download no React.
The following standalone assets show the additional cost when a page uses the
bundled component registry. Gzip sizes here use Python gzip; actual wire bytes
above come from the Rust server.

| React asset | Decoded KiB | Gzip KiB |
| --- | --- | --- |
| components-7CdcbHJ9.css | 671.0 | 78.5 |
| components-DpRbI6sD.js | 268.3 | 73.5 |
| react.js | 207.3 | 65.4 |

Third-party React components hydrate an explicit Rust-rendered HTML fallback,
then mount their interactive version in the browser. Native HTML elements use
real React hydration over the Rust SSR output. No claim of arbitrary third-party
React SSR, React Server Components or full Reflex wrapper parity is made.

### Environment, provenance and reproduction

- CPU: AMD EPYC 9V74 80-Core Processor; cgroup quota `800000 100000`.
- Python 3.12.14, Node v24.19.0,
  Rust `rustc 1.97.0 (2d8144b78 2026-07-07)`, Chromium 133.0.6943.0.
- Headless Linux, 1280×900, loopback frontend/backend, no CPU/network throttling.
- Nano React 19.2.8; Reflex React 19.2.8.
  The frameworks retain their pinned frontend dependencies and different native
  plan/transport implementations. React mode narrows the renderer difference;
  it does not isolate Rust as the only experimental variable.
- Nano uses in-memory Rust sessions and native WebSockets; Reflex uses its
  in-memory Python state manager and Socket.IO WebSockets. Distributed state,
  TLS, remote latency and production deployment behavior are outside this test.
- Source and binary hashes are in `benchmarks/lifecycle/react-results.json`.
  All recorded hashes are verified by this report generator. The per-navigation
  data is also flattened to `react-summary.csv`.

```bash
python benchmarks/build_engine.py
python benchmarks/verify_release.py
NANO_CHROMIUM_PATH=/path/to/chromium python benchmarks/lifecycle/run.py \
  --rows 100 1000 10000 --build-runs 3 --server-runs 3 --contexts 2 \
  --frameworks nano nano_react nano_react_python reflex --output react-results.json
python benchmarks/lifecycle/react_report.py
```

Install the pinned benchmark Python/frontend dependencies first as described in
the existing lifecycle guide. The 0.3 lifecycle and backend results remain
historical reports; they have not been relabeled as 0.4 measurements.

## WebSocket performance — Reflex Nano 0.4 — fresh run

Experiment date: 2026-09-15. These are **measured backend event-delivery results** on one machine, not projected Rust speedups.

Nano’s authoritative state, event programs, computed expressions and scheduling are Rust implementations. The Python-hosted case only enters PyO3 to construct/start that same native runtime. The standalone case does not load Python.

### One connected client

Median event-to-observed-state latency in milliseconds. Each value is the median of five independent process-run medians. Smaller is better.

| Rows | Operation | Nano Rust | Nano PyO3 | Reflex | Reflex / Nano latency |
| --- | --- | --- | --- | --- | --- |
| 0 | increment | 0.084 | 0.098 | 0.451 | 5.40x |
| 0 | increment_async | 0.114 | 0.131 | 0.471 | 4.14x |
| 100 | increment | 0.091 | 0.085 | 0.426 | 4.68x |
| 100 | increment_async | 0.112 | 0.111 | 0.459 | 4.09x |
| 100 | toggle | 0.223 | 0.192 | 0.869 | 3.89x |
| 1,000 | increment | 0.099 | 0.088 | 0.478 | 4.81x |
| 1,000 | increment_async | 0.114 | 0.126 | 0.411 | 3.61x |
| 1,000 | toggle | 0.866 | 0.900 | 4.325 | 4.99x |
| 10,000 | increment | 0.092 | 0.120 | 0.402 | 4.37x |
| 10,000 | increment_async | 0.116 | 0.117 | 0.471 | 4.06x |
| 10,000 | toggle | 8.072 | 8.833 | 38.534 | 4.77x |

Ratios divide Reflex latency by Nano latency. For example, a ratio of 3 means one-third the latency in this workload.

### Eight concurrent clients

Aggregate completed events per second, median across five runs. Higher is better. Each client sends its next event after observing the expected state; Nano also waits for its receipt before the next iteration.

| Rows | Operation | Nano Rust | Nano PyO3 | Reflex | Nano / Reflex throughput |
| --- | --- | --- | --- | --- | --- |
| 0 | increment | 17,502 | 17,108 | 2,955 | 5.92x |
| 0 | increment_async | 18,078 | 15,761 | 3,064 | 5.90x |
| 100 | increment | 16,500 | 18,630 | 3,141 | 5.25x |
| 100 | increment_async | 15,447 | 17,169 | 3,050 | 5.06x |
| 100 | toggle | 9,342 | 9,159 | 1,343 | 6.95x |
| 1,000 | increment | 14,175 | 16,472 | 2,948 | 4.81x |
| 1,000 | increment_async | 16,644 | 14,731 | 3,204 | 5.19x |
| 1,000 | toggle | 1,667 | 1,648 | 259 | 6.44x |
| 10,000 | increment | 13,958 | 12,018 | 2,974 | 4.69x |
| 10,000 | increment_async | 13,702 | 12,694 | 3,221 | 4.25x |
| 10,000 | toggle | 197 | 204 | 30 | 6.56x |

### Tail latency with 1,000 and 10,000 rows

Median of per-run p95 latencies, milliseconds. These are not pooled percentiles or production service-level guarantees.

| Rows | Operation | Clients | Nano Rust | Nano PyO3 | Reflex |
| --- | --- | --- | --- | --- | --- |
| 1,000 | increment | 1 | 0.218 | 0.138 | 0.761 |
| 1,000 | increment | 8 | 0.671 | 0.640 | 3.918 |
| 1,000 | increment_async | 1 | 0.219 | 0.213 | 0.599 |
| 1,000 | increment_async | 8 | 0.684 | 0.661 | 3.767 |
| 1,000 | toggle | 1 | 1.167 | 1.444 | 5.864 |
| 1,000 | toggle | 8 | 6.770 | 6.100 | 44.790 |
| 10,000 | increment | 1 | 0.180 | 0.234 | 0.526 |
| 10,000 | increment | 8 | 0.676 | 0.851 | 3.934 |
| 10,000 | increment_async | 1 | 0.204 | 0.214 | 0.675 |
| 10,000 | increment_async | 8 | 0.682 | 0.673 | 3.336 |
| 10,000 | toggle | 1 | 11.131 | 13.973 | 42.133 |
| 10,000 | toggle | 8 | 47.758 | 48.072 | 293.176 |

### Method

- 330 workload groups, **148,500 measured events**, 20 warm-up events per client per group, and 100 measured events per client. The 1,000-row list is approximately 42 KiB as compact JSON; the exact payload depends on values and protocol framing.
- Five process runs per framework and state size. Framework order is randomized with recorded seed 20260915. Each framework runs alone; concurrent clients within a workload are intentional. The harness schedules workloads sequentially; shared-machine activity and other processes are not isolated or controlled.
- One backend process per case. Nano uses its standard Tokio runtime and blocking worker pool; the Reflex app uses Granian 2.8.3 embedded ASGI with its default single runtime thread and an asyncio Python loop. Both have the same machine CPU allocation. This is not a single-core or equal-thread-count experiment.
- Reflex 0.9.11 runs its actual application, state manager, lifecycle, event processor, middleware, computed tracking, Engine.IO and Socket.IO stack. Its Python page/state definitions are compiled once before serving. A skip-compile environment setting prevents redundant startup compilation. No event dispatch, state manager or transport is mocked.
- Reflex is configured for **in-memory state**, WebSocket transport, no development reload and telemetry disabled. Nano also uses in-memory state. The result does not compare Nano against disk-backed Reflex state.
- The same aiohttp client implementation handles both protocols over loopback WebSockets with compression not requested. The client accounts for the actual Socket.IO/Engine.IO framing in Reflex. The server is a separate process from the load generator.
- Timing begins before serializing/sending an event and ends when the client has decoded and observed the expected changed state and computed fields. Nano’s subsequent ack wait is excluded from the latency metric but included in workload throughput. Connection establishment, hydration and warm-up are untimed.
- Concurrent workers synchronize after warm-up. Throughput uses all measured events divided by the wall-clock span from the earliest measured start to the latest measured finish.
- Every timed workload checks its complete final public state. The shared contract also verifies forms, input, streaming checkpoints, background pushes alongside foreground events, chains, reconnects and session isolation for all three configurations. These extra features are correctness checks, not additional timing workloads.
- The baseline behavior is defined in `shared.py`; its Rust port is `crates/nano-core/src/demo.rs`. The two implementations have matching tested behavior, rather than identical source code.
- Raw per-event samples, all five run medians, bytes received/sent, startup metadata, package versions and hashes of measured sources are retained in `backend-results.json`. `backend-summary.csv` includes all aggregate metrics.

### What the result means

The Rust implementation removes Python event execution and avoids copying unchanged state collections. Rust stores each field behind a shared pointer and copies a field when it changes. Computed values reuse shared cached outputs; full snapshots and wire deltas serialize borrowed Rust values. This makes the scalar-event path largely insensitive to the untouched list size in these tests.

Toggling a row still copies and sends its containing list and recomputes the remaining-item count. It therefore scales with collection size. The Python binding host and native executable are in similar performance ranges; differences between their measured runs should not be interpreted as a precise binding overhead, because no Python callback participates in either event path.

### Limits

These ratios include differences in implementation, state layout, schedulers, server stacks, protocol framing and the remaining framework work. They do not isolate the Rust language as a causal factor. In particular, the Reflex state hierarchy, routing metadata, proxies and ecosystem support are more extensive; see [PARITY.md](PARITY.md).

This measures warmed backend event delivery. It excludes React/browser rendering, page compilation/builds, startup, initial synchronization, TLS, remote latency, database work, uploads and application-specific computation. It does not establish a whole-application speedup or complete Reflex feature parity.

Runs are short and use a shared execution environment. The Python load generator, OS scheduling, allocator effects and machine noise can limit throughput or move microsecond-scale medians. Five repetitions provide observed variation, not a universal performance guarantee. Raw per-run medians are included so that variation is visible.

### Environment

- CPU: AMD EPYC 9V74 80-Core Processor.
- CPU affinity exposes 9 logical IDs; cgroup quota/period: `800000 100000` (8 CPU equivalents).
- rustc 1.97.0 (2d8144b78 2026-07-07); Python 3.12.14.
- Linux-6.18.44-x86_64-with-glibc2.39.
- Exact package versions and source hashes are in the raw results.



## Fresh repeated browser updates

Three processes per size/configuration, 50 scalar updates and 20 last-row toggles per page.
The complete DOM and computed result is verified after each event. Values are milliseconds,
medians of per-run medians. This includes transport, backend work and browser reconciliation.

| Rows | Update | Nano HTML / Rust | Nano HTML / PyO3 | Nano React / Rust | Nano React / PyO3 | Reflex |
| --- | --- | --- | --- | --- | --- | --- |
| 100 | scalar | 0.60 | 0.50 | 0.75 | 0.70 | 1.55 |
| 100 | toggle | 0.90 | 0.90 | 1.90 | 1.80 | 1.90 |
| 1,000 | scalar | 0.50 | 0.40 | 0.80 | 0.70 | 3.50 |
| 1,000 | toggle | 3.25 | 3.10 | 11.40 | 11.10 | 8.45 |
| 10,000 | scalar | 0.60 | 0.50 | 14.75 | 14.85 | 41.50 |
| 10,000 | toggle | 37.80 | 39.25 | 92.55 | 92.65 | 75.70 |

At 1,000 rows Nano React/Rust takes **11.4 ms** for a row toggle versus Reflex's **8.45 ms**.
At 10,000 rows it takes **92.55 ms** versus **75.7 ms**; direct HTML takes **37.8 ms**.
Nano React's scalar path is faster, but its generic component-plan adapter is not a consistent
winner for list mutations. Profiling is needed to attribute the regression precisely.

The memory sweep forces GC before these repetitions. These samples are kept separate from
the original first-event/lifecycle timings. Per-run p95 values are in browser-summary.csv;
all individual durations are in browser-results.json. These short tests do not establish an SLO.

## Server memory and retained JavaScript heap

Three fresh processes per cell, one connected page. Server RSS/PSS comes from Linux
smaps_rollup after resolving the PID namespace. PSS apportions shared pages; RSS counts shared
pages in full. The Reflex process includes its production static and backend servers.

V8 heap is sampled after forced garbage collection with CDP. The first sample follows full
hydration and one counter event; the second follows the 50 scalar and 20 list updates.
**These are retained JavaScript bytes, not total browser RAM or peak memory.** Native DOM/layout,
GPU memory and other browser processes are excluded. This is not a leak or capacity test.

All values below are MiB, medians across three runs.

| Rows | Configuration | Server PSS | Server RSS | V8 after hydrate/event | V8 after updates |
| --- | --- | --- | --- | --- | --- |
| 100 | Nano HTML / Rust | 4.06 | 4.84 | 1.39 | 1.48 |
| 100 | Nano HTML / PyO3 | 11.18 | 13.48 | 1.39 | 1.48 |
| 100 | Nano React / Rust | 5.50 | 6.28 | 2.35 | 2.84 |
| 100 | Nano React / PyO3 | 12.11 | 14.41 | 2.34 | 2.83 |
| 100 | Reflex | 63.52 | 66.72 | 3.07 | 3.35 |
| 1,000 | Nano HTML / Rust | 6.27 | 7.04 | 2.61 | 2.71 |
| 1,000 | Nano HTML / PyO3 | 13.99 | 16.29 | 2.61 | 2.71 |
| 1,000 | Nano React / Rust | 8.10 | 8.88 | 5.36 | 6.87 |
| 1,000 | Nano React / PyO3 | 15.41 | 17.70 | 5.36 | 6.87 |
| 1,000 | Reflex | 64.37 | 67.57 | 4.91 | 5.16 |
| 10,000 | Nano HTML / Rust | 28.23 | 29.00 | 13.59 | 13.69 |
| 10,000 | Nano HTML / PyO3 | 43.31 | 45.61 | 13.59 | 13.69 |
| 10,000 | Nano React / Rust | 30.66 | 31.44 | 35.02 | 46.52 |
| 10,000 | Nano React / PyO3 | 44.62 | 46.92 | 35.01 | 46.67 |
| 10,000 | Reflex | 74.01 | 77.22 | 22.96 | 23.43 |

Nano's server footprint is much smaller, especially without Python. However, at 10,000 rows,
Nano React retains **35.0 MiB** of JavaScript heap versus Reflex's **23.0 MiB**.
Direct HTML retains **13.6 MiB**. Python mainly changes host startup/footprint;
it does not execute the application event handlers.

## Fresh-run validation

All 330 backend groups, 148,500 events and twelve 11-check live contracts passed.
All 90 extra browser navigations passed, adding 2,250 repeated scalar updates and 900 timed
row toggles. No recorded page errors or recoverable React hydration errors occurred.
Failed setup/smoke attempts were excluded; a PID-namespace lookup fix was required
in the memory collector, with no change to the application runtime.

Python 3.12.14, AMD EPYC 9V74 80-Core Processor,
Linux-6.18.44-x86_64-with-glibc2.39, CPU quota 800000 100000.
Chromium 133.0.6943.0, 1280x900, loopback, no artificial CPU/network throttling.
These are five-process backend and three-process browser/memory experiments in a shared environment.

The immutable source release and its original source hashes were verified against the archive.
All new measured scripts, relevant sources, binaries and wheels were also hash-checked.
The report validates every expected sample count and recorded correctness condition.
The bundle includes the original release, raw JSON/CSV and reproduction scripts; see README.md.

## Feature coverage and semantics

The Reflex column describes framework capabilities, not features all enabled in the timed fixture. Authentication and database workloads were not benchmarked.

| Capability | Nano HTML | Nano React | Reflex |
| --- | --- | --- | --- |
| Authoritative state and event execution | Rust | Rust | Python |
| Definition API | Native Rust; PyO3 constructs native objects | Same | Python classes/decorators |
| Arbitrary application handlers | Native Rust; bounded programs through bindings | Same | Python sync/async functions and libraries |
| Unchanged Reflex source | Not compatible | Not compatible | Native |
| Browser renderer | Direct DOM JavaScript | React 19 / ReactDOM | React 19 / ReactDOM |
| Native HTML SSR/hydration | Rust SSR + DOM adoption | Rust SSR + hydrateRoot | React prerendering/hydration in measured build |
| Third-party component SSR | Explicit HTML fallback | Fallback then browser mount | SSR-capable wrappers; NoSSR where needed |
| Radix, React hooks, portals | Fallback only | Registered components, tested | Broader component/wrapper ecosystem |
| Add React package | Needs HTML fallback | Registry + frontend and Rust rebuild | Python wrapper + frontend build |
| Controlled forms and keyed lists | Tested | Tested | Supported |
| Computed caching | Native dependency expressions | Same | Computed vars/dependency tracking |
| Debounce/throttle/temporal/propagation | Tested | Tested | Supported event actions |
| Streaming/background/chaining | Native checkpoints and Tokio; bounded program semantics | Same | Python generators/async/background handlers |
| State inheritance and substates | Missing | Missing | Supported |
| Static/dynamic routes and history | Tested | Tested | Supported |
| Full on_load lifecycle | Missing | Missing | Supported |
| Cross-tab session semantics | Cookie-shared state | Cookie-shared state | Normally per-tab tokens |
| Event protocol | nano.v1 WebSocket | Same | Socket.IO/Engine.IO; WebSocket in this test |
| File uploads | Missing | Missing | Supported |
| Cookie/local-storage state vars | Missing; session cookie is separate | Same | Supported |
| ORM integration | No framework integration | Same | Database integration available |
| OIDC authentication helpers | No framework integration | Same | Enterprise AuthPlugin, separate package |
| Redis/distributed state manager | Missing | Missing | Available; benchmark used memory |
| Restart persistence | In-memory state is lost | Same | Depends on state manager/deployment |
| Development HMR | No complete watcher/HMR workflow | Same | Development tooling available; not timed |
| Deployment | Native executable or wheel | Same, with embedded frontend | CLI/hosting/container workflows |
| Durable exactly-once events | No; bounded 256-receipt cache | Same | Not established by this comparison |

Reflex supports [wrapping React components](https://reflex.dev/docs/wrapping-react/overview/), [file uploads](https://reflex.dev/docs/library/forms/upload/), [database integration](https://reflex.dev/docs/database/overview/), [client storage](https://reflex.dev/docs/client-storage/overview/) and [page-load events](https://reflex.dev/docs/events/page-load-events/). [OIDC authentication](https://reflex.dev/docs/enterprise/auth/overview/) is an Enterprise capability requiring an additional package; it was not enabled in these measurements.

Nano's Radix integration is real, including composed refs/handlers and portal callbacks. It is not an unchanged generated Reflex frontend. Imported components hydrate explicit Rust-rendered HTML fallbacks and then mount their interactive React version. Rust does not execute arbitrary third-party JavaScript SSR. A single feature-parity percentage would hide these differences.

## Optimization coverage and remaining measurements

Nano includes shared native state fields, transactional writes, dependency-cached computed expressions, constant folding, reusable compiled render plans, SSR DOM reuse, stable keys, subtree dependency skipping, early-input preservation, queued events, gzip/immutable assets and lazy React component bundles. Equal row values are shared in the browser so unrelated scalar events can skip list subtrees.

Collection mutations still copy and transmit their containing field, and list comparison remains proportional to its size. There are no nested wire patches or list virtualization. The full 10,000-row state remains roughly 437.5 KiB. Backend gains cannot simply be multiplied into page-load gains: parsing, DOM size, React work and full state delivery remain. This is an architectural interpretation, not a measured CPU breakdown.

The current results do not establish every Reflex compiler optimization in Rust. They also do not establish source-edit HMR speed, custom Rust application incremental builds, production CPU cost per request, total/peak browser RAM, long-running leaks, maximum connected users, multi-worker or Redis operation, persistence recovery, TLS/WAN/mobile-network behavior, database/auth/upload performance, accessibility conformance or independent security certification. Paint-ready and detailed resource timing are retained in the lifecycle data; LCP and INP are not established.

Direct HTML is the stronger measured performance option for the native-node fixture. Nano React adds useful interoperability but has measurable browser regressions on list-heavy work. Reflex provides the broader application framework. The evidence supports these specific tradeoffs, not complete compatibility at universally higher speed.
