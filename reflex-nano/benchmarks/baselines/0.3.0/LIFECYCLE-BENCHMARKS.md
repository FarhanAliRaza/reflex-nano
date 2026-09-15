# Compile, first load, reload and full hydration

Measured 2026-09-15T10:15:04Z on **Nano 0.3.0**, the preserved **Nano 0.2.0** baseline, and **Reflex 0.9.11**.
All values below are milliseconds unless indicated; brackets show min–max.

## What was verified

Both production applications render the same 100, 1,000 or 10,000 rows. Every row
has an ID, label, done value, and working toggle button. The complete six-field
public state (`count`, `doubled`, `remaining`, `name`, `progress`, `items`) must
arrive over the real WebSocket. The harness validates **every item**, checks
every rendered row, and waits for the framework's connected/hydrated condition.
It then clicks Increment and checks both the changed count and computed double.
The first reload must restore that changed state; another increment and the last
row's toggle must work. All **240 navigations passed**, with zero recorded
page errors. Nothing is virtualized or lazily omitted from this fixture.

Nano's schema, programs, storage, computed values and event execution are Rust.
The fixture binds native `Node` objects through PyO3 and exports a native manifest;
the Rust executable serves that manifest without a Python process. The Python
host reads the same manifest and releases the GIL while Rust serves it.

## App preparation and production compilation

5 fresh process trials per framework/row count, randomized order. Dependencies
are already installed. Reflex runs its real compiler, production React Router/Vite
build, route prerendering and configured static compression. Each production build
cleans its previous bundle; code generation reuses the initialized output directory.
Nano constructs, exports, validates and renders the entire application using the
**prebuilt Rust engine**; it has no per-app JavaScript bundling step. Process startup
and shutdown are included in these wall-clock values.

| Rows | Nano app preparation | Reflex import + codegen | Reflex production build | Reflex total |
| --- | --- | --- | --- | --- |
| 100 | 23.43 [22.79–27.14] | 517.77 [480.10–738.11] | 2096.31 [1932.21–2380.75] | 2677.94 [2441.08–2860.85] |
| 1,000 | 26.75 [23.73–29.79] | 496.39 [445.36–535.49] | 1995.45 [1835.19–2027.22] | 2495.58 [2370.68–2501.22] |
| 10,000 | 67.73 [58.40–306.78] | 552.64 [515.32–884.20] | 2260.39 [2203.15–2492.61] | 2830.64 [2800.39–3087.36] |

These are application-to-deployable-output costs, **not a Rust-vs-Python compiler
speed claim**. Rust source builds are measured separately in `build-timings.json`:
one fresh target build with cached registry sources, one cached no-change rebuild,
and wheel construction after the native build. Toolchain/dependency installation
and OS cache flushing are excluded. Rust 1.97.0 is pinned because this container's
1.98.1 executable crashes before compilation; the preserved 0.2.0 binary was built
with 1.98.1. That compiler change is a confound in the native before/after comparison.

Before each Reflex codegen trial, the stock frontend initializer normalizes its
package.json outside the timer. npm changes its formatting; without normalization,
Reflex's byte-comparison cache unnecessarily re-runs package installation. No
compiler, runtime, dependency version or generated browser code is patched.

## Initial load and first reload: full state + DOM ready

Times start at browser navigation. The end condition is the complete validated
state plus DOM and connected/hydrated framework condition described above.
This is a common **application readiness** condition, not identical internal
hydration algorithms: Reflex uses React hydration; Nano uses its own DOM renderer.
Both deliver server-rendered/prerendered HTML in this experiment.

5 independent server and browser processes per framework/size;
2 fresh browser contexts each, for 10
initial loads and first reloads per cell. Initial loads have fresh HTTP caches and
sessions. First reloads retain that tab's cache and session. “Reload” means a
browser page reload, not source-edit HMR or a server restart.

