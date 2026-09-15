"""Reject stale checked-in React bundles before building or packaging a release."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sources = json.loads((ROOT/'crates/nano-core/frontend-dist/sources.json').read_text())
for name, expected in sources.items():
    assert hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == expected, (
        f'{name} changed: run npm ci && npm run build:frontend before building Rust')
assert (ROOT/'crates/nano-core/frontend-dist/.vite/license.md').is_file()
print(f'Frontend bundle matches all {len(sources)} pinned source inputs; licenses present.')
