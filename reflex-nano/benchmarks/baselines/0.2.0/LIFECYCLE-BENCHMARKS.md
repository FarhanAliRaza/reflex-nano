# Compile, first load, reload and full hydration

Measured 2026-09-15T09:24:02Z on the existing **Nano 0.2.0** engine and **Reflex 0.9.11**.
All values below are milliseconds unless indicated; brackets show min–max.

## What was verified

Both production applications render the same 100, 1,000 or 10,000 rows. Every row
has an ID, label, done value, and working toggle button. The complete six-field
public state (`count`, `doubled`, `remaining`, `name`, `progress`, `items`) must
arrive over the real WebSocket. The harness validates **every item**, checks
every rendered row, and waits for the framework's connected/hydrated condition.
It then clicks Increment and checks both the changed count and computed double.
The first reload must restore that changed state; another increment and the last
row's toggle must work. All **180 navigations passed**, with zero recorded
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
| 100 | 23.27 [22.50–24.37] | 472.74 [452.68–616.74] | 1923.66 [1855.25–2008.39] | 2446.61 [2376.34–2483.20] |
| 1,000 | 28.61 [27.54–31.55] | 469.46 [456.79–500.09] | 1941.27 [1878.09–2074.03] | 2398.07 [2346.89–2553.19] |
| 10,000 | 70.06 [67.20–71.80] | 523.88 [493.98–662.24] | 2104.32 [2039.03–2152.86] | 2628.20 [2556.91–2809.64] |

These are application-to-deployable-output costs, **not a Rust-vs-Python compiler
speed claim**. The Rust engine and PyO3 wheel build could not be measured in this
resumed environment: the compiler was absent and its download timed out. No zero
or estimated source-build timing is substituted. Dependency/toolchain installation,
`cargo build`, wheel construction, and a clean OS filesystem cache are excluded.

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

| Rows | Navigation | Nano Rust | Nano PyO3 host | Reflex |
| --- | --- | --- | --- | --- |
| 100 | initial | 42.70 [39.80–53.30] | 46.40 [39.00–52.80] | 82.80 [76.30–90.90] |
| 100 | first_reload | 42.05 [33.10–45.60] | 41.20 [37.70–45.90] | 52.00 [48.60–59.00] |
| 1,000 | initial | 171.30 [129.40–200.30] | 171.70 [132.60–194.60] | 154.20 [128.70–176.20] |
| 1,000 | first_reload | 155.45 [148.70–169.20] | 164.85 [150.80–229.90] | 123.40 [108.00–159.40] |
| 10,000 | initial | 1357.55 [1270.00–1433.80] | 1402.75 [1303.00–1443.50] | 749.35 [698.10–786.00] |
| 10,000 | first_reload | 1405.60 [1340.20–2037.40] | 1374.50 [1312.00–1493.30] | 660.65 [634.30–887.80] |

The 1,000-row timing breakdown below reports medians. TTFB, response end, full
WebSocket state, ready and paint-ready are all measured from navigation start.
Paint-ready waits two animation frames after ready. The final column is the
separate click-to-observed-DOM-change latency, using a programmatic DOM click;
it is not human input latency. Browser timestamps exclude Playwright polling time.

| Framework | Navigation | TTFB | Response end | Full WS state | Ready | Paint-ready | First event |
| --- | --- | --- | --- | --- | --- | --- | --- |
| nano | initial | 3.90 | 5.20 | 142.95 | 171.30 | 176.60 | 23.00 |
| nano | first_reload | 3.45 | 4.85 | 132.60 | 155.45 | 158.80 | 21.60 |
| nano_python | initial | 4.20 | 5.65 | 142.40 | 171.70 | 172.65 | 28.00 |
| nano_python | first_reload | 3.20 | 4.30 | 139.30 | 164.85 | 166.70 | 22.10 |
| reflex | initial | 5.10 | 6.55 | 141.90 | 154.20 | 177.20 | 10.40 |
| reflex | first_reload | 2.70 | 3.70 | 109.85 | 123.40 | 143.85 | 6.70 |

## Cold process startup

Spawn-to-successful-HTTP-probe, before any page navigation. Native probes an existing
static asset; Reflex probes `/ping`. This includes native manifest loading/runtime
construction, or Reflex imports/backend page evaluation and the production static
server. No app compilation or browser launch is included. Probe interval is 5 ms;
Node process/fetch overhead makes very short native startup timings coarse.
The server filesystem cache is not flushed.

| Rows | Nano Rust | Nano PyO3 host | Reflex |
| --- | --- | --- | --- |
| 100 | 18.68 [17.57–20.37] | 38.19 [31.27–39.12] | 463.83 [458.30–501.62] |
| 1,000 | 20.67 [17.54–27.18] | 39.82 [36.00–43.97] | 499.66 [474.81–504.73] |
| 10,000 | 35.11 [29.68–38.06] | 54.70 [48.37–59.59] | 556.36 [536.65–598.92] |

## Initial-load transfer sizes

Median KiB. HTTP transfer is Resource Timing's transferred bytes (including its
header estimate); decoded bytes include decompressed bodies. WebSocket bytes are
received application-frame payloads through ready, without protocol framing.
Use these to distinguish data/asset costs from backend execution speed.

| Rows | Framework | HTTP transferred KiB | HTTP decoded KiB | WS received KiB |
| --- | --- | --- | --- | --- |
| 100 | nano | 41.3 | 40.4 | 4.2 |
| 100 | nano_python | 41.3 | 40.4 | 4.2 |
| 100 | reflex | 162.4 | 514.4 | 6.4 |
| 1,000 | nano | 227.6 | 226.7 | 41.9 |
| 1,000 | nano_python | 227.6 | 226.7 | 41.9 |
| 1,000 | reflex | 174.0 | 692.8 | 44.2 |
| 10,000 | nano | 2126.1 | 2125.2 | 437.5 |
| 10,000 | nano_python | 2126.1 | 2125.2 | 437.5 |
| 10,000 | reflex | 266.9 | 2512.2 | 439.7 |

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
- The earlier warm backend experiment in `../results.json` is unchanged and
  must not be combined statistically with these browser measurements.

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

## What the measurements expose

App preparation benefits from Nano's reusable prebuilt engine. Browser costs are
different: at 1,000 and 10,000 rows this version's full readiness and reload are
slower than Reflex. The browser source explains a likely optimization target:
`previous` starts empty, so the first `render()` creates nodes rather than adopting
the server-rendered DOM. The initial WebSocket snapshot calls `render()` again,
and subsequent state updates materialize and reconcile the full tree, including
unchanged lists. This code inspection identifies work to reduce; the experiment
does not isolate how much each operation contributes to the measured difference.
Native backend event speed alone does not make the full frontend faster.

Raw samples: `benchmarks/lifecycle/results.json`; navigation CSV: `summary.csv`.
Source/binary SHA-256 provenance is recorded in the raw JSON. The downloadable
package also contains all three exported manifests and the frontend lockfile.
