"""Extra real-browser event and retained-memory measurements on all five modes."""
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import random
import signal
import subprocess
import sys
import time

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent/'reflex-nano'
LIFE=ROOT/'benchmarks/lifecycle'
env={**os.environ,'NANO_PYTHON':sys.executable,'REFLEX_ENV_MODE':'prod',
     'NANO_CHROMIUM_PATH':os.environ.get('NANO_CHROMIUM_PATH','/tmp/chromium')}
result={'version':'0.4.0','date_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),
    'method':'GC-retained V8 heap and server /proc smaps_rollup; memory is not peak RSS or whole-browser memory',
    'settings':{'rows':[100,1000,10000],'repeats':3,'contexts':1,'warm_scalar_events':50,'warm_toggle_events':20},
    'packages':{name:importlib.metadata.version(name) for name in ['reflex','reflex-nano','granian']},
    'runs':[],'completed':False}
output=HERE/'browser-results.json'
logs=HERE/'logs';logs.mkdir(exist_ok=True)
def run(command,label,extra=None,cwd=LIFE):
    with (logs/f'{label}.log').open('w') as log:
        process=subprocess.Popen(command,cwd=cwd,env={**env,**(extra or {})},stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
        try:code=process.wait(timeout=180)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid,signal.SIGKILL);process.wait();raise
    if code:raise RuntimeError((logs/f'{label}.log').read_text()[-6000:])

rng=random.Random(20260915)
for rows in result['settings']['rows']:
    env['NANO_BENCH_ROWS']=str(rows)
    run([sys.executable,'prepare_reflex.py'],f'prepare-{rows}')
    run([sys.executable,'compile_reflex.py'],f'compile-{rows}')
    run([sys.executable,'build_reflex.py'],f'build-{rows}')
    jobs=[(kind,repeat) for kind in ['nano','nano_python','nano_react','nano_react_python','reflex'] for repeat in range(3)]
    rng.shuffle(jobs)
    for kind,repeat in jobs:
        destination=logs/f'{kind}-{rows}-{repeat}.json'
        run(['node',str(HERE/'browser.cjs')],f'{kind}-{rows}-{repeat}',
            {'BENCH_KIND':kind,'BENCH_REPETITIONS':'1','BENCH_OUTPUT':str(destination)})
        record=json.loads(destination.read_text());record['repeat']=repeat
        result['runs'].append(record);output.write_text(json.dumps(result,indent=2))
        print(f'Browser {kind} rows={rows} run={repeat+1}/3; processes={len(result["runs"])}',flush=True)
sources=[HERE/'browser.py',HERE/'browser.cjs',HERE/'python_server.py',ROOT/'dist/nano',
    *LIFE.glob('*.py'),LIFE/'probe.js',LIFE/'reflex.lock/package-lock.json',
    *ROOT.joinpath('dist').glob('*0.4.0*.whl')]
result['source_sha256']={str(p.relative_to(HERE.parent)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
result['frontend_versions']={name:json.loads((LIFE/'.web/node_modules'/name/'package.json').read_text())['version']
    for name in ['react','react-dom','vite','react-router','socket.io-client']}
result['completed']=True;output.write_text(json.dumps(result,indent=2))
print('Extra browser measurement complete',flush=True)
