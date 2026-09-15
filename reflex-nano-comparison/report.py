"""Build a comparison from the recovered release and new current-version data."""
import csv, hashlib, json, re, zipfile
from pathlib import Path
from statistics import median
from markdown_it import MarkdownIt
HERE=Path(__file__).resolve().parent
ROOT=HERE.parent/'reflex-nano'
b=json.loads((HERE/'backend-results.json').read_text())
w=json.loads((HERE/'browser-results.json').read_text())
old=json.loads((ROOT/'benchmarks/lifecycle/react-results.json').read_text())
assert b['completed'] and w['completed'] and old['completed']
assert len(b['runs'])==330 and sum(r['events'] for r in b['runs'])==148500
assert len(b['contracts'])==12 and all(len(r['checks'])==11 for r in b['contracts'])
assert len(w['runs'])==45
for r in w['runs']:
    assert len(r['navigations'])==2 and not r['errors']
    for n in r['navigations']:
        assert n['row_count']==r['rows'] and not n['errors']
        assert n['marks']['full_state_ms']<=n['marks']['ready_ms']<=n['marks']['paint_ready_ms']
        assert n['memory_hydrated']['server']['Pss_kib']>0
        if 'react' in r['kind']:
            t=n['native_hydration_stats']
            assert t['hydration']=='hydrateRoot' and not t['errors'] and not t['recoverableErrors']
        if n['phase']=='initial':
            assert len(n['warm_events']['scalar_ms'])==50 and len(n['warm_events']['toggle_ms'])==20
        else:assert n['last_row_event_verified']
release=HERE.parent/'recovered/reflex-nano-0.4.0.zip'
assert hashlib.sha256(release.read_bytes()).hexdigest()=='30a03453e1bba2bd7d97cf1a9185806fca0ac72d60ff4e4104796986d25eda38'
with zipfile.ZipFile(release) as z:
    for name,digest in old['sha256'].items():
        assert hashlib.sha256(z.read('reflex-nano/'+name)).hexdigest()==digest,name
for d in [b,w]:
    for name,digest in d['source_sha256'].items():
        assert hashlib.sha256((HERE.parent/name).read_bytes()).hexdigest()==digest,name
def table(headers,rows):
    return '\n'.join(['| '+' | '.join(headers)+' |','| '+' | '.join(['---']*len(headers))+' |',*['| '+' | '.join(map(str,r))+' |' for r in rows]])
def bm(k,rows,op,c,key):
    return median(r[key] for r in b['runs'] if r['framework']==k and r['rows']==rows and r['operation']==op and r['clients']==c)
def pct(v,p):
    v=sorted(v);i=(len(v)-1)*p;lo=int(i);hi=min(lo+1,len(v)-1);return v[lo]+(v[hi]-v[lo])*(i-lo)
def csvfile(name,rows):
    with (HERE/name).open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
latency=[];rates=[];tails=[];back=[]
for rows in [0,100,1000,10000]:
    for op in ['increment','increment_async','toggle']:
        if rows==0 and op=='toggle':continue
        latency.append([f'{rows:,}',op,*[f'{bm(k,rows,op,1,"median_ms"):.3f}' for k in ['nano','nano_python','reflex']],f'{bm("reflex",rows,op,1,"median_ms")/bm("nano",rows,op,1,"median_ms"):.2f}x'])
        rates.append([f'{rows:,}',op,*[f'{bm(k,rows,op,8,"events_per_second"):,.0f}' for k in ['nano','nano_python','reflex']],f'{bm("nano",rows,op,8,"events_per_second")/bm("reflex",rows,op,8,"events_per_second"):.2f}x'])
        for c in [1,8]:
            if rows>=1000:tails.append([f'{rows:,}',op,c,*[f'{bm(k,rows,op,c,"p95_ms"):.3f}' for k in ['nano','nano_python','reflex']]])
            for k in ['nano','nano_python','reflex']:
                back.append({'rows':rows,'operation':op,'clients':c,'kind':k,**{key:bm(k,rows,op,c,key) for key in ['median_ms','p95_ms','events_per_second']},
                    'received_bytes_per_event':median(r['received_bytes']/r['events'] for r in b['runs'] if r['framework']==k and r['rows']==rows and r['operation']==op and r['clients']==c)})
csvfile('backend-summary.csv',back)
kinds=['nano','nano_python','nano_react','nano_react_python','reflex']
labels=['Nano HTML / Rust','Nano HTML / PyO3','Nano React / Rust','Nano React / PyO3','Reflex']
def navs(k,rows):return [n for r in w['runs'] if r['kind']==k and r['rows']==rows for n in r['navigations'] if n['phase']=='initial']
mem=[];ui=[];updates=[]
for rows in [100,1000,10000]:
    for k,label in zip(kinds,labels):
        ns=navs(k,rows)
        item={'rows':rows,'kind':k,
            'server_pss_mib':median(n['memory_hydrated']['server']['Pss_kib']/1024 for n in ns),
            'server_rss_mib':median(n['memory_hydrated']['server']['Rss_kib']/1024 for n in ns),
            'v8_retained_mib':median(n['memory_hydrated']['browser_heap']['usedSize']/1024**2 for n in ns),
            'v8_after_updates_mib':median(n['memory_after_updates']['browser_heap']['usedSize']/1024**2 for n in ns)}
        for op in ['scalar','toggle']:
            item[op+'_median_ms']=median(median(n['warm_events'][op+'_ms']) for n in ns)
            item[op+'_p95_ms']=median(pct(n['warm_events'][op+'_ms'],.95) for n in ns)
        ui.append(item)
        mem.append([f'{rows:,}',label,*[f'{item[key]:.2f}' for key in ['server_pss_mib','server_rss_mib','v8_retained_mib','v8_after_updates_mib']]])
    for op in ['scalar','toggle']:
        updates.append([f'{rows:,}',op,*[f'{median(median(n["warm_events"][op+"_ms"]) for n in navs(k,rows)):.2f}' for k in kinds]])
