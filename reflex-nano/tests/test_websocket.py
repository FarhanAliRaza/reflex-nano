"""Real TCP/WebSocket correctness and architecture checks."""
import asyncio
import json
from pathlib import Path
import sys
import subprocess

import aiohttp
import pytest
import reflex_nano as rn

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'benchmarks'))
from driver import Backend,Client,contract


@pytest.mark.parametrize('python_host',[False,True])
def test_runtime_contract_through_both_hosts(python_host):
    async def run():
        async with Backend('nano',10,port=3346,python_host=python_host) as backend:
            assert len(await contract(backend))==11
    asyncio.run(run())


def test_security_retries_and_concurrent_sessions():
    async def run():
        async with Backend('nano',10,port=3346) as backend:
            async with Client(backend) as first:
                with pytest.raises(aiohttp.WSServerHandshakeError) as origin:
                    await first.http.ws_connect(backend.url+'/__nano/ws',origin='https://evil.example')
                assert origin.value.status==403
                async with aiohttp.ClientSession() as anonymous:
                    with pytest.raises(aiohttp.WSServerHandshakeError) as cookie:
                        await anonymous.ws_connect(backend.url+'/__nano/ws',origin=backend.url)
                    assert cookie.value.status==401
                async with first.http.ws_connect(backend.url+'/__nano/ws',origin=backend.url) as bad:
                    await bad.send_json({'type':'hello','csrf':'wrong','path':'/'})
                    assert (await bad.receive_json())['type']=='error'
                request=await first.emit('increment',[1],request_id='duplicate')
                assert (await first.ack(request))['ok'];assert first.state['count']==1
                version=first.version
                assert (await first.ack(await first.emit('increment',[1],request_id='duplicate')))['ok']
                assert first.state['count']==1 and first.version==version
                assert not (await first.ack(await first.emit('increment',[9],request_id='duplicate')))['ok']
                assert not (await first.ack(await first.emit('increment',['wrong'])))['ok']
                assert first.state['count']==1
                async with Client(backend,http=first.http) as second:
                    assert second.state['count']==1
                    async def clicks(client):
                        for _ in range(30):
                            assert (await client.ack(await client.emit('increment',[1])))['ok']
                    await asyncio.gather(clicks(first),clicks(second))
                    await first.wait(lambda s:s['count']==61)
                    await second.wait(lambda s:s['count']==61)
                    await first.disconnect();await first.connect();assert first.state['count']==61
                    # An older retry must not roll the newly connected client back.
                    assert (await first.ack(await first.emit('increment',[1],request_id='duplicate')))['ok']
                    assert first.state['count']==61
                    # More than the task cap over time proves completed tasks release slots.
                    for i in range(20):
                        assert (await first.ack(await first.emit('background',[1])))['ok']
                        await first.wait(lambda s:s['progress']==i+1)
                    await second.wait(lambda s:s['progress']==20)
    asyncio.run(run())


@pytest.mark.skipif(sys.platform!='linux',reason='ELF linkage check requires Linux')
def test_exported_python_definition_runs_without_python(tmp_path):
    app=rn.App.benchmark(10)
    source=json.loads(app.to_json())
    source['schema']['events']['RuntimeState.failure']={'actions':[
        {'op':'set','field':'count','value':{'op':'literal','value':999}},
        {'op':'require','condition':{'op':'literal','value':False},'message':'rollback'}]}
    source['schema']['events']['RuntimeState.stream_failure']={'mode':{'kind':'streaming'},'actions':[
        {'op':'set','field':'count','value':{'op':'literal','value':3}},
        {'op':'publish'},
        {'op':'set','field':'count','value':{'op':'literal','value':999}},
        {'op':'require','condition':{'op':'literal','value':False},'message':'rollback after checkpoint'}]}
    source['schema']['events']['RuntimeState.bad_redirect']={'mode':{'kind':'streaming'},'actions':[
        {'op':'set','field':'count','value':{'op':'literal','value':777}},
        {'op':'redirect','path':{'op':'literal','value':'javascript:alert(1)'}}]}
    manifest=tmp_path/'native-app.json';manifest.write_text(json.dumps(source))
    async def run():
        async with Backend('nano',10,port=3346,manifest=manifest) as backend:
            exe=Path(backend.command[0])
            assert exe.read_bytes()[:4]==b'\x7fELF'
            libraries=subprocess.check_output(['ldd',str(exe)],text=True)
            assert 'libpython' not in libraries
            async with Client(backend) as client:
                assert not (await client.ack(await client.emit('failure')))['ok']
                assert client.state['count']==0
                await client.event('increment',[2],lambda s:s['count']==2)
                assert not (await client.ack(await client.emit('stream_failure')))['ok']
                assert client.state['count']==3
                assert not (await client.ack(await client.emit('bad_redirect')))['ok']
                assert client.state['count']==3
                await client.event('increment',[1],lambda s:s['count']==4)
    asyncio.run(run())