| Rows | Navigation | nano | nano_python | reflex | nano_before |
| --- | --- | --- | --- | --- | --- |
| 100 | initial | 37.65 [34.10–43.80] | 38.70 [35.30–44.10] | 89.35 [82.80–111.50] | 48.85 [41.70–57.50] |
| 100 | first_reload | 33.05 [21.50–43.70] | 34.75 [31.30–43.80] | 59.80 [53.90–67.70] | 46.70 [34.40–55.30] |
| 1,000 | initial | 100.00 [87.00–104.90] | 94.10 [91.10–108.30] | 157.65 [136.90–174.90] | 177.90 [149.20–194.00] |
| 1,000 | first_reload | 88.35 [82.30–130.50] | 92.60 [77.40–240.40] | 122.30 [113.70–128.30] | 160.70 [150.50–188.80] |
| 10,000 | initial | 598.75 [552.70–765.50] | 588.30 [567.80–668.30] | 772.55 [705.20–850.60] | 1419.20 [1350.30–1712.50] |
| 10,000 | first_reload | 571.70 [545.00–594.70] | 561.90 [537.40–594.60] | 668.20 [640.80–807.90] | 1453.70 [1270.50–2033.40] |

The 1,000-row timing breakdown below reports medians. TTFB, response end, full
WebSocket state, ready and paint-ready are all measured from navigation start.
Paint-ready waits two animation frames after ready. The final column is the
separate click-to-observed-DOM-change latency, using a programmatic DOM click;
it is not human input latency. Browser timestamps exclude Playwright polling time.

| Framework | Navigation | TTFB | Response end | Full WS state | Ready | Paint-ready | First event |
| --- | --- | --- | --- | --- | --- | --- | --- |
| nano | initial | 2.50 | 3.75 | 97.80 | 100.00 | 120.10 | 1.85 |
| nano | first_reload | 2.05 | 3.55 | 86.80 | 88.35 | 107.95 | 1.10 |
| nano_python | initial | 2.75 | 4.30 | 91.35 | 94.10 | 118.20 | 1.85 |
| nano_python | first_reload | 2.05 | 3.70 | 90.95 | 92.60 | 109.85 | 1.25 |
| reflex | initial | 4.95 | 6.10 | 143.90 | 157.65 | 177.55 | 10.15 |
| reflex | first_reload | 2.65 | 4.10 | 111.20 | 122.30 | 141.15 | 6.65 |
| nano_before | initial | 3.85 | 5.20 | 150.00 | 177.90 | 179.10 | 25.25 |
| nano_before | first_reload | 3.05 | 4.30 | 134.60 | 160.70 | 166.95 | 23.25 |

## Cold process startup

Spawn-to-successful-HTTP-probe, before any page navigation. Native probes an existing
static asset; Reflex probes `/ping`. This includes native manifest loading/runtime
construction, or Reflex imports/backend page evaluation and the production static
server. No app compilation or browser launch is included. Probe interval is 5 ms;
Node process/fetch overhead makes very short native startup timings coarse.
The server filesystem cache is not flushed.

| Rows | nano | nano_python | reflex | nano_before |
| --- | --- | --- | --- | --- |
| 100 | 22.84 [21.33–23.94] | 41.79 [35.58–45.10] | 464.83 [460.02–481.93] | 20.16 [18.00–22.71] |
| 1,000 | 23.79 [20.50–32.28] | 44.09 [40.82–51.42] | 471.85 [458.38–504.57] | 20.09 [18.26–23.09] |
| 10,000 | 40.01 [35.48–43.12] | 59.59 [52.17–66.16] | 558.60 [556.94–633.82] | 36.29 [35.34–44.86] |

## Initial-load transfer sizes

Median KiB. HTTP transfer is Resource Timing's transferred bytes (including its
header estimate); decoded bytes include decompressed bodies. WebSocket bytes are
received application-frame payloads through ready, without protocol framing.
Use these to distinguish data/asset costs from backend execution speed.

