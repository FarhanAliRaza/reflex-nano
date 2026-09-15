"""Prepare an application against the prebuilt Rust engine, through bindings."""
import json
import os
import time
from pathlib import Path

start = time.perf_counter_ns()
from nano_fixture import fixture
imported = time.perf_counter_ns()
rows = int(os.environ['NANO_BENCH_ROWS'])
app = fixture(rows)
definition = time.perf_counter_ns()
source = app.to_json()
Path(f'nano-{rows}.json').write_text(source)
exported = time.perf_counter_ns()
# Rust validates/builds the complete runtime and renders every row.
html = app.render('/bench')
end = time.perf_counter_ns()
assert html.count('class="item"') == rows
print('COMPILE_RESULT ' + json.dumps({
    'import_ms': (imported-start)/1e6, 'definition_ms': (definition-imported)/1e6,
    'export_ms': (exported-definition)/1e6, 'validate_build_render_ms': (end-exported)/1e6,
    'inside_process_ms': (end-start)/1e6, 'manifest_bytes': len(source.encode()),
    'render_bytes': len(html.encode()), 'javascript_build_required': False,
}), flush=True)
