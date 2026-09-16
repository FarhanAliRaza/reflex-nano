# Reflex Nano

An experimental reactive web framework implemented in Rust, with Rust and Python
APIs and selectable direct HTML or React rendering.

State definitions, storage, event execution, computed values, scheduling,
routing and render-plan compilation run in Rust. Python exposes PyO3 bindings;
it does not run Python state classes or event callbacks. Both renderers use
WebSocket events and Rust-managed sessions.

This repository contains Nano **0.4.0** and its comparison against
**Reflex 0.9.11**. It implements a tested set of basic Reflex behaviors, with
documented compatibility gaps. It is not a drop-in replacement for Reflex.

**Upload status:** Rust and Python source, both frontend implementations,
generated React assets, examples, tests, build configuration, benchmark harnesses,
reports and CSV summaries are present. Large raw benchmark JSON files, some
generated benchmark fixtures, prebuilt executables/wheels, screenshots and the
original release ZIP have not been uploaded. Build from source using the commands
below. Reproducing the historical report also requires the full comparison
archive supplied with the original experiment.

## Start here

- [Framework documentation and examples](reflex-nano/README.md)
- [Rust and Python source](reflex-nano/)
- [Renderer guide](reflex-nano/docs/RENDERERS.md)
- [Feature compatibility](reflex-nano/docs/PARITY.md)
- [Full performance comparison](reflex-nano-comparison/REPORT.md)
- [Benchmark reproduction instructions](reflex-nano-comparison/README.md)
- [Validation results](reflex-nano/docs/VALIDATION.md)

## Run with Rust

The framework workspace is in the `reflex-nano/` subdirectory:

```bash
cd reflex-nano
cargo run --release --locked -p reflex-nano --example dashboard
```

Open http://127.0.0.1:3001. The toolchain is pinned in
`reflex-nano/rust-toolchain.toml`.

## Run through Python bindings

With Rust, uv and Python 3.12 available:

```bash
cd reflex-nano
./setup.sh
.venv/bin/nano run examples/python/dashboard.py --port 3000
```

The example uses `App.dashboard(renderer="react")`. Select `renderer="html"`
for direct HTML rendering. Both options keep application state and event
execution in Rust.

## Benchmark evidence

The full comparison contains 148,500 fresh backend event measurements, 90
additional browser navigations for memory and repeated updates, and the earlier
144-navigation lifecycle experiment. CSV summaries, harnesses, source hashes and
methodological limitations are included. The full raw datasets remain in the
original comparison archive; see the upload status above.

The tested Rust backend delivered roughly 4.7–6.6 times the throughput of the
tested Reflex backend in the selected 1,000/10,000-row, eight-client workloads.
Direct HTML performed best in the browser. Nano React also showed regressions:
large-list toggles were slower and retained JavaScript heap was larger than
Reflex. See the report for complete results; these are fixture measurements,
not a universal speedup or an isolated measurement of language overhead.

## Repository layout

| Path | Contents |
| --- | --- |
| `reflex-nano/` | Rust workspace, PyO3 bindings, frontend, examples, tests and historical benchmarks |
| `reflex-nano-comparison/` | Current-version comparison report, raw data and reproduction harnesses |

The sibling directory layout preserves the benchmark scripts and their recorded
hashes. Historical reproduction instructions describe the complete experiment
archive, including its prebuilt artifacts. This GitHub checkout currently contains
the source portion and reports; the prebuilt-artifact commands in the original
framework guide require that archive. Dependencies, virtual environments and
build caches are not included.

The root `SHA256SUMS` verifies the original comparison package; the nested
framework `SHA256SUMS` verifies the original release. They are historical
manifests: some referenced artifacts are not uploaded, and they do not cover this
repository overview.

Licensed under the [MIT license](LICENSE). Bundled frontend dependency notices
are preserved in `reflex-nano/crates/nano-core/frontend-dist/.vite/license.md`.
