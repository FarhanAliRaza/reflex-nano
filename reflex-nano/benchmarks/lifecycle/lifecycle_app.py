"""Real Reflex frontend and backend, with the shared runtime benchmark state."""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import reflex as rx
from shared import state_class

RuntimeState = state_class(rx)


def row(item, index):
    return rx.el.div(
        rx.el.span(item['id'], class_name='item-id'),
        rx.el.span(item['label'], class_name='item-label'),
        rx.el.span(rx.cond(item['done'], 'true', 'false'), class_name='item-done'),
        rx.el.button('Toggle', type='button', on_click=RuntimeState.toggle(index)),
        class_name='item', key=item['id'],
    )


app = rx.App(enable_state=True)
app.add_page(lambda: rx.el.main(
    *[rx.el.p(getattr(RuntimeState, name), id=name)
      for name in ['count', 'doubled', 'remaining', 'name', 'progress']],
    rx.el.button('Increment', id='increment', type='button', on_click=RuntimeState.increment(1)),
    rx.el.section(rx.foreach(RuntimeState.items, row), id='items'),
    rx.el.span(rx.cond(rx.State.is_hydrated, 'true', 'false'), id='hydrated', hidden=True),
), route='/bench', title='Lifecycle benchmark')
