"""Build native UI definitions through PyO3; state/programs come from Rust demo.rs.

The exported manifest runs in dist/nano with no Python process or callbacks.
"""
import json
import os
from reflex_nano import App, Expr, Node


def fixture(rows):
    app = App.benchmark(rows, renderer=os.environ.get("NANO_RENDERER", "html"))
    literal = lambda value: Expr.literal(json.dumps(value))
    element = Node.element
    text = Node.text
    def field(name):
        return element('p', [text(Expr.state(name))]).attr('id', literal(name))
    item = Expr.local('item')
    row = element('div', [
        element('span', [text(item.get(literal('id')))]).attr('class', literal('item-id')),
        element('span', [text(item.get(literal('label')))]).attr('class', literal('item-label')),
        element('span', [text(Expr.choose(item.get(literal('done')), literal('true'), literal('false')))])
            .attr('class', literal('item-done')),
        element('button', [text(literal('Toggle'))]).attr('type', literal('button'))
            .on('click', 'RuntimeState.toggle', [Expr.local('index')], False),
    ]).attr('class', literal('item')).key(item.get(literal('id')))
    app.add_page('/bench', 'Lifecycle benchmark', element('main', [
        *[field(name) for name in ['count', 'doubled', 'remaining', 'name', 'progress']],
        element('button', [text(literal('Increment'))]).attr('id', literal('increment'))
            .attr('type', literal('button')).on('click', 'RuntimeState.increment', [literal(1)], False),
        element('section', [Node.each(Expr.state('items'), 'item', 'index', row)])
            .attr('id', literal('items')),
    ]))
    return app


if __name__ == '__main__':
    import os
    from pathlib import Path
    app = App.from_json(Path(os.environ['NANO_MANIFEST']).read_text())
    app.run(port=int(os.environ.get('NANO_BENCH_PORT', '3355')), renderer=os.environ.get('NANO_RENDERER', 'html'))
