"""Validate and report the 0.4 dual-renderer experiment, without rerunning it."""
import csv
import gzip
import hashlib
import json
from pathlib import Path
from statistics import median

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
r = json.loads((HERE/'react-results.json').read_text())
assert r['completed']
a = r['arguments']
assert r['environment']['packages']['reflex-nano'] == '0.4.0'
assert len(r['compilations']) == len(a['rows'])*3*a['build_runs']
assert len(r['browser_runs']) == len(a['rows'])*len(a['frameworks'])*a['server_runs']
flat = []
for run in r['browser_runs']:
    assert not run['errors']
    assert len(run['navigations']) == a['contexts']*2
    for nav in run['navigations']:
        assert nav['row_count'] == run['rows'] and not nav['errors']
        assert nav['incoming_frames'] > 0 and nav['first_event_ms'] > 0
        assert nav['marks']['full_state_ms'] <= nav['marks']['ready_ms'] <= nav['marks']['paint_ready_ms']
        if nav['phase'] == 'first_reload':
            assert nav['last_row_event_verified']
        if 'react' in run['kind']:
            stats = nav['native_hydration_stats']
            assert stats['renderer'] == 'react' and stats['hydration'] == 'hydrateRoot'
            assert not stats['recoverableErrors'] and not stats['errors']
        flat.append({'kind': run['kind'], 'rows': run['rows'], 'run': run['run'],
            'phase': nav['phase'], 'repetition': nav['repetition'], **nav['marks'],
            'ttfb_ms': nav['document']['responseStart'],
            'response_end_ms': nav['document']['responseEnd'],
            **{k: nav[k] for k in ['first_event_ms', 'http_transfer_bytes',
                'http_decoded_bytes', 'websocket_in_bytes', 'dom_nodes']}})
for name, expected in r['sha256'].items():
    assert hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == expected, name
with (HERE/'react-summary.csv').open('w', newline='') as f:
    writer = csv.DictWriter(f, fieldnames=sorted(set().union(*(v.keys() for v in flat))))
    writer.writeheader(); writer.writerows(flat)

names = {'nano':'Nano HTML / Rust', 'nano_react':'Nano React / Rust',
    'nano_react_python':'Nano React / PyO3', 'reflex':'Reflex'}
def table(headers, rows):
    return '\n'.join(['| '+' | '.join(headers)+' |', '| '+' | '.join(['---']*len(headers))+' |',
        *['| '+' | '.join(map(str,row))+' |' for row in rows]])
def values(rows, kind, phase, key):
    return [v[key] for v in flat if v['rows']==rows and v['kind']==kind and v['phase']==phase]
def stat(values):
    return f'{median(values):.1f} [{min(values):.1f}–{max(values):.1f}]'
def m(rows,kind,phase,key='ready_ms'):
    return median(values(rows,kind,phase,key))

ready = [[f'{rows:,}',phase,*[stat(values(rows,kind,phase,'ready_ms')) for kind in a['frameworks']]]
    for rows in a['rows'] for phase in ['initial','first_reload']]
builds = [[f'{rows:,}',*[stat([v['total_app_prepare_ms'] for v in r['compilations']
    if v['rows']==rows and v['kind']==kind]) for kind in ['nano','nano_react','reflex']]] for rows in a['rows']]
events = [[f'{rows:,}',phase,*[stat(values(rows,kind,phase,'first_event_ms')) for kind in a['frameworks']]]
    for rows in a['rows'] for phase in ['initial','first_reload']]
details = [[names[kind],phase,*[f'{m(1000,kind,phase,key):.1f}' for key in
    ['ttfb_ms','response_end_ms','full_state_ms','ready_ms','paint_ready_ms']]]
    for kind in a['frameworks'] for phase in ['initial','first_reload']]
starts = [[f'{rows:,}',*[stat([v['startup_ms'] for v in r['browser_runs'] if v['rows']==rows and v['kind']==kind])
    for kind in a['frameworks']]] for rows in a['rows']]