csvfile('browser-summary.csv',ui)
summary={'nano_version':'0.4.0','reflex_version':'0.9.11','backend_events':148500,'backend_groups':330,'original_navigations':144,'additional_navigations':90,'extra_scalar_events':2250,'extra_toggle_events':900,'backend_summary':back,'browser_summary':ui}
(HERE/'summary.json').write_text(json.dumps(summary,indent=2))
prior=(ROOT/'docs/REACT-BENCHMARKS.md').read_text()
prior=prior.replace('# Nano 0.4: React versus direct HTML and Reflex','## Release lifecycle measurements',1).replace('\n## ','\n### ')
backend=(ROOT/'docs/BACKEND-OPTIMIZED.md').read_text().split('## Reproduce')[0]
tables=list(re.finditer(r'(?m)^\| .+(?:\n\| .+)*',backend))
newtables=[table(['Rows','Operation','Nano Rust','Nano PyO3','Reflex','Reflex / Nano latency'],latency),table(['Rows','Operation','Nano Rust','Nano PyO3','Reflex','Nano / Reflex throughput'],rates),table(['Rows','Operation','Clients','Nano Rust','Nano PyO3','Reflex'],tails)]
assert len(tables)==3
for match,new in reversed(list(zip(tables,newtables))):backend=backend[:match.start()]+new+backend[match.end():]
backend=backend.replace('Reflex Nano 0.3','Reflex Nano 0.4 — fresh run').replace('240 workload groups','330 workload groups').replace('162,000 measured events','148,500 measured events').replace('150 measured events','100 measured events')
backend=backend.replace('## Tail latency with 1,000 rows','## Tail latency with 1,000 and 10,000 rows')
backend=backend.replace('No compilers or browser tests ran during the final timing run.','The harness schedules workloads sequentially; shared-machine activity and other processes are not isolated or controlled.')
backend=backend.replace('optimized-results.json','backend-results.json').replace('optimized-summary.csv','backend-summary.csv')
backend=backend.replace('# WebSocket performance','## WebSocket performance',1).replace('\n## ','\n### ')
intro=(HERE/'introduction.md').read_text()
features=(HERE/'features.md').read_text()
extra=f"""
## Fresh repeated browser updates

Three processes per size/configuration, 50 scalar updates and 20 last-row toggles per page.
The complete DOM and computed result is verified after each event. Values are milliseconds,
medians of per-run medians. This includes transport, backend work and browser reconciliation.

{table(['Rows','Update',*labels],updates)}

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

{table(['Rows','Configuration','Server PSS','Server RSS','V8 after hydrate/event','V8 after updates'],mem)}

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

Python {b['environment']['python'].split()[0]}, {b['environment']['cpu']},
{b['environment']['platform']}, CPU quota {b['environment']['cpu_quota']}.
Chromium {w['runs'][0]['browser']}, 1280x900, loopback, no artificial CPU/network throttling.
These are five-process backend and three-process browser/memory experiments in a shared environment.

The immutable source release and its original source hashes were verified against the archive.
All new measured scripts, relevant sources, binaries and wheels were also hash-checked.
The report validates every expected sample count and recorded correctness condition.
The bundle includes the original release, raw JSON/CSV and reproduction scripts; see README.md.
"""
report=intro+'\n'+prior+'\n'+backend+'\n'+extra+'\n'+features
(HERE/'REPORT.md').write_text(report)
body=MarkdownIt('commonmark',{'html':False}).enable('table').render(report)
style="html{font-family:ui-sans-serif,system-ui,sans-serif;color:#182e3c;background:#eef3f6;line-height:1.65}body{margin:0}main{max-width:1200px;margin:auto;padding:45px 5vw 70px;background:white}h1{font-size:2.3rem;line-height:1.2;color:#123d52}h2{margin-top:3rem;border-top:2px solid #cbe0e8;padding-top:1rem}h3{margin-top:2rem}table{display:block;overflow:auto;width:100%;border-collapse:collapse;font-size:.85rem;font-variant-numeric:tabular-nums;margin:1.5rem 0}th,td{border:1px solid #d1dee5;padding:8px 11px;vertical-align:top;text-align:left}th{background:#163f54;color:white}tr:nth-child(even){background:#f0f6f8}a{color:#066f93}code,pre{background:#eef3f6}code{padding:2px 4px}pre{overflow:auto;padding:15px}nav{background:#163f54;color:white;padding:12px 5vw;display:flex;gap:20px;position:sticky;top:0}nav a{color:white;text-decoration:none}button{margin-left:auto;border:0;border-radius:5px;padding:7px 13px;cursor:pointer}@media(max-width:650px){main{padding:25px 16px}h1{font-size:1.8rem}table{font-size:.75rem}nav{position:static;flex-wrap:wrap}}@media print{nav{display:none}main{padding:0}table{display:table;font-size:8px}h2,h3{break-after:avoid}tr{break-inside:avoid}}"
doc='<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Nano / Reflex full comparison</title><style>'+style+'</style><nav><b>NANO 0.4 / REFLEX 0.9.11</b><a href="summary.json">Summary JSON</a><a href="backend-summary.csv">Backend CSV</a><a href="browser-summary.csv">Browser CSV</a><button onclick="print()">Print / PDF</button></nav><main>'+body+'</main></html>'
(HERE/'REPORT.html').write_text(doc)
print(json.dumps({k:v for k,v in summary.items() if not k.endswith('_summary')},indent=2))

