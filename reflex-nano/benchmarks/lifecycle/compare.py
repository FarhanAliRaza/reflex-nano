"""Create the concise release comparison from validated raw measurements."""
import json
from pathlib import Path
from statistics import median

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
r=json.loads((HERE/'results.json').read_text())
b=json.loads((ROOT/'docs/build-timings.json').read_text())
assert r['completed'] and b['completed']
flat=[{'kind':v['kind'],'rows':v['rows'],**n} for v in r['browser_runs'] for n in v['navigations']]
def m(rows,kind,phase,metric):
    values=[n for n in flat if n['rows']==rows and n['kind']==kind and n['phase']==phase]
    return median(n['marks'][metric] if metric in n['marks'] else n[metric] for n in values)
def table(headers,rows):
    return '\n'.join(['| '+' | '.join(headers)+' |','| '+' | '.join(['---']*len(headers))+' |',
                     *['| '+' | '.join(map(str,row))+' |' for row in rows]])
readiness=[];events=[];transfers=[];compiles=[]
for rows in r['arguments']['rows']:
    for phase in ['initial','first_reload']:
        before=m(rows,'nano_before',phase,'ready_ms');after=m(rows,'nano',phase,'ready_ms')
        reflex=m(rows,'reflex',phase,'ready_ms')
        readiness.append([f'{rows:,}',phase,f'{before:.1f}',f'{after:.1f}',f'{reflex:.1f}',
                          f'{before/after:.2f}×',f'{reflex/after:.2f}×'])
    values=[m(rows,k,'initial','first_event_ms') for k in ['nano_before','nano','nano_python','reflex']]
    events.append([f'{rows:,}',*[f'{v:.2f}' for v in values],f'{values[0]/values[1]:.1f}×'])
    values=[m(rows,k,'initial','http_transfer_bytes')/1024 for k in ['nano_before','nano','reflex']]
    transfers.append([f'{rows:,}',*[f'{v:.1f}' for v in values],f'{(1-values[1]/values[0])*100:.1f}%'])
    values=[median(v['total_app_prepare_ms'] for v in r['compilations'] if v['rows']==rows and v['kind']==kind)
            for kind in ['nano','reflex']]
    compiles.append([f'{rows:,}',*[f'{v:.2f}' for v in values],f'{values[1]/values[0]:.1f}×'])
stats=[n for n in flat if n['kind']=='nano' and n['rows']==10000 and n['phase']=='initial']
assert all(n['native_hydration_stats']['createdNodes']==0 for n in stats)
assert all(n['native_event_stats']['last']['visited']==5 for n in stats)
text=f'''# Nano 0.3 optimization results

Measured {r['date_utc']}. Same-machine randomized rerun: Nano 0.2.0, Nano 0.3.0
standalone Rust, Nano 0.3.0 through PyO3, and Reflex {r['environment']['packages']['reflex']} production.
All {len(flat)} navigations passed full WebSocket state, every-row DOM and real
event checks. No rows are virtualized. Values below are medians in milliseconds
unless stated. Each browser cell has {r['arguments']['server_runs']} independent
process runs × {r['arguments']['contexts']} fresh contexts. Full ranges, PyO3
readiness, startup and timing breakdown are in `LIFECYCLE-BENCHMARKS.md`.

## Initial readiness and first reload

{table(['Rows','Navigation','Nano 0.2','Nano 0.3 Rust','Reflex','Old / new Nano','Reflex / new Nano'],readiness)}

Ratios above 1 mean Nano 0.3 is faster. This measures navigation to verified full
state, rendered DOM and connected/hydrated framework readiness. Paint-ready is
also recorded separately. Reload retains the browser cache and session.

## First counter event after initial hydration

{table(['Rows','Nano 0.2','Nano 0.3 Rust','Nano 0.3 PyO3','Reflex','Old / new Nano'],events)}

This is a real programmatic DOM click through WebSocket execution and back to
the observed DOM update, including the computed double. At 10,000 rows Nano
adopted {int(median(n['native_hydration_stats']['hydratedNodes'] for n in stats)):,}
existing nodes and created zero replacement nodes. A counter update visited
five render-plan nodes and changed two text nodes, regardless of list size.
These counts demonstrate skipped list work; they do not imply all events are O(1).

## Initial HTTP transfer

{table(['Rows','Nano 0.2 KiB','Nano 0.3 KiB','Reflex KiB','Nano reduction'],transfers)}

Includes document and browser assets, using Resource Timing transfer sizes.
WebSocket snapshot bytes are reported separately in the full lifecycle report.

## Application preparation / production compilation

{table(['Rows','Nano app preparation','Reflex total build','Reflex / Nano'],compiles)}

These use an already-built Rust engine and already-installed dependencies.
Nano constructs, exports, validates and renders the app. Reflex performs actual
code generation, clean production bundling, route prerendering and compression.
This is a comparison of deployment workflows, not language compiler speed.

The final Rust source build was measured separately, once per stage:

{table(['Build stage','Seconds'],[[v['stage'],f"{v['seconds']:.3f}"] for v in b['runs']])}

The first stage uses an empty target directory and cached registry sources;
the second is an unchanged cached rebuild. Wheel construction follows the native
build and reuses its compiled dependencies where applicable. Thin LTO, one
codegen unit, one Cargo job; toolchain installation and downloads excluded.

## Interpretation and scope

The complete optimization inventory is in `OPTIMIZATIONS.md`. Rust owns state,
schema, events, computed expressions, scheduling and render/asset compilation.
Python only binds native types; browser DOM operations use the JS adapter.
This release ports applicable core optimization mechanisms and does not claim
complete Reflex/React ecosystem parity.

The comparison is a combined implementation change, with no per-optimization
ablation. The old binary used Rust 1.98.1 and this release uses working Rust 1.97.0;
the compiler change is a confound. Loopback/headless measurements do not establish
WAN performance, memory savings, distributed scalability or a pure language
speedup. Raw samples and source/binary hashes are in
`benchmarks/lifecycle/results.json`; build hashes are in `docs/build-timings.json`.
'''
(ROOT/'docs/OPTIMIZATION-RESULTS.md').write_text(text)
print(text)
