# Compatibility scope

Nano 0.4 implements the following runtime contract and basic UI behaviors.
It does not claim complete feature or source compatibility with Reflex.
Python exposes Rust objects; Python State classes and event function execution
are deliberately absent from Nano.

| Capability | Nano implementation | Evidence/comparison scope |
| --- | --- | --- |
| State and event definitions | Rust `Schema`, `Program`, `Action` | Rust and binding tests; exported manifest runs without Python |
| Per-client state isolation | Cookie-based in-memory Rust sessions | Shared live contract with Reflex |
| Integer/string/list state and computed values | Native fields and pure expression cache | Shared live contract; final states checked in every benchmark workload |
| Sync events | Rust transactions | Shared live contract and timed workload |
| Async behavior | Rust deferred events and native async APIs | Shared behavior contract; timed yield-then-increment workload |
| Forms and controlled input | Native event arguments, browser bindings | Shared backend payload contract; Nano browser tests |
| Nested list mutation | Copy-on-write collection field | Shared contract and timed workload |
| Streaming updates | Rust publish checkpoints | Both runtimes deliver all three checkpoints in the contract test |
| Background updates | Tokio jobs and short state transactions | Shared contract verifies pushes alongside a foreground event |
| Event chaining | Rust `call` actions, immediate events | Shared final-state contract; no extra client round trips in Nano benchmark |
| Reconnection | Snapshot resynchronization and retained live-session state | Shared contract; Nano browser test |
| Cross-tab state | Tabs sharing a cookie share state and broadcasts | Nano test; Reflex ordinarily uses per-tab tokens, so semantics differ |
| Ordering and duplicate receipts | Session gate, 256-entry receipt cache | Nano concurrent/retry tests; not a claim about Reflex delivery guarantees |
| Components | Native HTML; registered React components and Radix Themes | Browser tests for both renderers and Radix portals/callbacks |
| React hydration | React 19 hydrateRoot for native HTML; explicit fallback then browser mount for imported components | SSR element identity and zero recoverable hydration errors in the tested fixtures |
| React composition | Synthetic events, composed refs/handlers, custom hooks, native state/event contexts | Dialog trigger composition and custom hook state retention |
| Reactive UI | Rust dependency plans, SSR adoption, selective keyed updates, input bindings | Nano browser tests, 15 cross-language expression fixtures |
| Routes | Static/dynamic paths, SPA navigation, history and reload | Nano browser tests |
| Error handling | Rollback since last checkpoint; visible errors | Native, binding, socket and browser tests |

See [OPTIMIZATIONS.md](OPTIMIZATIONS.md) for the optimization audit and its limits.

## Remaining gaps

- Reflex's Python class/decorator API, inheritance/substates, and arbitrary Python
  handlers are not supported. Python code is not compiled into Rust automatically.
- React and registered Radix/custom components work, but Nano does not implement
  Reflex’s entire wrapper catalog or generated client API. External component
  packages must be added to the explicit frontend registry and rebuilt.
- Third-party JavaScript components execute only in the browser. Rust renders
  their explicit fallback during SSR; it does not run React server-side JS,
  React Server Components, streaming React SSR or Suspense data loaders.
- Database/ORM integration, authentication helpers, uploads, client storage,
  cookies/local-storage state vars, background cancellation/superseding,
  arbitrary background effects and Reflex's full routing/on-load lifecycle are
  not implemented as framework features.
- Declarative native programs cover the operations listed in the README. They
  are not a general Python replacement or a complete async workflow language.
  Additional native Rust application logic can use the Rust API.
- Sessions are local to one process. Redis/distributed state, durable receipts,
  restart recovery, worker coordination, deployment tooling and production
  operations have not reached Reflex parity.
- Nano sends top-level field deltas. A change within a collection sends that
  whole collection; nested wire patches are not implemented.
- Both renderers use Nano’s native render plan and WebSocket protocol. It is not compatible
  with an existing compiled Reflex frontend.

The original benchmark measures backend event delivery for the explicit shared
contract. The separate lifecycle benchmark measures real production compilation,
browser load/reload, and full state/DOM readiness for a defined row-based fixture;
see `LIFECYCLE-BENCHMARKS.md`. Neither establishes whole-framework, ecosystem,
or language-only performance parity.
