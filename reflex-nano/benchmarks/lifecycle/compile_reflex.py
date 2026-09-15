"""Timed Reflex code generation (dependency installation is a prior setup step)."""
import json
import time
from pathlib import Path

start = time.perf_counter_ns()
from lifecycle_app import app
imported = time.perf_counter_ns()
from reflex.compiler.compiler import compile_app
from reflex.utils.build import set_env_json
assert compile_app(app, prerender_routes=True, dry_run=False, use_rich=False)
set_env_json()
end = time.perf_counter_ns()
print('COMPILE_RESULT ' + json.dumps({
    'import_definition_ms': (imported-start)/1e6,
    'codegen_ms': (end-imported)/1e6,
    'inside_process_ms': (end-start)/1e6,
}), flush=True)
