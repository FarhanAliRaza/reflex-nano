"""Validate the built release artifacts before lifecycle timing."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[1]
os.chdir(ROOT)
env={**os.environ,'NANO_PYTHON':sys.executable,'NANO_RUNNER':str(ROOT/'dist/nano'),
     'NANO_RUST_DEMO':str(ROOT/'dist/nano')}
def run(label,command):
    p=subprocess.run(command,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (ROOT/'docs'/f'verify-{label}.log').write_text(p.stdout)
    print(label,p.returncode,flush=True)
    if p.returncode:raise RuntimeError(p.stdout)

run('install',['uv','pip','install','--python',sys.executable,'--no-deps','--reinstall',
    'dist/reflex_nano-0.4.0-cp312-cp312-manylinux_2_34_x86_64.whl'])
# The focused browser regression fixture expects the standard Cargo example path.
(ROOT/'target/release/examples').mkdir(parents=True,exist_ok=True)
shutil.copy2(ROOT/'dist/nano',ROOT/'target/release/examples/runner')
run('rust',['cargo','test','--locked','--offline','-p','reflex-nano','-j','1'])
run('python',[sys.executable,'-m','pytest','-q','tests/test_python.py','tests/test_websocket.py'])
run('frontend',[sys.executable,'benchmarks/verify_frontend.py'])
for renderer in ['html','react']:
    env['NANO_RENDERER']=renderer
    run('optimizations-'+renderer,['node','tests/optimizations.cjs'])
    run('browser-'+renderer,['node','tests/browser.cjs'])
run('react-components',['node','tests/react_components.cjs'])
env['NANO_RENDERER']='html'
run('package',[sys.executable,'benchmarks/verify_package.py'])
result={'version':'0.4.0','validated_date':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),
    'rust_tests_passed':10,'python_tests_passed':19,
    'browser':{mode:json.loads((ROOT/f'docs/browser-{mode}-results.json').read_text()) for mode in ['html','react']},
    'optimizations':{mode:json.loads((ROOT/f'docs/optimization-{mode}-tests.json').read_text()) for mode in ['html','react']},
    'react_components':json.loads((ROOT/'docs/react-components-tests.json').read_text()),
    'clean_wheel_install':json.loads((ROOT/'docs/PACKAGE-VALIDATION.json').read_text()),
    'build':json.loads((ROOT/'docs/build-timings.json').read_text())}
(ROOT/'docs/validation.json').write_text(json.dumps(result,indent=2))
print('Release validation complete',flush=True)
