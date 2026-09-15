# Reflex Nano 0.4

A reactive web framework whose **state definitions, state storage, event execution,
computed values, scheduling, routing and render-plan compilation are implemented in Rust**.
Python is a thin PyO3 binding layer. It contains no State class, Python event
callbacks, Python computed functions, or Python task runner.

The same Rust-defined app runs through either API. It can also be exported and
run by a standalone executable without Python installed.

This implements a tested set of basic Reflex behaviors. It is **not a drop-in
implementation of the full Reflex/React ecosystem**. See [the compatibility
matrix](docs/PARITY.md), [backend event measurements](docs/BENCHMARKS.md), and
[compile, reload and full hydration measurements](docs/LIFECYCLE-BENCHMARKS.md).

Version 0.4 adds a selectable **React frontend** alongside the default direct HTML
renderer. Rust translates the same UI definition into React props/events. The
browser runs real React 19, including reconciliation, synthetic events and
`hydrateRoot`. Registered Radix Themes and custom React components are supported.
State and event execution stay in Rust in both modes. See the
[renderer guide](docs/RENDERERS.md) and [new comparison](docs/REACT-BENCHMARKS.md).

Version 0.3 introduced Rust-compiled render plans and dependency boundaries, SSR DOM
adoption, selective keyed updates, shared computed outputs, borrowed WebSocket
serialization, minified browser code, gzip and immutable asset caching. See the
[optimization audit](docs/OPTIMIZATIONS.md) for coverage and measured limits.
The [0.3 results summary](docs/OPTIMIZATION-RESULTS.md) compares full hydration,
reload and browser events against 0.2 and production Reflex. The separate
[0.3 backend rerun](docs/BACKEND-OPTIMIZED.md) measures warmed WebSocket workloads.

![Native application](docs/native-demo.png)

## Run the Rust application

```bash
cargo run --release --locked -p reflex-nano --example dashboard
# http://127.0.0.1:3001
```

The bundled Linux executable runs the same application:

```bash
PORT=3000 ./dist/nano --renderer html
PORT=3000 ./dist/nano --renderer react
```

## Run through Python bindings

Build from source with Rust, uv and Python 3.12:

```bash
./setup.sh
.venv/bin/nano run examples/python/dashboard.py --port 3000
```

Or install the bundled CPython 3.12/Linux x86-64 wheel into a compatible environment:

```bash
python -m pip install dist/reflex_nano-0.4.0-cp312-cp312-manylinux_2_34_x86_64.whl
python -m reflex_nano run examples/python/dashboard.py --port 3000
```

The complete Python dashboard definition is:

```python
from reflex_nano import App

app = App.dashboard(renderer="react")  # "html" keeps the direct renderer

if __name__ == "__main__":
    app.run()
```

`App.dashboard()` exposes the application defined in
[`crates/nano-core/src/demo.rs`](crates/nano-core/src/demo.rs). All state and
handlers are native. `App.run()` releases the GIL for the duration of serving.

Python can also construct native definitions through the bound `Schema`,
`Program`, `Action`, `Expr` and `Node` types. These are Rust objects, not Python
implementations. Literal/default values and parameter metadata use explicit JSON
at this construction boundary. See the small, runnable
[Python counter](examples/python/counter.py).

```python
from reflex_nano import Schema, Program, Action, Expr

schema = Schema()
schema.field("count", "int", "0")
schema.computed("doubled", Expr.state("count").binary("mul", Expr.literal("2")))
schema.event("increment", Program([
    Action.set("count", Expr.state("count").binary("add", Expr.literal("1")))
]))
```

No Python callable can be registered as an event handler. Applications needing
custom executable logic can register native Rust closures through the Rust API.
`App.on_native` operates directly on the shared native state; `App.on` is a Rust
JSON convenience adapter. Exportable programs use `Application` and `Schema`.

## Export and run without Python

```python
from pathlib import Path
from reflex_nano import App

Path("app.json").write_text(App.dashboard().to_json())
```

```bash
PORT=3000 ./dist/nano --manifest app.json
```

