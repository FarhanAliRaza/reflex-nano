#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
command -v uv >/dev/null || { echo 'Install uv: https://docs.astral.sh/uv/'; exit 1; }
command -v cargo >/dev/null || { echo 'Install Rust: https://rustup.rs/'; exit 1; }
uv venv .venv --python 3.12
uv pip install --python .venv/bin/python -r requirements-dev.txt
VIRTUAL_ENV="$PWD/.venv" .venv/bin/maturin develop --release --locked
echo 'Ready: .venv/bin/nano run examples/python/dashboard.py'