| Rows | Framework | HTTP transferred KiB | HTTP decoded KiB | WS received KiB |
| --- | --- | --- | --- | --- |
| 100 | nano | 10.0 | 43.9 | 4.2 |
| 100 | nano_python | 10.0 | 43.9 | 4.2 |
| 100 | reflex | 162.4 | 514.4 | 6.4 |
| 100 | nano_before | 41.3 | 40.4 | 4.2 |
| 1,000 | nano | 21.5 | 230.2 | 41.9 |
| 1,000 | nano_python | 21.5 | 230.2 | 41.9 |
| 1,000 | reflex | 174.0 | 692.8 | 44.2 |
| 1,000 | nano_before | 227.6 | 226.7 | 41.9 |
| 10,000 | nano | 123.0 | 2128.7 | 437.5 |
| 10,000 | nano_python | 123.0 | 2128.7 | 437.5 |
| 10,000 | reflex | 266.9 | 2512.2 | 439.7 |
| 10,000 | nano_before | 2126.1 | 2125.2 | 437.5 |

## Environment and limits

- AMD EPYC 9V74 80-Core Processor; 9 affinity-visible CPUs,
  cgroup quota `800000 100000` (8 CPU equivalents). One benchmark
  workload at a time; compilers do not overlap browser timing.
- Python 3.12.14, Node v24.19.0,
  Chromium 133.0.6943.0; headless Linux, 1280×900, no network or
  CPU throttling. Both frontend and backend are on loopback.
- React 19.2.8, React Router
  8.3.1, Vite
  8.2.2, Granian
  2.8.3. See raw JSON and frontend lockfile.
- Full DOM/state validation adds observer/scan work to both apps. The first
  counter event confirms readiness after measurement; the reported ready time
  itself does not include the subsequent round trip.
- Native and Reflex assets use their implemented caching/compression behavior;
  no artificial equal-size bundle or equal-payload adjustment is applied.
- This fixture covers full hydration of its defined state and components. It
  does not establish hydration of every component in Reflex's ecosystem, full
  feature parity, real-WAN latency, or a pure language-only speedup.
- The earlier warm backend experiment in `../results.json` is preserved. The
  0.3.0 rerun is `../optimized-results.json`; neither is pooled with browser timing.

## Reproduce

Install the pinned Python benchmark requirements, Nano wheel, Node, and Playwright
Chromium. From `benchmarks/lifecycle`, run `prepare_reflex.py`, compile once to
install Reflex's frontend dependencies, then run:

```bash
NANO_CHROMIUM_PATH=/path/to/chromium python run.py
python report.py
```

`NANO_CHROMIUM_PATH` is optional if Playwright's bundled Chromium is installed.
`PLAYWRIGHT_MODULE_PATH` can point to an existing Playwright module. The runner
uses `dist/nano`, ports 3355/3356, the current Python interpreter, and one server
at a time. `--rows`, `--build-runs`, `--server-runs`, and `--contexts` configure
the workload. The default report expects the default 1,000-row breakdown.

## Optimization comparison

`nano` is the new standalone 0.3.0 executable; `nano_python` is its PyO3 host.
`nano_before` is the preserved 0.2.0 executable, rerun alongside the new versions
with the same fixture, browser, readiness checks and randomized scheduling.
`reflex` is the real production Reflex app. This rerun, rather than mixing old
and new measurement sessions, supplies the before/after results.

The new Rust compiler emits reusable SSR operations, constant expressions and
state/local dependency boundaries. The browser adopts SSR nodes, skips an equal
initial snapshot, shares unchanged row values and updates affected boundaries.
It uses eight delegated event listeners rather than per-row closures. Rust caches
computed result allocations, serializes borrowed snapshots/deltas and serves
content-addressed, compressed assets and gzip HTML. The browser client is syntax
minified by Rust at framework build time. See `OPTIMIZATIONS.md` for the audit
and explicit limits; no full Reflex ecosystem parity is claimed.

Raw samples: `benchmarks/lifecycle/results.json`; navigation CSV: `summary.csv`.
Source/binary SHA-256 provenance is recorded in the raw JSON. The downloadable
package also contains all three exported manifests and the frontend lockfile.
