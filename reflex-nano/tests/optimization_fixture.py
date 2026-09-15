"""Native definitions only; the exported fixture executes in the Rust runner."""
import json
from pathlib import Path
import sys
import os
from reflex_nano import App, Expr, Node

literal = lambda value: Expr.literal(json.dumps(value))
text = lambda value: Node.text(value if isinstance(value, Expr) else literal(value))
def el(tag, children, id=None):
    node = Node.element(tag, children)
    return node.attr('id', literal(id)) if id else node


def fixture():
    # Editing exported native IR is an authoring operation; there are no Python handlers.
    source = json.loads(App.benchmark(3,renderer=os.environ.get('NANO_RENDERER','html')).to_json())
    items = source['schema']['fields']['items']['initial']
    groups = [{'label':'Group A','children':[{'label':'x'},{'label':'y'}]},
              {'label':'Group B','children':[{'label':'z'}]}]
    source['schema']['fields']['groups'] = {'kind':'list','initial':groups}
    def set_program(field, value):
        return {'actions':[{'op':'set','field':field,'value':{'op':'literal','value':value}}]}
    source['schema']['events'].update({
        'reorder':set_program('items',[items[1],items[2],items[0]]),
        'empty':set_program('items',[]),
        'restore':set_program('items',items),
        'nested':set_program('groups',[{'label':'Changed','children':[{'label':'q'}]}]),
    })
    app=App.from_json(json.dumps(source))
    def button(id,event='RuntimeState.increment',args=None,**options):
        return el('button',[text(id)],id).attr('type',literal('button')).on(
            'click',event,[literal(1)] if args is None else args,False,**options)
    item=Expr.local('item')
    row=el('div',[
        el('span',[text(item.get(literal('label')))]).attr('class',literal('label')),
        el('input',[]).attr('value',item.get(literal('label'))).attr('class',literal('edit')),
        el('input',[]).attr('type',literal('checkbox')).attr('checked',item.get(literal('done')))
            .on('change','RuntimeState.toggle',[Expr.local('index')],False),
    ]).key(item.get(literal('id'))).attr('data-id',item.get(literal('id')))
    nested=Node.each(Expr.state('groups'),'item','index',el('section',[
        text(item.get(literal('label'))),
        Node.each(item.get(literal('children')),'item','index',el('b',[text(item.get(literal('label')))])),
        text(item.get(literal('label'))),
    ]))
    app.add_page('/opt','Optimization regressions',el('main',[
        el('p',[text(Expr.state('count'))],'count'),el('p',[text(Expr.state('doubled'))],'doubled'),
        el('p',[text(Expr.state('remaining'))],'remaining'),
        el('p',[text(''),text('A'),text(Expr.state('name')),text('Z')],'adjacent'),
        el('p',[text(literal(2).binary('mul',literal(3)))],'constant'),
        el('input',[],'early-input').attr('value',Expr.state('name')),
        el('input',[],'debounced-name').attr('value',Expr.state('name'))
            .on('input','RuntimeState.rename',[Expr.local('event').get(literal('value'))],False,debounce_ms=80),
        button('increment'),button('debounce',debounce_ms=80),button('throttle',throttle_ms=200),
        button('temporal',temporal=True),
        el('div',[button('stop',stop_propagation=True),button('bubble')],'ancestor')
            .on('click','RuntimeState.increment',[literal(2)],False),
        button('reorder','reorder',[]),button('empty','empty',[]),button('restore','restore',[]),
        button('nested','nested',[]),
        Node.when(Expr.state('count').binary('gt',literal(0)),
            Node.fragment([el('p',[text('visible')],'conditional'),text('!')]),Node.fragment([])),
        el('div',[Node.each(Expr.state('items'),'item','index',row)],'rows'),
        el('div',[nested],'groups'),
    ]))
    return app


if __name__=='__main__':
    Path(sys.argv[1]).write_text(fixture().to_json())
