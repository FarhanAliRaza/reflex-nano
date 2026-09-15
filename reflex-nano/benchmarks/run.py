"""Matched event-to-observed-state benchmark over real local WebSockets."""
import argparse
import asyncio
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import shutil
import time
from contextlib import AsyncExitStack

from driver import Backend,Client,HERE,contract,initial


def percentile(values,p):
    data=sorted(values);index=(len(data)-1)*p;lo=int(index);hi=min(lo+1,len(data)-1)
    return data[lo]+(data[hi]-data[lo])*(index-lo)


async def workload(client,operation,n,warmup,barrier):
    expected=initial(client.backend.rows)
    latency=[]
    async def step(i):
        if operation in ('increment','increment_async'):
            expected['count']+=1;expected['doubled']=expected['count']*2
            target=expected['count'];double=target*2
            args=[1];predicate=lambda s:s.get('count')==target and s.get('doubled')==double
        elif operation=='toggle':
            index=i%client.backend.rows;item=expected['items'][index];item['done']=not item['done']
            expected['remaining']+=-1 if item['done'] else 1
            done=item['done'];remaining=expected['remaining']
            args=[index];predicate=lambda s:s.get('remaining')==remaining and s['items'][index]['done']==done
        else:raise ValueError(operation)
        return await client.event(operation,args,predicate)
    for i in range(warmup):await step(i)
    await barrier.wait()
    before=(client.sent_bytes,client.received_bytes)
    begin=time.perf_counter()
    for i in range(n):latency.append(await step(i+warmup))
    duration=time.perf_counter()-begin
    assert client.public()==expected,(operation,client.public(),expected)
    return dict(latency_ms=latency,duration_s=duration,started=begin,ended=begin+duration,
        sent_bytes=client.sent_bytes-before[0],received_bytes=client.received_bytes-before[1])


async def measure(backend,operation,clients,n,warmup):
    async with AsyncExitStack() as stack:
        peers=[await stack.enter_async_context(Client(backend)) for _ in range(clients)]
        barrier=asyncio.Barrier(clients)
        results=await asyncio.gather(*(workload(c,operation,n,warmup,barrier) for c in peers))
    latency=[v for r in results for v in r['latency_ms']]
    duration=max(r['ended'] for r in results)-min(r['started'] for r in results)
    return dict(operation=operation,clients=clients,events=n*clients,
        median_ms=statistics.median(latency),p95_ms=percentile(latency,.95),
        events_per_second=n*clients/duration,duration_s=duration,
        sent_bytes=sum(r['sent_bytes'] for r in results),received_bytes=sum(r['received_bytes'] for r in results),
        latency_ms=latency)


def environment():
    packages={p:importlib.metadata.version(p) for p in ('reflex','reflex-nano','aiohttp','granian','websockets','python-socketio')}
    cpu=Path('/proc/cpuinfo').read_text().split('model name\t: ')[-1].splitlines()[0]
    rustc=shutil.which('rustc') or str(Path.home()/'.cargo/bin/rustc')
    return dict(python=platform.python_version(),platform=platform.platform(),cpu=cpu,
        rustc=subprocess.check_output([rustc,'--version'],cwd=HERE.parent,text=True).strip(),
        affinity=list(os.sched_getaffinity(0)),cpu_max=Path('/sys/fs/cgroup/cpu.max').read_text().strip() if Path('/sys/fs/cgroup/cpu.max').exists() else None,
        packages=packages,source_sha256={str(p.relative_to(HERE.parent)):hashlib.sha256(p.read_bytes()).hexdigest()
            for p in [*HERE.glob('*.py'),*HERE.parent.joinpath('crates').rglob('*.rs'),*HERE.parent.joinpath('examples/rust').glob('*.rs'),
                      HERE.parent/'Cargo.lock',HERE.parent/'Cargo.toml',HERE.parent/'pyproject.toml',
                      HERE.parent/'crates/nano-core/src/client.js', HERE.parent/'dist/nano',
                      *HERE.parent.joinpath('dist').glob('*0.3.0*.whl')]})


async def main(args):
    output=dict(scope='Warm local WebSocket event-to-observed-state; backend framework comparison, not whole-app or language-only speedup',
        environment=environment(),settings=vars(args),runs=[],contracts=[])
    rng=random.Random(args.seed)
    for repeat in range(args.repeats):
        for rows in args.rows:
            order=list(args.frameworks);rng.shuffle(order)
            for kind in order:
                async with Backend('nano' if kind=='nano_python' else kind,rows,python_host=kind=='nano_python') as backend:
                    if repeat==0:
                        output['contracts'].append(dict(framework=kind,rows=rows,checks=await contract(backend)))
                    for operation in ('increment','increment_async','toggle'):
                        if operation=='toggle' and rows==0:continue
                        for clients in (1,args.clients):
                            result=await measure(backend,operation,clients,args.events,args.warmup)
                            result.update(framework=kind,rows=rows,repeat=repeat,backend_info=backend.info)
                            output['runs'].append(result)
                            print(f'{kind:6} rows={rows:4} {operation:16} c={clients} median={result["median_ms"]:.3f}ms p95={result["p95_ms"]:.3f}ms rate={result["events_per_second"]:.0f}/s',flush=True)
                Path(args.output).write_text(json.dumps(output,indent=2)+'\n')
    summary=[]
    for rows in args.rows:
        for operation in ('increment','increment_async','toggle'):
            for clients in (1,args.clients):
                entry=dict(rows=rows,operation=operation,clients=clients)
                for kind in args.frameworks:
                    runs=[r for r in output['runs'] if r['framework']==kind and r['rows']==rows and r['operation']==operation and r['clients']==clients]
                    if not runs:continue
                    entry[kind]={k:statistics.median(r[k] for r in runs) for k in ('median_ms','p95_ms','events_per_second')}
                    entry[kind]['received_bytes_per_event']=statistics.median(r['received_bytes']/r['events'] for r in runs)
                    entry[kind]['run_medians_ms']=[r['median_ms'] for r in runs]
                if 'nano' in entry and 'reflex' in entry:
                    entry['latency_ratio_reflex_over_nano']=entry['reflex']['median_ms']/entry['nano']['median_ms']
                    entry['throughput_ratio_nano_over_reflex']=entry['nano']['events_per_second']/entry['reflex']['events_per_second']
                    if 'nano_python' in entry:entry['latency_ratio_reflex_over_nano_python']=entry['reflex']['median_ms']/entry['nano_python']['median_ms']
                    summary.append(entry)
    output['summary']=summary
    Path(args.output).write_text(json.dumps(output,indent=2)+'\n')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rows',nargs='+',type=int,default=[0,100,1000])
    parser.add_argument('--frameworks',nargs='+',choices=['nano','nano_python','reflex'],default=['nano','nano_python','reflex'])
    parser.add_argument('--events',type=int,default=150)
    parser.add_argument('--warmup',type=int,default=20)
    parser.add_argument('--clients',type=int,default=8)
    parser.add_argument('--repeats',type=int,default=5)
    parser.add_argument('--seed',type=int,default=20260915)
    parser.add_argument('--output',default=str(HERE/'results.json'))
    asyncio.run(main(parser.parse_args()))