transfers = [[f'{rows:,}',names[kind],*[f'{m(rows,kind,"initial",key)/1024:.1f}' for key in
    ['http_transfer_bytes','http_decoded_bytes','websocket_in_bytes']]] for rows in a['rows'] for kind in a['frameworks']]
assets = [[p.name,f'{p.stat().st_size/1024:.1f}',f'{len(gzip.compress(p.read_bytes(),mtime=0))/1024:.1f}']
    for p in sorted((ROOT/'crates/nano-core/frontend-dist').glob('*')) if p.suffix in {'.js','.css'}]
source_build = json.loads((ROOT/'docs/build-timings.json').read_text())
frontend_build = json.loads((ROOT/'docs/frontend-build-timings.json').read_text())
source_rows = [[v['stage'],f"{v['seconds']:.3f}"] for v in source_build['runs']]
source_rows.append(['React frontend build, median of 3',f"{median(v['seconds'] for v in frontend_build['runs']):.3f}"])

report = f'''# Nano 0.4: React versus direct HTML and Reflex

Measured {r['date_utc']} using Nano 0.4.0 and production Reflex
{r['environment']['packages']['reflex']}. All table values are milliseconds
unless marked otherwise. Cells report **median [minimum–maximum]**.

For 1,000 rows, Nano React/Rust reached complete state + hydrated DOM in
**{m(1000,'nano_react','initial'):.1f} ms initially** and
**{m(1000,'nano_react','first_reload'):.1f} ms on first reload**.
Reflex took {m(1000,'reflex','initial'):.1f} and {m(1000,'reflex','first_reload'):.1f} ms:
ratios of {m(1000,'reflex','initial')/m(1000,'nano_react','initial'):.2f}× and
{m(1000,'reflex','first_reload')/m(1000,'nano_react','first_reload'):.2f}×, respectively.
Direct HTML remains the lighter option: {m(1000,'nano','initial'):.1f} and
{m(1000,'nano','first_reload'):.1f} ms for the same conditions.
At 10,000 rows the React difference narrows: Nano React/Rust was
{m(10000,'nano_react','initial'):.1f} ms initially and {m(10000,'nano_react','first_reload'):.1f} ms on reload,
versus Reflex's {m(10000,'reflex','initial'):.1f} and {m(10000,'reflex','first_reload'):.1f} ms.
The reload median is slightly worse, with overlapping observed ranges. React
mode does not deliver a consistent large-load hydration speedup in this test.
These are local fixture results, not universal framework or language speedups.

## What was timed and verified

All **{len(flat)} navigations passed** across 100, 1,000 and 10,000 rows.
Each row has the same ID, label, done value and working toggle button. There is
no list virtualization. The complete public state must arrive over a real
WebSocket; the probe validates every item and every rendered row. Nano 0.4 must
also report `ready` after its renderer commits, and be connected. React mode
must use real `hydrateRoot`, with no recorded recoverable hydration errors.
Reflex must report its framework hydration condition. The initial click changes
both the counter and computed double. Reload retains that state; another click
and the last row toggle must work. Zero recorded page errors.

One workload runs at a time, with no compiler overlapping browser timings.
There are {a['server_runs']} fresh server/browser processes per framework/size,
{a['contexts']} fresh contexts per process: {a['server_runs']*a['contexts']} initial
loads and six first reloads per cell. Trials use deterministic randomized order.
Initial loads have fresh browser HTTP caches and sessions. Reload retains them.
Reload means document reload, not source-edit HMR or restarting the server.
Small sample counts and shared-container variability limit fine comparisons.

The Rust and PyO3 React hosts run the same native manifest/backend and bundled
React adapter. PyO3 releases the GIL while Rust serves. All schemas, authoritative
state, computed values and event programs are native in both hosts.

## Complete state and hydrated DOM, from navigation start

{table(['Rows','Phase',*[names[k] for k in a['frameworks']]],ready)}

This condition includes document delivery, parsing, JavaScript startup, WebSocket
state transfer and hydration. It is not an isolated React hydration function
timer. In particular, Nano also does Rust SSR/compression on the request path.

The 1,000-row median breakdown below separates these milestones. Paint-ready
waits two animation frames after the validated ready condition.

{table(['Framework','Phase','TTFB','Response end','Full WS state','Ready','Paint-ready'],details)}

## First event: click to observed DOM update

Programmatic DOM click, with both count and computed double validated. This
includes WebSocket transit, Rust/Python backend execution and browser update.

{table(['Rows','Phase',*[names[k] for k in a['frameworks']]],events)}

## Per-application preparation / compilation

{a['build_runs']} fresh process trials per framework/size, with dependencies
installed. Process startup/shutdown is included. Nano imports bindings,
constructs/exports native definitions, validates/builds the app and renders all
rows using the prebuilt engine. Both Nano modes use a prebuilt generic adapter;
there is no per-app JavaScript bundle build. Reflex runs its real Python
compiler, production React Router/Vite build, route prerendering and compression.
Reflex's stock frontend initializer normalizes package.json outside the timer
to preserve its existing dependency-installation cache.

{table(['Rows','Nano HTML','Nano React','Reflex total'],builds)}

This compares app preparation with those architectures; it is not a Rust versus
Python compiler benchmark. Changing the registered React package set or adapter
requires a separate frontend and Rust build. Those costs are measured below.

## Framework build costs (seconds)

{table(['Stage','Seconds'],source_rows)}

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

{table(['Rows',*[names[k] for k in a['frameworks']]],starts)}

## Transfer sizes and React component costs

Median initial-load KiB. HTTP transfer uses Resource Timing (including its header
estimate); decoded bytes are decompressed bodies. WebSocket bytes are received
application payloads through readiness, excluding framing.

{table(['Rows','Framework','HTTP transferred KiB','HTTP decoded KiB','WS received KiB'],transfers)}

The timed fixture uses native HTML elements in both React frontends. Radix is
tested functionally in a separate fixture; its component/CSS chunk is lazy and
is not downloaded by this benchmark. Direct HTML pages download no React.
The following standalone assets show the additional cost when a page uses the
bundled component registry. Gzip sizes here use Python gzip; actual wire bytes
above come from the Rust server.

{table(['React asset','Decoded KiB','Gzip KiB'],assets)}

Third-party React components hydrate an explicit Rust-rendered HTML fallback,
then mount their interactive version in the browser. Native HTML elements use
real React hydration over the Rust SSR output. No claim of arbitrary third-party
React SSR, React Server Components or full Reflex wrapper parity is made.

## Environment, provenance and reproduction

- CPU: {r['environment']['cpu']}; cgroup quota `{r['environment']['cpu_quota']}`.
- Python {r['environment']['python'].split()[0]}, Node {r['environment']['node']},
  Rust `{r['environment']['rustc']}`, Chromium {r['browser_runs'][0]['browser']}.
- Headless Linux, 1280×900, loopback frontend/backend, no CPU/network throttling.
- Nano React 19.2.8; Reflex React {r['environment']['frontend_packages']['react']}.
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
NANO_CHROMIUM_PATH=/path/to/chromium python benchmarks/lifecycle/run.py \\
  --rows 100 1000 10000 --build-runs 3 --server-runs 3 --contexts 2 \\
  --frameworks nano nano_react nano_react_python reflex --output react-results.json
python benchmarks/lifecycle/react_report.py
```

Install the pinned benchmark Python/frontend dependencies first as described in
the existing lifecycle guide. The 0.3 lifecycle and backend results remain
historical reports; they have not been relabeled as 0.4 measurements.
'''
(ROOT/'docs/REACT-BENCHMARKS.md').write_text(report)
summary = {'version':'0.4.0','validated_navigations':len(flat),'errors':[],
    'median_1000':{kind:{phase:{key:m(1000,kind,phase,key) for key in ['ready_ms','first_event_ms']}
        for phase in ['initial','first_reload']} for kind in a['frameworks']}}
(ROOT/'docs/react-benchmark-validation.json').write_text(json.dumps(summary,indent=2))
print(json.dumps(summary,indent=2))
