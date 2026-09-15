"""Measure release source builds in a new target directory, then package artifacts.

Rust and registry dependencies must already be installed. Nothing is downloaded
inside the timer. This is separate from the per-application lifecycle benchmark.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
os.chdir(ROOT)
subprocess.run([sys.executable, 'benchmarks/verify_frontend.py'], check=True)
target = Path(tempfile.mkdtemp(prefix='nano-build-'))
env = {**os.environ, 'CARGO_TARGET_DIR': str(target)}
result = {'version':'0.4.0', 'date_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),
    'rustc':subprocess.check_output(['rustc','--version'],text=True).strip(),
    'target_was_empty':True, 'jobs':1, 'network':'offline',
    'profile':'release; thin LTO; one codegen unit',
    'excluded':['toolchain installation','registry downloads','OS filesystem cache flush','prebuilt React bundle; npm run build:frontend measured separately'],
    'runs':[], 'target_directory':str(target)}
def measure(label, command):
    start=time.perf_counter_ns()
    p=subprocess.run(command,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    seconds=(time.perf_counter_ns()-start)/1e9
    (ROOT/'docs'/f'build-{label}.log').write_text(p.stdout)
    record={'stage':label,'command':command,'seconds':seconds,'returncode':p.returncode}
    result['runs'].append(record)
    (ROOT/'docs/build-timings.json').write_text(json.dumps(result,indent=2))
    print(json.dumps(record),flush=True)
    if p.returncode: raise RuntimeError(p.stdout[-12000:])

native=['cargo','build','--release','--locked','--offline','-j','1',
        '-p','reflex-nano','--example','runner']
measure('clean-native',native)
shutil.copy2(target/'release/examples/runner',ROOT/'dist/nano')
measure('cached-native',native)
measure('python-wheel-after-native',[
    str(Path(sys.executable).parent/'maturin'),'build','--release','--locked','--offline',
    '-j','1','--compatibility','manylinux_2_34','--interpreter',sys.executable,
    '--out',str(ROOT/'dist')])
result['sha256']={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()
    for p in [ROOT/'dist/nano',ROOT/'Cargo.lock',ROOT/'Cargo.toml',ROOT/'rust-toolchain.toml',
              *ROOT.joinpath('dist').glob('*0.4.0*.whl'),*ROOT.joinpath('crates').rglob('*.rs'),
              ROOT/'crates/nano-core/src/client.js']}
result['completed']=True
(ROOT/'docs/build-timings.json').write_text(json.dumps(result,indent=2))
