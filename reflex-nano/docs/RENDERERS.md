# React and direct HTML rendering

Nano 0.4 has two browser renderers over one Rust runtime. `html` remains the
default. The authoritative state, event programs, computed values, session store,
WebSocket server and UI definitions are Rust in both modes. Python constructs
and exports those native objects through PyO3; it registers no Python handlers.

## Select a renderer

```python
from reflex_nano import App
app = App.dashboard(renderer="react")  # or "html"
app.run()
```

For custom schemas use `App(schema, renderer="react")`. `app.set_renderer(mode)`
validates the application before changing its default. `app.add_page(route,
title, node, renderer="html")` overrides one page. Export/import preserves both
settings. `app.run(renderer="react")` overrides the application default for
that invocation; explicit page choices still win.

```rust
use reflex_nano::{dashboard_application, Renderer};
let mut definition = dashboard_application()?;
definition.renderer = Renderer::React; // Renderer::Html uses the direct adapter
let runtime = definition.build()?;
```

The Rust `Application.page_with_renderer` and runtime `App.add_page_with_renderer`
provide per-page selection. Set the lower-level runtime `App.renderer` before
registering pages. Runnable examples: `examples/rust/react.rs` and
`examples/python/react_components.py`.

```bash
PORT=3000 ./dist/nano --renderer react
PORT=3000 ./dist/nano --renderer html
python -m reflex_nano run examples/python/react_components.py --port 3000
cargo run --release --locked -p reflex-nano --example react
NANO_RENDERER=html cargo run --release --locked -p reflex-nano --example react
```

Both launchers accept `NANO_RENDERER`. A navigation between different renderers
loads a new document and retains the same HttpOnly session cookie and Rust state.
Navigation within one renderer continues through the existing SPA route path.

## What Rust translates

Rust validates immutable `Node` trees and compiles an SSR operation plan and a
browser plan. It folds constant expressions and annotates state/lexical
dependencies. In React mode it additionally translates HTML attributes, styles
and event names into React props: `class → className`, `for → htmlFor`,
`background-color → backgroundColor`, `click → onClick`, and
`value_change → onValueChange`. `data-*`, `aria-*` and CSS variables are preserved.
The browser uses those translated props directly with real React elements.

| Behavior | `html` | `react` |
| --- | --- | --- |
| Native HTML nodes | Rust SSR, direct DOM adoption/updates | Rust SSR, React `hydrateRoot` and reconciliation |
| Keyed iteration | Native plan, direct keyed DOM update | Native plan, stable React keys and dependency memoization |
| Event handlers | Delegated DOM events | React synthetic events or component callbacks |
| Events/state transport | Rust WebSockets | The same Rust WebSockets |
| Controlled inputs | Direct input binding | Immediate React input draft, authoritative value from Rust |
| Imported React component | Its explicit native HTML fallback | Fallback during SSR/first hydration, then browser component |
| React package download | None | Core React adapter; component/CSS chunk loaded only when needed |

The browser necessarily executes JavaScript for React and DOM interaction. Input
drafts, focus, animation, component hooks and other transient UI state live in
the browser. Application state and registered event execution remain in Rust.

## React components

`Node.component(library, export_name, children)` identifies a component in the
build-time registry. Use dotted exports such as `TextField.Root`, `Dialog.Root`
and `Select.Item`. `attr`, `style`, `on`, `key` and `fallback` work on native
component nodes. Wrap Radix Themes components in its `Theme` provider.

```python
from reflex_nano import Node, Expr
button = Node.component("@radix-ui/themes", "Button", [
    Node.text(Expr.state("count"))
]).on("click", "increment", [])
```

A callback such as `onValueChange(value)` is available to native argument
expressions as `Expr.local("event").get(Expr.literal('"value"'))`. All positional
callback arguments are also exposed in `event.args`. Native DOM events expose
value, checked and key; form submissions retain the shared FormData projection.
The event must name a registered Rust program/handler. Existing debounce,
throttle, temporal discard, prevent-default and stop-propagation settings work
through the shared dispatcher.

Refs and injected handlers are composed through the node adapter, allowing
Radix `asChild`/trigger composition. React portal events dispatch directly even
when the element is outside `#nano-root`.

Imported components can use ordinary React hooks and context. The adapter
exports `NanoStateContext`, `NanoEventContext`, `useNanoState()` and
`useNanoEvents()` from `frontend/react.jsx`; the same exports are available on
`window.__NANO_REACT__`. The state context contains a native public snapshot.
The event hook dispatches `(registeredName, argumentArray)` to Rust. Treat
snapshots as read-only; mutating a browser snapshot does not modify Rust state.

## Explicit HTML fallbacks

Rust does not execute arbitrary third-party JavaScript to generate SSR HTML.
Provide a native fallback with `.fallback(node)` on a component or enclosing
component subtree. React hydrates that exact fallback before activating the
imported components. Without a fallback React mode starts that component empty;
HTML mode rejects an imported component without an explicit fallback.

The fallback can itself contain state expressions and native event bindings.
The renderer test uses an interactive HTML counter as the fallback for a Radix
page. This is a client component island strategy, not full third-party React SSR.
React server components, streaming React SSR and Suspense data loaders are not
implemented.

## Add a package or custom component

1. Add the dependency with an exact version to `package.json` and update the lock.
2. Import/register its exports in `frontend/components-registry.js`; import its
   CSS in `frontend/components.js` when needed. Capitalized function/React-object
   exports and namespace members become the permitted native registry.
3. Run `npm ci` and `npm run build:frontend`, then rebuild the Rust binary/wheel.

The bundled registry contains Radix Themes and the small `nano/demo` example.
Unknown registry names fail at native compilation. Arbitrary package names in a
manifest are not downloaded or evaluated by the server. This explicit registry
also avoids duplicating React versions.

The minified, split frontend and license inventory are checked in under
`crates/nano-core/frontend-dist`. Ordinary Rust/Python app builds need no Node
process, npm installation, CDN or network dependency at runtime. A registry or
adapter change requires a frontend build before Rust compilation. Run
`python benchmarks/verify_frontend.py` to reject stale bundles; release scripts
run this check automatically.

## Compatibility boundary

This reuses real React, ReactDOM and Radix behavior with Nano's native compiler
and event transport. It adapts the relevant event modifiers and reactive state
flow; it does not import an entire generated Reflex frontend unchanged. Reflex's
Socket.IO protocol, generated StateContexts, Python wrapper API and all client
utilities are not drop-in compatible. See `PARITY.md` for the remaining gaps.

References used for the integration:
- https://react.dev/reference/react-dom/client/hydrateRoot
- https://reflex.dev/docs/wrapping-react/overview/
- Installed Reflex 0.9.11 generated `utils/state.js` and `utils/context.jsx` in
  the benchmark application, for event modifier and provider behavior.
