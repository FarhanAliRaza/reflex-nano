"""Serve the actual Reflex ASGI app and its production static frontend."""
import asyncio
import os
os.environ['REFLEX_SKIP_COMPILE'] = 'true'
from granian.server.embed import Server
from granian.constants import Interfaces
from lifecycle_app import app
from reflex.compiler.compiler import compile_app
from reflex.utils.exec import _frontend_prod_app

compile_app(app, dry_run=False, use_rich=False)  # backend page/state evaluation only

async def main():
    await asyncio.gather(
        Server(app(), address='127.0.0.1', port=3356, interface=Interfaces.ASGI, log_enabled=False).serve(),
        Server(_frontend_prod_app(), address='127.0.0.1', port=3355,
               interface=Interfaces.ASGI, log_enabled=False).serve(),
    )

if __name__ == '__main__':
    asyncio.run(main())
