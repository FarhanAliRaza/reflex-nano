# Full comparison: Nano 0.4.0 and Reflex 0.9.11

Open **REPORT.html** for the readable report, or REPORT.md for plain Markdown.
The HTML is self-contained; its table links open the accompanying CSV/JSON files.
Print / PDF uses your browser's print dialog.

Included evidence:

- backend-results.json: 148,500 fresh event timings, 330 groups, 12 live contracts.
- browser-results.json: 90 extra navigations, retained memory and repeated UI events.
- backend-summary.csv and browser-summary.csv: aggregated current-version results.
- summary.json: machine-readable combined summary.
- The recovered Nano 0.4 release contains the earlier 144-navigation experiment,
  full build/startup/reload/transfer data, source, native executable and Python wheel.
- SHA256SUMS covers every packaged file except itself.

The original lifecycle and fresh memory/update experiments are kept separate.
No 0.3 timings are reused as 0.4 results. The application runtimes were not changed.

## Reproduce on compatible Linux x86-64

From the outer extracted directory, containing this folder and recovered/:

```bash
unzip recovered/reflex-nano-0.4.0.zip
chmod +x reflex-nano/dist/nano
uv venv nano-compare-venv --python 3.12
uv pip install --python nano-compare-venv/bin/python \
  -r reflex-nano/benchmarks/requirements.lock \
  reflex-nano/dist/reflex_nano-0.4.0-cp312-cp312-manylinux_2_34_x86_64.whl
npm ci --prefix reflex-nano --ignore-scripts --no-audit --no-fund
```

Install a compatible Chromium and set NANO_CHROMIUM_PATH to its executable.
Set PLAYWRIGHT_MODULE_PATH to the absolute reflex-nano/node_modules/playwright path.
The original run used Chromium 133.0.6943.0 and the runtime's Playwright module.

Prepare the pinned Reflex production frontend outside the measurement timer:

```bash
cd reflex-nano/benchmarks/lifecycle
../../../nano-compare-venv/bin/python prepare_reflex.py
cp reflex.lock/package.json .web/package.json
cp reflex.lock/package-lock.json .web/package-lock.json
npm ci --prefix .web --legacy-peer-deps --ignore-scripts --no-audit --no-fund
cd ../../..
```

Run each benchmark sequentially; do not overlap CPU-heavy work:

```bash
nano-compare-venv/bin/python reflex-nano-comparison/backend.py
NANO_CHROMIUM_PATH=/path/to/chromium \
PLAYWRIGHT_MODULE_PATH=/absolute/path/reflex-nano/node_modules/playwright \
  nano-compare-venv/bin/python reflex-nano-comparison/browser.py
nano-compare-venv/bin/python reflex-nano-comparison/report.py
```

backend.py explicitly sets the supported REFLEX_SKIP_COMPILE flag before the
server process imports Reflex; its initial dry-run compiler still evaluates the
application. This avoids an unrelated frontend package installation at backend
startup. The browser harness builds the real production frontend for each size.

The memory collector resolves namespace PIDs to visible /proc PIDs, reads server
smaps_rollup and collects V8 retained heap after GC. It does not measure total or
peak browser memory. Failed setup/smoke runs are not in the datasets.

Changing measured source files intentionally invalidates recorded provenance.
The report verifies the original release against its immutable archive and the
new runs against their recorded source/binary hashes. Rerun relevant measurements
after changing a harness. The native build is reused; no Rust compiler is needed
to run this comparison on a compatible platform.
