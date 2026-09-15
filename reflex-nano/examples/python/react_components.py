"""Native definitions through bindings; all application state/programs execute in Rust."""
import json
from reflex_nano import App, Expr, Node, Action, Program, Schema

lit=lambda value:Expr.literal(json.dumps(value))
state=Expr.state
value=Expr.local('event').get(lit('value'))
def text(value):return Node.text(value if isinstance(value,Expr) else lit(value))
def html(tag,children,id=None):
    n=Node.element(tag,children)
    return n.attr('id',lit(id)) if id else n
def radix(name,children=(),id=None):
    n=Node.component('@radix-ui/themes',name,list(children))
    return n.attr('id',lit(id)) if id else n

schema=Schema()
for name,kind,initial in [('count','int',0),('name','string','Nano'),('checked','bool',False),
                          ('choice','string','one'),('open','bool',False)]:
    schema.field(name,kind,json.dumps(initial))
    schema.event('set_'+name,Program([Action.set(name,Expr.local('value'))],
                 json.dumps([{'name':'value','kind':kind}])))
schema.computed('doubled',state('count').binary('mul',lit(2)))
schema.event('increment',Program([Action.set('count',state('count').binary('add',lit(1)))]))

def fallback():
    return html('main',[
        html('h1',[text('Direct HTML + Rust')]),
        html('p',[text(state('count'))],'count'),
        html('button',[text('Increment')],'increment').on('click','increment',[]),
        html('a',[text('React widgets')],'to-widgets').attr('href',lit('/widgets')),
    ])

app=App(schema,renderer='react')
app.add_page('/','Direct HTML',fallback(),renderer='html')
widgets=radix('Theme',[
    radix('Flex',[
        radix('Heading',[text('React components + Rust state')]),
        radix('Text',[text(state('count'))],'count'),
        radix('Text',[text(state('doubled'))],'doubled'),
        radix('Button',[text('Increment')],'increment').on('click','increment',[]),
        radix('TextField.Root',id='name').attr('value',state('name')).on('change','set_name',[value]),
        radix('Text',[text(state('name'))],'name-value'),
        radix('Switch',id='native-switch').attr('checked',state('checked')).on('checked_change','set_checked',[value]),
        radix('Text',[text(Expr.choose(state('checked'),lit('on'),lit('off')))],'checked-value'),
        radix('Select.Root',[
            radix('Select.Trigger',id='choice'),
            radix('Select.Content',[
                radix('Select.Item',[text('First')]).attr('value',lit('one')),
                radix('Select.Item',[text('Second')]).attr('value',lit('two')),
            ]),
        ]).attr('value',state('choice')).on('value_change','set_choice',[value]),
        radix('Text',[text(state('choice'))],'choice-value'),
        radix('Dialog.Root',[
            radix('Dialog.Trigger',[radix('Button',[text('Open dialog')],'open-dialog')]),
            radix('Dialog.Content',[
                radix('Dialog.Title',[text('Native event inside a React portal')]),
                radix('Dialog.Description',[text('This button updates Rust state over WebSockets.')]),
                radix('Button',[text('Portal increment')],'portal-increment').on('click','increment',[]),
                radix('Dialog.Close',[radix('Button',[text('Close')],'close-dialog')]),
            ]),
        ]).attr('open',state('open')).on('open_change','set_open',[value]),
        Node.component('nano/demo','ClientCounter',[]).attr('id',lit('custom-counter'))
            .attr('value',state('count')).on('value_change','set_count',[value]),
        html('a',[text('Direct HTML page')],'to-html').attr('href',lit('/')),
    ]).attr('direction',lit('column')).attr('gap',lit('3')).style('padding',lit('24px')),
]).fallback(fallback())
app.add_page('/widgets','React widgets',widgets)

if __name__=='__main__':
    import sys
    if len(sys.argv)>1:
        from pathlib import Path
        Path(sys.argv[1]).write_text(app.to_json())
    else:app.run()