The manifest contains typed state fields, native expression trees, event
programs and page trees. Rust deserializes and validates it. It contains no
Python code. Rust closures are executable code and are not part of this
export format.

## Runtime features

- Selectable direct HTML or React rendering, per application or per page.
- HTML components, CSS, text and attributes; immutable Rust component trees.
- Registered React components, Radix Themes, custom hooks, portals and callback props.
- Native HTML SSR in both modes; explicit SSR fallbacks for third-party components.
- Reactive expressions, conditionals, keyed iteration, conditional selection,
  controlled inputs, selects, checkboxes, forms and validation.
- Typed native state fields and event parameters; isolated in-memory sessions.
- Pure computed expressions with dependency ordering, cycle detection and caching.
- Atomic foreground events, nested field mutation, append/remove, conditions,
  bounded repetition and server-side chains of immediate events.
- Deferred events, streaming checkpoints and background transactions scheduled
  by Rust/Tokio, with updates pushed to connected browsers.
- WebSocket events, top-level state deltas, ordered per-session execution,
  reconnect snapshots and cross-tab synchronization.
- Static and dynamic routes, SPA navigation, browser history, redirects and alerts.
- Rust-compiled server rendering, HTML escaping and SSR DOM reuse on hydration.
- Dependency-based subtree updates, constant folding and keyed row reuse.
- Delegated events with debounce, throttle, temporal discard and propagation controls.
- Gzip documents and content-addressed, immutable, compressed browser assets.

## Native programs and state

`Schema` holds `Field`, `Parameter`, `Program` and `Expr` definitions. Field and
parameter kinds are `json`, `bool`, `int`, `float`, `string`, `list` and `object`.
List/object kinds validate their outer type; nested record schemas are not yet
implemented. State and argument numbers must fit JavaScript's safe numeric range.

`Action` supports `set` with a dynamic nested path, `append`, `remove`, `require`,
`call`, `repeat`, `publish`, `alert` and `redirect`. `Expr` supports state/local
references, indexing, arithmetic/comparison/boolean operations, length,
concatenation, selection, trim, object construction and filtered counts.

Event modes are:

| Mode | Behavior |
| --- | --- |
| `immediate` | Runs a native transaction and commits once. |
| `deferred` | Waits with Tokio, then runs a foreground transaction. |
| `streaming` | `publish` sends intermediate committed state while preserving foreground order. |
| `background` | Tokio repeats a native transaction at a configured interval, releasing the session lock between updates. |

An error rolls back changes since the last committed checkpoint. Earlier
streaming checkpoints remain committed. Inline `call` actions invoke immediate
events; deferred/background event chaining and arbitrary async Rust application
work use the lower-level Rust API. Background declarative programs publish state
and currently do not emit redirect/alert effects.

`NativeState` stores each top-level field behind `Arc<Value>`. Snapshots and
transactions share unchanged fields. A nested write copies its containing field
once; it does not copy unrelated collections. Pure computed expressions compare
their dependencies and can reuse a cached result. These cached expressions have
no Python callbacks, external dependencies or side effects.

Render plans are compiled by Rust when a page is registered. The browser consumes that native plan through either a direct DOM adapter or
React. Neither executes Python state or event programs. Native HTML hydration
attaches to matching SSR nodes and preserves early input. Third-party React
components hydrate their explicit HTML fallback, then mount in the browser. A
counter change skips lists whose state and lexical dependencies did not change.
Changed collection payloads share equal row values before updating keyed nodes.
`App.render_plan_json(path)` exposes the native plan for inspection.

Event options are part of the native definition. Python's `Node.on` accepts
keyword arguments `debounce_ms`, `throttle_ms`, `temporal` and `stop_propagation`;
the Rust `Event` builders are `debounce`, `throttle`, `temporal` and
`stop_propagation`. Throttle is leading-edge with no trailing replay. Debounce
keeps the latest arguments. Temporal events are discarded while disconnected.

## WebSocket contract

The browser obtains an HttpOnly session cookie and CSRF token during bootstrap,
opens `/__nano/ws` with subprotocol `nano.v1`, and sends `hello` with its token and
route. The upgrade checks Origin and the session cookie; the hello validates the
token. The server returns a versioned snapshot.

