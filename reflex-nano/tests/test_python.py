import json
import pytest
import reflex_nano as rn


def test_renderer_selection_translation_and_round_trip():
    app = counter()
    assert app.renderer == 'html'
    app.set_renderer('react')
    node = rn.Node.element('label', [rn.Node.text(rn.Expr.state('count'))]).attr(
        'class', rn.Expr.literal('"label"')).attr('for', rn.Expr.literal('"name"')).on(
        'click', 'add', [rn.Expr.literal('1')])
    app.add_page('/translated', 'React', node)
    app.add_page('/direct', 'HTML', node, renderer='html')
    restored = rn.App.from_json(app.to_json())
    assert restored.renderer == 'react'
    plan = json.loads(restored.render_plan_json('/translated'))
    assert plan['attrs']['className']['value'] == 'label'
    assert plan['attrs']['htmlFor']['value'] == 'name'
    assert 'onClick' in plan['events']
    assert 'click' in json.loads(restored.render_plan_json('/direct'))['events']
    with pytest.raises(ValueError, match='Renderer'):
        app.set_renderer('unknown')
    assert app.renderer == 'react'


def test_react_components_and_html_fallbacks_are_native():
    fallback = rn.Node.text(rn.Expr.literal('"Fallback"'))
    component = rn.Node.component('@radix-ui/themes', 'Button', [fallback]).fallback(fallback)
    app = rn.App(renderer='react')
    app.add_page('/', 'React', component)
    assert app.render() == 'Fallback'
    assert json.loads(app.render_plan_json())['kind'] == 'component'
    app.set_renderer('html')
    assert json.loads(app.render_plan_json())['kind'] == 'text'
    assert app.render() == 'Fallback'
    with pytest.raises(ValueError, match='fallback'):
        rn.App().add_page('/', 'Bad', rn.Node.component('@radix-ui/themes', 'Button', []))
    with pytest.raises(ValueError, match='not registered'):
        rn.App(renderer='react').add_page('/', 'Bad', rn.Node.component('missing', 'Unknown', []))


def test_native_render_plan_and_event_options_round_trip():
    button = rn.Node.element('button', [rn.Node.text(rn.Expr.state('count'))]).on(
        'click', 'add', [rn.Expr.literal('1')], True,
        debounce_ms=80, throttle_ms=200, temporal=True, stop_propagation=True)
    app = counter()
    app.add_page('/options', 'Options', button)
    restored = rn.App.from_json(app.to_json())
    plan = json.loads(restored.render_plan_json('/options'))
    assert plan['_s'] == ['count'] and plan['_l'] == []
    event = plan['events']['click']
    assert event['debounce_ms'] == 80 and event['throttle_ms'] == 200
    assert event['temporal'] and event['stop_propagation'] and event['prevent_default']


def counter():
    schema=rn.Schema()
    schema.field("count","int","0")
    schema.computed("doubled",rn.Expr.state("count").binary("mul",rn.Expr.literal("2")))
    schema.event("add",rn.Program([rn.Action.set("count",rn.Expr.state("count").binary("add",rn.Expr.local("amount")))],
        '[{"name":"amount","kind":"int"}]'))
    app=rn.App(schema)
    app.add_page("/","Counter",rn.Node.text(rn.Expr.state("doubled")))
    return app


def test_python_is_only_native_bindings():
    for name in ('App','Schema','Program','Action','Expr','Node'):
        assert getattr(rn,name).__module__=='reflex_nano._native'
    assert not hasattr(rn,'State') and not hasattr(rn,'event')
    app=counter()
    assert not hasattr(app,'add_handler') and not hasattr(app,'add_context_handler')
    with pytest.raises(TypeError):rn.Program([lambda:None])


def test_native_state_events_and_computed():
    app=counter()
    result=json.loads(app.dispatch('add','[3]'))
    assert result['state']=={'count':3} and result['view']['doubled']==6
    assert app.render()=='0'
    assert json.loads(app.dispatch('add','[2]',json.dumps(result['state'])))['state']['count']==5


@pytest.mark.parametrize('args',['[true]','["3"]','[]','[1,2]','[9007199254740992]'])
def test_argument_validation_is_in_rust(args):
    with pytest.raises(ValueError):counter().dispatch('add',args)


def test_export_round_trip_owns_definitions():
    app=counter();restored=rn.App.from_json(app.to_json())
    assert restored.to_json()==app.to_json()
    assert json.loads(restored.dispatch('add','[4]'))['view']['doubled']==8
    source=json.loads(app.to_json());source['schema']['fields']['count']['initial']=10
    assert json.loads(rn.App.from_json(json.dumps(source)).initial_state_json())['count']==10
    assert json.loads(app.initial_state_json())['count']==0


def test_bad_manifest_and_computed_cycles():
    source=json.loads(counter().to_json())
    source['schema']['fields']['count']['initial']='wrong'
    with pytest.raises(ValueError):rn.App.from_json(json.dumps(source))
    source=json.loads(counter().to_json())
    source['schema']['computed']={'a':{'op':'state','name':'b'},'b':{'op':'state','name':'a'}}
    with pytest.raises(ValueError,match='cycle'):rn.App.from_json(json.dumps(source))


def test_native_program_transaction_rollback():
    source=json.loads(counter().to_json())
    source['schema']['events']['fail']={'actions':[
        {'op':'set','field':'count','value':{'op':'literal','value':99}},
        {'op':'require','condition':{'op':'literal','value':False},'message':'rollback'}]}
    app=rn.App.from_json(json.dumps(source));state=app.initial_state_json()
    with pytest.raises(ValueError,match='rollback'):app.dispatch('fail',state_json=state)
    assert json.loads(app.dispatch('add','[1]',state))['state']['count']==1


def test_native_nested_mutation_and_backend_chain():
    app=rn.App.benchmark(3)
    result=json.loads(app.dispatch('RuntimeState.toggle','[1]'))
    assert result['state']['items'][1]['done'] is True and result['view']['remaining']==2
    result=json.loads(app.dispatch('RuntimeState.chain'))
    assert result['state']['count']==5 and result['effects']==[]


def test_nodes_and_expressions_are_native():
    app=rn.App.dashboard()
    assert 'Rust all the way' in app.render()
    assert 'Task ID: 42' in app.render('/item/42')
    text=rn.Node.text(rn.Expr.literal(json.dumps('<script>bad</script>')))
    assert text.render('{}')=='&lt;script&gt;bad&lt;/script&gt;'
    with pytest.raises(ValueError):rn.App().add_page('/','Bad',rn.Node.element('script',[]))
    fixtures=json.loads((__import__('pathlib').Path(__file__).parent/'expression_cases.json').read_text())
    for fixture in fixtures:
        actual=json.loads(rn.Expr.from_json(json.dumps(fixture['expression'])).evaluate(json.dumps(fixture['state'])))
        assert actual==fixture['expected'],fixture['name']
