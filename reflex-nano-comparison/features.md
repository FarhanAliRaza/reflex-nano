## Feature coverage and semantics

The Reflex column describes framework capabilities, not features all enabled in the timed fixture. Authentication and database workloads were not benchmarked.

| Capability | Nano HTML | Nano React | Reflex |
| --- | --- | --- | --- |
| Authoritative state and event execution | Rust | Rust | Python |
| Definition API | Native Rust; PyO3 constructs native objects | Same | Python classes/decorators |
| Arbitrary application handlers | Native Rust; bounded programs through bindings | Same | Python sync/async functions and libraries |
| Unchanged Reflex source | Not compatible | Not compatible | Native |
| Browser renderer | Direct DOM JavaScript | React 19 / ReactDOM | React 19 / ReactDOM |
| Native HTML SSR/hydration | Rust SSR + DOM adoption | Rust SSR + hydrateRoot | React prerendering/hydration in measured build |
| Third-party component SSR | Explicit HTML fallback | Fallback then browser mount | SSR-capable wrappers; NoSSR where needed |
| Radix, React hooks, portals | Fallback only | Registered components, tested | Broader component/wrapper ecosystem |
| Add React package | Needs HTML fallback | Registry + frontend and Rust rebuild | Python wrapper + frontend build |
| Controlled forms and keyed lists | Tested | Tested | Supported |
| Computed caching | Native dependency expressions | Same | Computed vars/dependency tracking |
| Debounce/throttle/temporal/propagation | Tested | Tested | Supported event actions |
| Streaming/background/chaining | Native checkpoints and Tokio; bounded program semantics | Same | Python generators/async/background handlers |
| State inheritance and substates | Missing | Missing | Supported |
| Static/dynamic routes and history | Tested | Tested | Supported |
| Full on_load lifecycle | Missing | Missing | Supported |
| Cross-tab session semantics | Cookie-shared state | Cookie-shared state | Normally per-tab tokens |
| Event protocol | nano.v1 WebSocket | Same | Socket.IO/Engine.IO; WebSocket in this test |
| File uploads | Missing | Missing | Supported |
| Cookie/local-storage state vars | Missing; session cookie is separate | Same | Supported |
| ORM integration | No framework integration | Same | Database integration available |
| OIDC authentication helpers | No framework integration | Same | Enterprise AuthPlugin, separate package |
| Redis/distributed state manager | Missing | Missing | Available; benchmark used memory |
| Restart persistence | In-memory state is lost | Same | Depends on state manager/deployment |
| Development HMR | No complete watcher/HMR workflow | Same | Development tooling available; not timed |
| Deployment | Native executable or wheel | Same, with embedded frontend | CLI/hosting/container workflows |
| Durable exactly-once events | No; bounded 256-receipt cache | Same | Not established by this comparison |

Reflex supports [wrapping React components](https://reflex.dev/docs/wrapping-react/overview/), [file uploads](https://reflex.dev/docs/library/forms/upload/), [database integration](https://reflex.dev/docs/database/overview/), [client storage](https://reflex.dev/docs/client-storage/overview/) and [page-load events](https://reflex.dev/docs/events/page-load-events/). [OIDC authentication](https://reflex.dev/docs/enterprise/auth/overview/) is an Enterprise capability requiring an additional package; it was not enabled in these measurements.

Nano's Radix integration is real, including composed refs/handlers and portal callbacks. It is not an unchanged generated Reflex frontend. Imported components hydrate explicit Rust-rendered HTML fallbacks and then mount their interactive React version. Rust does not execute arbitrary third-party JavaScript SSR. A single feature-parity percentage would hide these differences.

## Optimization coverage and remaining measurements

Nano includes shared native state fields, transactional writes, dependency-cached computed expressions, constant folding, reusable compiled render plans, SSR DOM reuse, stable keys, subtree dependency skipping, early-input preservation, queued events, gzip/immutable assets and lazy React component bundles. Equal row values are shared in the browser so unrelated scalar events can skip list subtrees.

Collection mutations still copy and transmit their containing field, and list comparison remains proportional to its size. There are no nested wire patches or list virtualization. The full 10,000-row state remains roughly 437.5 KiB. Backend gains cannot simply be multiplied into page-load gains: parsing, DOM size, React work and full state delivery remain. This is an architectural interpretation, not a measured CPU breakdown.

The current results do not establish every Reflex compiler optimization in Rust. They also do not establish source-edit HMR speed, custom Rust application incremental builds, production CPU cost per request, total/peak browser RAM, long-running leaks, maximum connected users, multi-worker or Redis operation, persistence recovery, TLS/WAN/mobile-network behavior, database/auth/upload performance, accessibility conformance or independent security certification. Paint-ready and detailed resource timing are retained in the lifecycle data; LCP and INP are not established.

Direct HTML is the stronger measured performance option for the native-node fixture. Nano React adds useful interoperability but has measurable browser regressions on list-heavy work. Reflex provides the broader application framework. The evidence supports these specific tradeoffs, not complete compatibility at universally higher speed.