Events carry a request ID, registered name, arguments and route. Rust commits
state, broadcasts changed top-level fields plus removals, and sends an ack with
any effects or error. The initiating browser and other tabs receive updates.
Different-route tabs keep their own route context. Background updates use this
same connection without a new browser request.

The browser reconnects with backoff and receives current state. An event whose
connection disappears has an unknown outcome and is not automatically replayed.
The server remembers the last 256 request IDs per session: resending the same ID
and payload within that window does not execute it twice. This is a bounded
receipt cache, not durable exactly-once delivery. Replays resynchronize to current
state rather than restoring an old snapshot.

Limits: 64 KiB incoming messages, 64 queued events per connection, 8 connections
and 16 background tasks per session, 128 buffered state broadcasts, 5,000 sessions
and one-hour idle expiry by default. Slow consumers recover through a snapshot;
intermediate streaming checkpoints may be dropped when the broadcast buffer
lags. Foreground programs have instruction/nesting limits. Heartbeats detect
lost peers. Sessions and receipts are lost on server restart.

The HTTP event endpoint remains available as a compatibility API. The browser
uses WebSockets for all events; browser tests assert that no HTTP event POSTs
occur. This wire protocol is separate from Reflex's Socket.IO protocol.

## Validation and benchmarking

```bash
cargo build --release --locked -p reflex-nano --examples
cargo test --locked -p reflex-nano
.venv/bin/pytest -q
npm ci
npx playwright install chromium
NANO_PYTHON="$PWD/.venv/bin/python" node tests/browser.cjs
NANO_PYTHON="$PWD/.venv/bin/python" node tests/optimizations.cjs
NANO_RENDERER=react NANO_PYTHON="$PWD/.venv/bin/python" NANO_RUST_DEMO="$PWD/target/release/examples/runner" node tests/browser.cjs
NANO_RENDERER=react NANO_PYTHON="$PWD/.venv/bin/python" node tests/optimizations.cjs
NANO_PYTHON="$PWD/.venv/bin/python" node tests/react_components.cjs
```

The tests exercise both launch paths, actual sockets, concurrent sessions,
background pushes, reconnects, retries, transactional failures, native ownership,
exported applications, and real browser interactions. Results are recorded in
[VALIDATION.md](docs/VALIDATION.md).

The benchmark compares the standalone Rust executable, the Python binding host,
and a real Reflex 0.9.11 backend using equivalent application behavior, in-memory
state and actual WebSockets. It includes serial and eight-client workloads,
0/100/1,000-row states, five independent process runs, and raw per-event timings.
Frontend rendering, startup, compilation, TLS and remote networking are excluded.
See [BENCHMARKS.md](docs/BENCHMARKS.md) for results, limits and reproduction.

## Layout

- `crates/nano-core/src/state.rs`: persistent native fields.
- `crates/nano-core/src/runtime.rs`: schemas, validation and native event programs.
- `crates/nano-core/src/server.rs`, `websocket.rs`: sessions, transactions and transport.
- `crates/nano-core/src/model.rs`, `render.rs`, `frontend.rs`: native UI IR and dual render compiler.
- `frontend/`: React adapter, component registry and pinned frontend build.
- `crates/nano-core/frontend-dist/`: embedded React bundles and third-party licenses.
- `crates/nano-core/src/client.js`: browser DOM adapter and delegated transport events.
- `crates/nano-core/src/assets.rs`, `build.rs`: Rust compression, caching and minification.
- `crates/nano-core/src/demo.rs`: Rust definitions shared by both APIs.
- `crates/nano-python/src/lib.rs`: PyO3 bindings; no runtime Python callbacks.
- `python/reflex_nano/`: exports and a small launcher.
- `examples/`, `tests/`, `benchmarks/`: runnable examples and validation.

The bundled binaries were built on Linux x86-64; the wheel targets CPython 3.12.
Build from source for other environments. Nothing has been published to PyPI or
crates.io. License: MIT; third-party dependencies retain their own licenses.
