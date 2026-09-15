"""Fresh Nano 0.4/Reflex comparison using the release's unmodified WS clients."""
import argparse
import asyncio
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import random
import sys
import time

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent/'reflex-nano'
sys.path.insert(0,str(ROOT/'benchmarks'))
from driver import Backend, contract
from run import measure

def process_memory(pid):
    record={}
    for line in Path(f'/proc/{pid}/smaps_rollup').read_text().splitlines():
        if ':' in line:
            key,value=line.split(':',1)
            if key in ['Rss','Pss','Private_Clean','Private_Dirty']:
                record[key+'_kib']=int(value.split()[0])
    return record

async def main(args):
    os.environ['REFLEX_SKIP_COMPILE']='true'
    os.environ['NANO_RUNNER']=str(ROOT/'dist/nano')
    result={'version':'0.4.0','date_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),
        'settings':vars(args),'environment':{'python':sys.version,'platform':platform.platform(),
        'cpu':next(x.split(':',1)[1].strip() for x in Path('/proc/cpuinfo').read_text().splitlines() if x.startswith('model name')),
        'affinity':sorted(os.sched_getaffinity(0)),'cpu_quota':Path('/sys/fs/cgroup/cpu.max').read_text().strip(),
        'packages':{name:importlib.metadata.version(name) for name in ['reflex','reflex-base','reflex-nano','aiohttp','granian','python-socketio']},
        'rust_build':json.loads((ROOT/'docs/build-timings.json').read_text())['rustc']},
        'contracts':[],'runs':[],'completed':False}
    rng=random.Random(args.seed)
    output=HERE/args.output
    def save():output.write_text(json.dumps(result,indent=2))
    save()
    for repeat in range(args.repeats):
        for rows in args.rows:
            kinds=['nano','nano_python','reflex'];rng.shuffle(kinds)
            for kind in kinds:
                async with Backend('nano' if kind=='nano_python' else kind,rows,
                                   python_host=kind=='nano_python') as backend:
                    if repeat==0:
                        result['contracts'].append({'framework':kind,'rows':rows,'checks':await contract(backend)})
                    for operation in ['increment','increment_async','toggle']:
                        if not rows and operation=='toggle':continue
                        for clients in [1,args.clients]:
                            item=await measure(backend,operation,clients,args.events,args.warmup)
                            item.update(framework=kind,rows=rows,repeat=repeat,backend_info=backend.info)
                            result['runs'].append(item);save()
                    print(f'Completed {kind} rows={rows} run={repeat+1}/{args.repeats}; groups={len(result["runs"])}',flush=True)
    sources=[HERE/'backend.py',ROOT/'dist/nano',*ROOT.joinpath('dist').glob('*0.4.0*.whl'),
        ROOT/'benchmarks/driver.py',ROOT/'benchmarks/run.py',ROOT/'benchmarks/shared.py',
        ROOT/'benchmarks/reflex_app.py',ROOT/'benchmarks/nano_app.py',ROOT/'benchmarks/requirements.lock']
    result['source_sha256']={str(p.relative_to(HERE.parent)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
    result['completed']=True;save()
    print('Measured events:',sum(v['events'] for v in result['runs']),flush=True)

if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('--rows',nargs='+',type=int,default=[0,100,1000,10000])
    p.add_argument('--repeats',type=int,default=5)
    p.add_argument('--events',type=int,default=100)
    p.add_argument('--warmup',type=int,default=20)
    p.add_argument('--clients',type=int,default=8)
    p.add_argument('--seed',type=int,default=20260915)
    p.add_argument('--output',default='backend-results.json')
    asyncio.run(main(p.parse_args()))
