# Nano 0.4: React versus direct HTML and Reflex

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

## What was timed and verified

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

## Complete state and hydrated DOM, from navigation start

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

## First event: click to observed DOM update

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

## Per-application preparation / compilation

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

## Framework build costs (seconds)

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

## Cold server process startup

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

## Transfer sizes and React component costs

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

## Environment, provenance and reproduction

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
