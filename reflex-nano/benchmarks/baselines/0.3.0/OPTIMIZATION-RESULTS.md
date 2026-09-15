# Nano 0.3 optimization results

Measured 2026-09-15T10:15:04Z. Same-machine randomized rerun: Nano 0.2.0, Nano 0.3.0
standalone Rust, Nano 0.3.0 through PyO3, and Reflex 0.9.11 production.
All 240 navigations passed full WebSocket state, every-row DOM and real
event checks. No rows are virtualized. Values below are medians in milliseconds
unless stated. Each browser cell has 5 independent
process runs × 2 fresh contexts. Full ranges, PyO3
readiness, startup and timing breakdown are in `LIFECYCLE-BENCHMARKS.md`.

## Initial readiness and first reload

| Rows | Navigation | Nano 0.2 | Nano 0.3 Rust | Reflex | Old / new Nano | Reflex / new Nano |
| --- | --- | --- | --- | --- | --- | --- |
| 100 | initial | 48.8 | 37.6 | 89.4 | 1.30× | 2.37× |
| 100 | first_reload | 46.7 | 33.1 | 59.8 | 1.41× | 1.81× |
| 1,000 | initial | 177.9 | 100.0 | 157.7 | 1.78× | 1.58× |
| 1,000 | first_reload | 160.7 | 88.3 | 122.3 | 1.82× | 1.38× |
| 10,000 | initial | 1419.2 | 598.8 | 772.5 | 2.37× | 1.29× |
| 10,000 | first_reload | 1453.7 | 571.7 | 668.2 | 2.54× | 1.17× |

Ratios above 1 mean Nano 0.3 is faster. This measures navigation to verified full
state, rendered DOM and connected/hydrated framework readiness. Paint-ready is
also recorded separately. Reload retains the browser cache and session.

## First counter event after initial hydration

| Rows | Nano 0.2 | Nano 0.3 Rust | Nano 0.3 PyO3 | Reflex | Old / new Nano |
| --- | --- | --- | --- | --- | --- |
| 100 | 5.10 | 1.75 | 1.65 | 4.40 | 2.9× |
| 1,000 | 25.25 | 1.85 | 1.85 | 10.15 | 13.6× |
| 10,000 | 163.05 | 1.85 | 1.80 | 37.50 | 88.1× |

This is a real programmatic DOM click through WebSocket execution and back to
the observed DOM update, including the computed double. At 10,000 rows Nano
adopted 90,014
existing nodes and created zero replacement nodes. A counter update visited
five render-plan nodes and changed two text nodes, regardless of list size.
These counts demonstrate skipped list work; they do not imply all events are O(1).

## Initial HTTP transfer

| Rows | Nano 0.2 KiB | Nano 0.3 KiB | Reflex KiB | Nano reduction |
| --- | --- | --- | --- | --- |
| 100 | 41.3 | 10.0 | 162.4 | 75.7% |
| 1,000 | 227.6 | 21.5 | 174.0 | 90.5% |
| 10,000 | 2126.1 | 123.0 | 266.9 | 94.2% |

Includes document and browser assets, using Resource Timing transfer sizes.
WebSocket snapshot bytes are reported separately in the full lifecycle report.

## Application preparation / production compilation

| Rows | Nano app preparation | Reflex total build | Reflex / Nano |
| --- | --- | --- | --- |
| 100 | 23.43 | 2677.94 | 114.3× |
| 1,000 | 26.75 | 2495.58 | 93.3× |
| 10,000 | 67.73 | 2830.64 | 41.8× |

These use an already-built Rust engine and already-installed dependencies.
Nano constructs, exports, validates and renders the app. Reflex performs actual
code generation, clean production bundling, route prerendering and compression.
This is a comparison of deployment workflows, not language compiler speed.

The final Rust source build was measured separately, once per stage:

| Build stage | Seconds |
| --- | --- |
| clean-native | 79.685 |
| cached-native | 0.067 |
| python-wheel-after-native | 39.309 |

The first stage uses an empty target directory and cached registry sources;
the second is an unchanged cached rebuild. Wheel construction follows the native
build and reuses its compiled dependencies where applicable. Thin LTO, one
codegen unit, one Cargo job; toolchain installation and downloads excluded.

## Interpretation and scope

The complete optimization inventory is in `OPTIMIZATIONS.md`. Rust owns state,
schema, events, computed expressions, scheduling and render/asset compilation.
Python only binds native types; browser DOM operations use the JS adapter.
This release ports applicable core optimization mechanisms and does not claim
complete Reflex/React ecosystem parity.

The comparison is a combined implementation change, with no per-optimization
ablation. The old binary used Rust 1.98.1 and this release uses working Rust 1.97.0;
the compiler change is a confound. Loopback/headless measurements do not establish
WAN performance, memory savings, distributed scalability or a pure language
speedup. Raw samples and source/binary hashes are in
`benchmarks/lifecycle/results.json`; build hashes are in `docs/build-timings.json`.
