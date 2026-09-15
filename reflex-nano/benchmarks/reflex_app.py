import os
import reflex as rx
from shared import state_class

RuntimeState=state_class(rx)
app=rx.App(enable_state=True)
app.add_page(lambda:rx.el.div(rx.el.p(RuntimeState.count),rx.el.p(RuntimeState.doubled),
    rx.el.p(RuntimeState.remaining),rx.el.p(RuntimeState.name),rx.el.p(RuntimeState.progress),
    rx.foreach(RuntimeState.items,lambda item:rx.el.p(item['label'],key=item['id']))),route='/')

if __name__=='__main__':
    import json
    import asyncio
    from granian.server.embed import Server
    from granian.constants import Interfaces
    from reflex.config import get_config
    # Compile the actual Python plugin pipeline once before starting the backend.
    # Runtime measurements exclude both frameworks' startup/compiler work.
    from reflex.compiler.compiler import compile_app
    compile_app(app,dry_run=True,use_rich=False)
    os.environ['REFLEX_SKIP_COMPILE']='true'
    backend=app()
    print('BENCH_INFO '+json.dumps({'state':RuntimeState.get_full_name(),
        'namespace':get_config().get_event_namespace(),'state_manager':type(app.state_manager).__name__,
        'server':'Granian embedded ASGI','runtime_threads':1}),flush=True)
    asyncio.run(Server(backend,address='127.0.0.1',port=int(os.environ['NANO_BENCH_PORT']),
        interface=Interfaces.ASGI,log_enabled=False).serve())
