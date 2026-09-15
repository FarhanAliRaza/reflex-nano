"""Validate and summarize the recorded lifecycle experiment; no timing rerun."""
import csv
import json
from pathlib import Path
from statistics import median

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
r = json.loads((HERE/'results.json').read_text())
assert r['completed']
args = r['arguments']
assert len(r['compilations']) == len(args['rows']) * 2 * args['build_runs']
assert len(r['browser_runs']) == len(args['rows']) * len(args['frameworks']) * args['server_runs']
flat = []
for run in r['browser_runs']:
    assert not run['errors']
    assert len(run['navigations']) == args['contexts'] * 2
    for nav in run['navigations']:
        assert nav['row_count'] == run['rows'] and not nav['errors']
        assert nav['incoming_frames'] > 0
        assert nav['marks']['full_state_ms'] <= nav['marks']['ready_ms']
        assert nav['marks']['ready_ms'] <= nav['marks']['paint_ready_ms']
        if nav['phase'] == 'first_reload': assert nav['last_row_event_verified']
        flat.append({'kind': run['kind'], 'rows': run['rows'], 'run': run['run'],
            'repetition': nav['repetition'], 'phase': nav['phase'],
            **nav['marks'], 'ttfb_ms': nav['document']['responseStart'],
            'response_end_ms': nav['document']['responseEnd'],
            'post_response_ready_ms': nav['marks']['ready_ms']-nav['document']['responseEnd'],
            **{key: nav[key] for key in ['first_event_ms', 'http_transfer_bytes',
                'http_decoded_bytes', 'websocket_in_bytes', 'dom_nodes']}})
with (HERE/'summary.csv').open('w', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=sorted(set().union(*(v.keys() for v in flat))))
    writer.writeheader(); writer.writerows(flat)

def values(rows, kind, phase, metric):
    return [v[metric] for v in flat if v['rows']==rows and v['kind']==kind and v['phase']==phase]
def stat(v): return f'{median(v):.2f} [{min(v):.2f}–{max(v):.2f}]'
def table(headers, lines):
    return '\n'.join(['| '+' | '.join(headers)+' |', '| '+' | '.join(['---']*len(headers))+' |',
                      *['| '+' | '.join(map(str,line))+' |' for line in lines]])

build_lines=[]
for rows in args['rows']:
    nano=[v for v in r['compilations'] if v['kind']=='nano' and v['rows']==rows]
    reflex=[v for v in r['compilations'] if v['kind']=='reflex' and v['rows']==rows]
    build_lines.append([f'{rows:,}', stat([v['total_app_prepare_ms'] for v in nano]),
        stat([v['codegen_process_ms'] for v in reflex]),
        stat([v['production_build_process_ms'] for v in reflex]),
        stat([v['total_app_prepare_ms'] for v in reflex])])
browser_lines=[]
for rows in args['rows']:
    for phase in ['initial','first_reload']:
        browser_lines.append([f'{rows:,}',phase,
            *[stat(values(rows,kind,phase,'ready_ms')) for kind in args['frameworks']]])
detail_lines=[]
for kind in args['frameworks']:
    for phase in ['initial','first_reload']:
        detail_lines.append([kind,phase,*[f'{median(values(1000,kind,phase,metric)):.2f}'
            for metric in ['ttfb_ms','response_end_ms','full_state_ms','ready_ms','paint_ready_ms','first_event_ms']]])
startup_lines=[]
for rows in args['rows']:
    startup_lines.append([f'{rows:,}',*[stat([v['startup_ms'] for v in r['browser_runs']
        if v['rows']==rows and v['kind']==kind]) for kind in args['frameworks']]])
transfer_lines=[]
for rows in args['rows']:
    for kind in args['frameworks']:
        transfer_lines.append([f'{rows:,}',kind,*[f'{median(values(rows,kind,"initial",m))/1024:.1f}'
            for m in ['http_transfer_bytes','http_decoded_bytes','websocket_in_bytes']]])
