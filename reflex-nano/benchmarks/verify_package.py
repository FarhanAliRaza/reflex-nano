"""Check the wheel in a fresh environment with no runtime dependencies."""
import asyncio
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace

import driver

ROOT=Path(__file__).resolve().parents[1]
venv=Path(tempfile.mkdtemp(prefix='nano-wheel-check-'))/'venv'
subprocess.run(['uv','venv',str(venv),'--python',sys.executable],check=True)
python=str(venv/'bin/python')
wheel=ROOT/'dist/reflex_nano-0.4.0-cp312-cp312-manylinux_2_34_x86_64.whl'
subprocess.run(['uv','pip','install','--python',python,'--no-deps',str(wheel)],check=True)
check='''
import importlib.metadata, json
import reflex_nano as r
assert importlib.metadata.version('reflex-nano')=='0.4.0'
assert not hasattr(r,'State')
assert all(getattr(r,name).__module__=='reflex_nano._native' for name in ('App','Schema','Program','Action','Expr','Node'))
app=r.App.dashboard()
assert 'Rust all the way' in app.render()
assert '_s' in json.loads(app.render_plan_json())
assert r.__version__ == '0.4.0'
app.set_renderer('react')
assert app.renderer == 'react'
assert 'Rust all the way' in app.render()
assert json.loads(app.to_json())['renderer'] == 'react'
print(json.dumps({'version':importlib.metadata.version('reflex-nano'),'requires':importlib.metadata.requires('reflex-nano')}))
'''
details=json.loads(subprocess.check_output([python,'-c',check],text=True))
assert not details['requires']
driver.sys=SimpleNamespace(executable=python)
async def verify():
    async with driver.Backend('nano',10,port=3361,python_host=True) as backend:
        return await driver.contract(backend)
checks=asyncio.run(verify())
assert len(checks)==11
result={'version':'0.4.0','clean_wheel_install':True,'runtime_contract':checks,
        'python_dependencies_required':[], 'native_api_passed':True}
(ROOT/'docs/PACKAGE-VALIDATION.json').write_text(json.dumps(result,indent=2))
print(json.dumps(result,indent=2))
