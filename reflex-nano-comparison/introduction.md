# Reflex Nano 0.4 versus Reflex: full comparison

**Nano wins most clearly on app preparation, server startup, server memory and backend events. Direct HTML also gives the lightest browser path. Nano React is not uniformly faster or smaller: large list updates are slower than Reflex, and retained JavaScript heap is higher.**

This report compares the exact **Nano 0.4.0** release with **Reflex 0.9.11**. It combines the preserved **144-navigation** release experiment with **148,500 fresh backend events** and **90 additional browser navigations** for repeated updates and memory. No Nano 0.3 performance numbers are attributed to 0.4.

Nano's schema, authoritative state, event programs, computed values and render-plan compiler are Rust. PyO3 exposes native objects with no Python event callbacks. Exported applications run without Python. Native Rust closures extend the bounded declarative programs; those programs execute through the Rust interpreter, rather than being JIT-compiled to separate machine code per application. Browser DOM operations, React, public snapshots, input drafts and local hooks run in JavaScript.

Reflex executes state and application handlers in Python and compiles UI definitions to React. The tested version uses React Router/Vite and Granian ASGI; some public architecture documentation retains descriptions of older stack layers. The installed version and recorded dependencies govern this experiment. [Reflex architecture](https://reflex.dev/docs/advanced-onboarding/how-reflex-works/).

## How to read the report

The release section retains all compilation, startup, load, reload, first-event and transfer measurements. The next sections add current-version backend throughput, repeated browser mutations, memory and feature coverage. These datasets are not pooled: the memory sweep forces garbage collection and runs additional events between initial load and reload.

The roughly 29 ms Nano app-preparation figure uses its **prebuilt engine through Python bindings and exported native definitions**. Editing native Rust application source requires Cargo compilation. The roughly 63 ms cached Cargo figure means **no source changed**, not an incremental edit. Custom Rust app rebuilds and source-edit HMR were not measured.

All tests are local loopback experiments in a shared container. Results are specific to these fixtures, versions and configurations; they do not establish universal speedups or complete Reflex compatibility.