text = f'''# Compile, first load, reload and full hydration

Measured {r['date_utc']} on **Nano 0.3.0**, the preserved **Nano 0.2.0** baseline, and **Reflex {r['environment']['packages']['reflex']}**.
All values below are milliseconds unless indicated; brackets show min–max.

## What was verified

Both production applications render the same 100, 1,000 or 10,000 rows. Every row
has an ID, label, done value, and working toggle button. The complete six-field
public state (`count`, `doubled`, `remaining`, `name`, `progress`, `items`) must
arrive over the real WebSocket. The harness validates **every item**, checks
every rendered row, and waits for the framework's connected/hydrated condition.
It then clicks Increment and checks both the changed count and computed double.
The first reload must restore that changed state; another increment and the last
row's toggle must work. All **{len(flat)} navigations passed**, with zero recorded
page errors. Nothing is virtualized or lazily omitted from this fixture.

Nano's schema, programs, storage, computed values and event execution are Rust.
The fixture binds native `Node` objects through PyO3 and exports a native manifest;
the Rust executable serves that manifest without a Python process. The Python
host reads the same manifest and releases the GIL while Rust serves it.

## App preparation and production compilation

{args['build_runs']} fresh process trials per framework/row count, randomized order. Dependencies
are already installed. Reflex runs its real compiler, production React Router/Vite
build, route prerendering and configured static compression. Each production build
cleans its previous bundle; code generation reuses the initialized output directory.
Nano constructs, exports, validates and renders the entire application using the
**prebuilt Rust engine**; it has no per-app JavaScript bundling step. Process startup
and shutdown are included in these wall-clock values.

{table(['Rows','Nano app preparation','Reflex import + codegen','Reflex production build','Reflex total'],build_lines)}

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

{args['server_runs']} independent server and browser processes per framework/size;
{args['contexts']} fresh browser contexts each, for {args['server_runs']*args['contexts']}
initial loads and first reloads per cell. Initial loads have fresh HTTP caches and
sessions. First reloads retain that tab's cache and session. “Reload” means a
browser page reload, not source-edit HMR or a server restart.

{table(['Rows','Navigation',*args['frameworks']],browser_lines)}

The 1,000-row timing breakdown below reports medians. TTFB, response end, full
WebSocket state, ready and paint-ready are all measured from navigation start.
Paint-ready waits two animation frames after ready. The final column is the
separate click-to-observed-DOM-change latency, using a programmatic DOM click;
it is not human input latency. Browser timestamps exclude Playwright polling time.

{table(['Framework','Navigation','TTFB','Response end','Full WS state','Ready','Paint-ready','First event'],detail_lines)}

## Cold process startup

Spawn-to-successful-HTTP-probe, before any page navigation. Native probes an existing
static asset; Reflex probes `/ping`. This includes native manifest loading/runtime
construction, or Reflex imports/backend page evaluation and the production static
server. No app compilation or browser launch is included. Probe interval is 5 ms;
Node process/fetch overhead makes very short native startup timings coarse.
The server filesystem cache is not flushed.

{table(['Rows',*args['frameworks']],startup_lines)}

## Initial-load transfer sizes

Median KiB. HTTP transfer is Resource Timing's transferred bytes (including its
header estimate); decoded bytes include decompressed bodies. WebSocket bytes are
received application-frame payloads through ready, without protocol framing.
Use these to distinguish data/asset costs from backend execution speed.

{table(['Rows','Framework','HTTP transferred KiB','HTTP decoded KiB','WS received KiB'],transfer_lines)}

## Environment and limits

- {r['environment']['cpu']}; {len(r['environment']['affinity'])} affinity-visible CPUs,
  cgroup quota `{r['environment']['cpu_quota']}` (8 CPU equivalents). One benchmark
  workload at a time; compilers do not overlap browser timing.
- Python {r['environment']['python'].split()[0]}, Node {r['environment']['node']},
  Chromium {r['browser_runs'][0]['browser']}; headless Linux, 1280×900, no network or
  CPU throttling. Both frontend and backend are on loopback.
- React {r['environment']['frontend_packages']['react']}, React Router
  {r['environment']['frontend_packages']['react-router']}, Vite
  {r['environment']['frontend_packages']['vite']}, Granian
  {r['environment']['packages']['granian']}. See raw JSON and frontend lockfile.
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
'''
(ROOT/'docs/LIFECYCLE-BENCHMARKS.md').write_text(text)
print(f'Validated {len(flat)} navigations; wrote lifecycle report and CSV')
