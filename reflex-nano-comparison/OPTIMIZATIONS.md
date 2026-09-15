# Optimization audit — Nano 0.3.0

The implementation covers the core optimization mechanisms relevant to Nano's
native HTML, state and event contract. It does not claim every optimization or
component in the Reflex/React ecosystem. All schema, state, programs, computed
values, scheduling, render compilation, asset compilation and server execution
remain Rust. Python only exposes native PyO3 objects. Browser DOM operations and
browser event timing necessarily use the bundled JavaScript adapter.

| Mechanism | Implementation and effect | Coverage |
| --- | --- | --- |
| Memoization / state hook isolation | Rust annotates every node with free state and lexical dependencies. Browser updates skip unaffected subtrees. | Added; no React hooks required |
| Constant expressions and static output | Rust folds pure expressions and compiles static HTML/attributes/styles into coalesced SSR operations. Browser caches constant property maps. | Added |
| Reusable compilation | A page's validated render plan and serialized wire tree are shared across sessions. | Added; no per-app npm build |
| Server rendering and hydration | Rust emits HTML; browser adopts existing nodes, splits adjacent text when needed and preserves early focused input. | Added adoption; zero replaced elements in regression fixture |
| Keyed lists | Existing keyed DOM identity retained; unchanged row values shared; moved rows refresh event index scope. | Improved; focus and selection tested |
| Cached computed values | Dependency-ordered pure Rust expressions reuse shared `Arc<Value>` outputs. Unchanged state fields use pointer equality before deep equality. | Improved; transitive invalidation tested |
| Dirty state / structural sharing | Rust snapshots/transactions share unchanged top-level fields; writes copy the modified field. | Retained from 0.2 |
| Selective state transport | Full snapshots and changed top-level fields serialize directly from borrowed Rust state, avoiding intermediate deep-cloned JSON values. | Improved; nested collection wire patches absent |
| Initial snapshot | Equal snapshot data reuses existing values and avoids a second render; changed snapshots still reconcile. | Added; reconnect always receives authoritative state |
| Event processing | Eight delegated browser listeners use current scopes. Native definitions specify debounce, leading throttle, temporal discard and propagation. Rust executes all application events. | Added; final arguments, no trailing throttle and reconnect discard tested |
| Native async / chains | Tokio scheduling, transaction checkpoints, background pushes and immediate chains avoid a Python scheduler and unnecessary chain round trips. | Retained |
| Asset optimization | Rust syntax minification at engine build; precomputed gzip JS/CSS; content hashes, immutable caching, ETags, preload and gzip HTML. | Added; compression negotiation and 304 tested |
| Routing and concurrency | JSON navigation skips discarded HTML rendering; SSR releases the session lock after capturing its consistent state/version. | Added |

The minifier uses `parse-js` and `minify-js`'s Rust parser/emitter. Aggressive
control-flow optimization in minify-js 0.6.0 panicked on this valid client; the
implementation uses syntax minification without that pass. No third-party
compiler code was patched. This is not a claim of whole-program tree shaking,
identifier mangling or React compiler equivalence.

React-specific code splitting, third-party component bundling, hooks, dynamic
component imports and React compiler optimizations do not have equivalent
components in Nano. Distributed state, Redis, persistence, database integration,
uploads and full Reflex deployment tooling remain outside the current contract.
Lists are fully rendered: virtualization or omitted state would invalidate this
experiment. Changed lists still transmit the whole top-level field and require
an O(n) comparison, even when only one row's DOM changes. Custom native Rust
computed closures retain their ordinary execution semantics; only declared pure
expressions are automatically cached. These limits are not hidden by timing
claims.

## Evidence

`optimization-tests.json` records SSR identity, early input, constant folding,
nested lexical scopes, dependency skipping, keyed reordering with focus and
selection, empty-list recovery, event controls and reload correctness. The
ordinary browser suite covers both the Python binding host and native process,
including forms, background jobs, streaming, cross-tab updates and routing.

`LIFECYCLE-BENCHMARKS.md` reports randomized same-session measurements against
the preserved Nano 0.2.0 executable and actual Reflex production builds. The
complete state and every DOM row are verified at 100, 1,000 and 10,000 rows.
Compilation, full readiness, reload, first event and transfer sizes are separate
measurements. This combined release comparison does not isolate the causal
contribution of each optimization and is not a pure Rust-versus-Python result.

Source-build timings use pinned Rust 1.97.0. Rust 1.98.1 crashes before compilation
in this container; the older 0.2 binary was built with 1.98.1. Both use the same
release settings (thin LTO, one codegen unit). The compiler version difference is
a confound for native before/after measurements. Registry/toolchain setup is
excluded from build timing; no zero-second compile time is asserted.

## Reflex sources used in the audit

- [Memoized components and state boundaries](https://reflex.dev/docs/library/other/memo/)
- [Cached computed variables](https://reflex.dev/docs/vars/computed-vars/)
- [Event actions and timing semantics](https://reflex.dev/docs/events/event-actions/)
- Installed Reflex 0.9.11 `compiler/plugins/memoize.py`, `state.py`, and
  `utils/precompressed_staticfiles.py` were inspected directly. Reflex and its
  generated production frontend were not patched for the comparison.
