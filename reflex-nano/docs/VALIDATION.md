# Validation — Reflex Nano 0.4.0

The final standalone Rust executable and CPython 3.12 manylinux wheel passed on
Linux x86-64 with Rust 1.97.0, Python 3.12.14 and Chromium 133.0.6943.0.

- **10 Rust tests** passed: native state sharing/transactions, computed caches,
  programs, scope/escaping, dual render translation, React SSR text boundaries,
  component fallbacks, routes, sessions, gzip, asset caching and 304 responses.
- **19 Python-driven native API/socket tests** passed. Both renderer choices and
  per-page overrides round-trip through native manifests. Python State classes
  and callable event handlers remain absent.
- **60 dashboard behavior groups** passed: 15 groups × two hosts × two renderers.
  These include forms, controlled inputs, keyed lists, dynamic routes, history,
  refresh, session isolation, streaming, background events, server-side chains,
  reconnects, concurrent events, cross-tab updates and desktop/mobile layout.
- **12 hydration/optimization groups** passed: six groups per renderer. Checks
  retain SSR element identities and early input, validate adjacent text/lexical
  scopes, skip unaffected rows, preserve focus/selection during keyed movement,
  refresh event indices, and verify debounce/throttle/temporal/propagation.
- **Six React component groups** passed: HTML pages download no React; renderer
  transitions retain Rust state; Radix Button/TextField/Switch/Select; controlled
  Dialog and events from portals outside the root; custom React hooks retained
  across Rust updates; reload and direct HTML fallback behavior.
- No recorded JavaScript page errors or React recoverable hydration errors in
  these fixtures. Events use real WebSockets, with no HTTP event POSTs.
- A fresh wheel environment with **zero Python runtime dependencies** passed
  native binding checks and all **11 live runtime-contract checks**.
- The frontend build matches all nine hashed source/lock inputs, includes its
  third-party license inventory, and reproduced the tested embedded asset bytes
  identically in three timed rebuilds.

Evidence is in `validation.json`, `browser-html-results.json`,
`browser-react-results.json`, `optimization-html-tests.json`,
`optimization-react-tests.json`, `react-components-tests.json`,
`PACKAGE-VALIDATION.json`, `frontend-build-timings.json` and `verify-*.log`.
The older `browser-results.json` and `optimization-tests.json` describe 0.3.

`build-timings.json` measures this release's clean native build, cached rebuild
and subsequent wheel build; dependency downloads and installation are excluded.
`REACT-BENCHMARKS.md` and `benchmarks/lifecycle/react-results.json` contain the new
browser comparison. Prior 0.2/0.3 backend/lifecycle results remain historical;
the new work does not relabel them as 0.4 measurements.

Reproduce after `benchmarks/build_engine.py`:

```bash
NANO_CHROMIUM_PATH=/path/to/chromium python benchmarks/verify_release.py
```

Install Playwright's Chromium for `tests/browser.cjs`, or set
`NANO_CHROMIUM_MODULE` to a compatible executable provider.
`PLAYWRIGHT_MODULE_PATH` selects an existing Playwright module. These are test
requirements, not application runtime dependencies. The test container disables
the browser sandbox; ordinary web security remains enabled.

Coverage applies to the tested contract, not complete Reflex ecosystem parity.
In particular, third-party components mount in the browser after their explicit
SSR fallback; Rust does not execute arbitrary JavaScript component SSR.
