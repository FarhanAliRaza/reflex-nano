"""Real WebSocket clients and isolated backend processes; no mocked dispatch paths."""
import asyncio
import contextlib
import json
import os
from pathlib import Path
import sys
import time
import uuid

import aiohttp
from shared import ARGUMENTS, PUBLIC_FIELDS

HERE=Path(__file__).resolve().parent


class Backend:
    def __init__(self,kind,rows,port=3345,python_host=False,manifest=None):
        self.kind,self.rows,self.port=kind,rows,port
        self.info={};self.logs=[]
        self.url=f'http://127.0.0.1:{port}'
        self.python_host=python_host;self.manifest=manifest
    async def __aenter__(self):
        env={**os.environ,'NANO_BENCH_PORT':str(self.port),'NANO_BENCH_ROWS':str(self.rows),
             'REFLEX_TELEMETRY_ENABLED':'false','PYTHONUNBUFFERED':'1'}
        command=[sys.executable,f'{self.kind}_app.py']
        if self.kind=='nano' and not self.python_host:
            command=[os.environ.get('NANO_RUNNER',str(HERE.parent/'target/release/examples/runner')),
                *(['--manifest',str(self.manifest)] if self.manifest else ['--benchmark'])]
            self.info={'host':'standalone Rust executable','python_runtime':False}
        elif self.kind=='nano':self.info={'host':'PyO3 binding host; GIL released during serving','python_callbacks':False}
        self.command=command
        self.process=await asyncio.create_subprocess_exec(*command,
            cwd=HERE,env=env,stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.STDOUT)
        async def read_log():
            async for line in self.process.stdout:
                line=line.decode(errors='replace').rstrip();self.logs.append(line)
                if line.startswith('BENCH_INFO '):self.info.update(json.loads(line[11:]))
        self.log_task=asyncio.create_task(read_log())
        try:
            async with aiohttp.ClientSession() as http:
                for _ in range(300):
                    if self.process.returncode is not None:raise RuntimeError('\n'.join(self.logs))
                    try:
                        async with http.get(self.url+('/ping' if self.kind=='reflex' else '/')) as r:
                            if r.status==200 and (self.kind=='nano' or self.info):return self
                    except aiohttp.ClientError:pass
                    await asyncio.sleep(.1)
            raise TimeoutError('\n'.join(self.logs))
        except BaseException:
            await self.__aexit__(None,None,None);raise
    async def __aexit__(self,*exc):
        if self.process.returncode is None:
            self.process.terminate()
            try:await asyncio.wait_for(self.process.wait(),8)
            except asyncio.TimeoutError:self.process.kill();await self.process.wait()
        await self.log_task


def initial(rows):
    return dict(count=0,name='Nano',items=[dict(id=i,label=f'Item {i}',done=False) for i in range(rows)],
                progress=0,doubled=0,remaining=rows)


class Client:
    def __init__(self,backend,http=None):
        self.backend=backend;self.kind=backend.kind;self.http=http;self.owns_http=http is None
        self.state={};self.condition=asyncio.Condition();self.receipts={};self.error=None
        self.sent_bytes=0;self.received_bytes=0;self.history=[];self.record=False;self.sequence=0
        self.token=uuid.uuid4().hex
    async def __aenter__(self):
        if self.http is None:self.http=aiohttp.ClientSession(cookie_jar=aiohttp.CookieJar(unsafe=True))
        try:
            await self.connect();return self
        except BaseException:
            await self.__aexit__(None,None,None);raise
    async def connect(self):
        if self.kind=='nano':
            async with self.http.get(self.backend.url+'/__nano/page?path=/') as response:
                response.raise_for_status();boot=await response.json()
            self.csrf=boot['csrf']
            self.ws=await self.http.ws_connect(self.backend.url+'/__nano/ws',origin=self.backend.url,protocols=['nano.v1'],compress=0)
            await self._send(json.dumps(dict(type='hello',csrf=self.csrf,path='/'),separators=(',',':')))
            self._consume(json.loads(await self._receive()))
        else:
            ns=self.backend.info['namespace']
            self.ws=await self.http.ws_connect(self.backend.url+f'/_event/?EIO=4&transport=websocket&token={self.token}',
                protocols=['0.9.11'],compress=0)
            assert (await self._receive()).startswith('0')
            await self._send('40'+ns+',')
            assert (await self._receive()).startswith('40'+ns+',')
        self.reader=asyncio.create_task(self._read())
        if self.kind=='reflex':
            await self.emit_raw(self.backend.info['state'].split('.')[0]+'.hydrate',{})
            await self.wait(lambda s:all(k in s for k in PUBLIC_FIELDS))
    async def _send(self,text):
        self.sent_bytes+=len(text.encode());await self.ws.send_str(text)
    async def _receive(self):
        message=await self.ws.receive(timeout=10)
        if message.type!=aiohttp.WSMsgType.TEXT:raise ConnectionError(f'WebSocket closed: {message}')
        self.received_bytes+=len(message.data.encode());return message.data
    def _consume(self,message):
        if self.kind=='nano':
            kind=message['type']
            if kind=='snapshot':self.state=message['state'];self.version=message['version']
            elif kind=='update':
                self.state.update(message['delta'])
                for name in message['removed']:self.state.pop(name,None)
                self.version=message['version']
            elif kind=='ack':self.receipts[message['id']]=message
            elif kind in ('error','background_error'):self.error=RuntimeError(message['error'])
        else:
            fields=message.get('delta',{}).get(self.backend.info['state'],{})
            self.state.update({k.removesuffix('_rx_state_'):v for k,v in fields.items()})
            for event in message.get('events',[]):
                if event['name'] in ('_alert','_toast'):
                    self.error=RuntimeError(str(event))
        if self.record:self.history.append({k:v for k,v in self.state.items() if k!='items'})
    async def _read(self):
        try:
            while True:
                text=await self._receive()
                if self.kind=='reflex':
                    if text=='2':await self._send('3');continue
                    prefix='42'+self.backend.info['namespace']+','
                    if not text.startswith(prefix):continue
                    packet=json.loads(text[len(prefix):])
                    if packet[0]!='event':continue
                    message=packet[1]
                else:message=json.loads(text)
                self._consume(message)
                async with self.condition:self.condition.notify_all()
        except asyncio.CancelledError:raise
        except Exception as exc:
            self.error=exc
            async with self.condition:self.condition.notify_all()
    async def wait(self,predicate):
        async with self.condition:
            await asyncio.wait_for(self.condition.wait_for(lambda:self.error is not None or predicate(self.state)),10)
            if self.error:raise self.error
    async def emit_raw(self,name,payload):
        message=['event',dict(name=name,payload=payload,router_data={'pathname':'/'})]
        await self._send('42'+self.backend.info['namespace']+','+json.dumps(message,separators=(',',':')))
    async def emit(self,method,args=(),request_id=None):
        self.sequence+=1;request_id=request_id or f'{self.token}-{self.sequence}'
        if self.kind=='nano':
            await self._send(json.dumps(dict(type='event',id=request_id,name='RuntimeState.'+method,args=args,path='/'),separators=(',',':')))
        else:
            await self.emit_raw(self.backend.info['state']+'.'+method,dict(zip(ARGUMENTS[method],args)))
        return request_id
    async def ack(self,request_id):
        if self.kind!='nano':return None
        await self.wait(lambda _:request_id in self.receipts)
        return self.receipts.pop(request_id)
    async def event(self,method,args,predicate):
        begin=time.perf_counter_ns()
        request_id=await self.emit(method,args)
        await self.wait(predicate)
        elapsed=time.perf_counter_ns()-begin
        if self.kind=='nano':
            reply=await self.ack(request_id)
            if not reply['ok']:raise RuntimeError(reply['error'])
        return elapsed/1e6
    def public(self):return {k:self.state[k] for k in PUBLIC_FIELDS}
    async def disconnect(self):
        if hasattr(self,'reader'):
            self.reader.cancel()
            with contextlib.suppress(asyncio.CancelledError):await self.reader
        if hasattr(self,'ws'):await self.ws.close()
    async def __aexit__(self,*exc):
        await self.disconnect()
        if self.owns_http and self.http:await self.http.close()


async def contract(backend):
    checks=[]
    async with Client(backend) as client:
        assert client.public()==initial(backend.rows),client.public()
        await client.event('increment',[2],lambda s:s.get('count')==2 and s.get('doubled')==4)
        await client.event('increment_async',[3],lambda s:s.get('count')==5 and s.get('doubled')==10)
        await client.event('rename',['Parity'],lambda s:s.get('name')=='Parity')
        await client.event('submit',[{'name':'  Form  '}],lambda s:s.get('name')=='Form')
        if backend.rows:
            await client.event('toggle',[0],lambda s:s.get('remaining')==backend.rows-1 and s['items'][0]['done'])
        checks.extend(['initial state','sync and async events','computed values','controlled input','form payload',
            'nested list mutation' if backend.rows else 'nested mutation skipped for empty-list workload'])
        client.record=True;client.history=[]
        await client.event('stream',[3],lambda s:s.get('count')==8 and s.get('doubled')==16)
        assert {6,7,8}.issubset({h.get('count') for h in client.history}),client.history
        checks.append('generator checkpoints delivered before completion')
        task=await client.emit('background',[3]);await client.ack(task)
        await client.event('increment',[1],lambda s:s.get('count')==9)
        await client.wait(lambda s:s.get('progress')==3)
        assert {1,2,3}.issubset({h.get('progress') for h in client.history}),client.history
        checks.append('background push with foreground events')
        task=await client.emit('chain')
        if backend.kind=='nano':
            reply=await client.ack(task)
            assert reply['ok'],reply
            for effect in reply['effects']:
                count=client.state['count']+effect['args'][0]
                await client.event(effect['name'].split('.')[-1],effect['args'],lambda s:s.get('count')==count)
        await client.wait(lambda s:s.get('count')==14 and s.get('doubled')==28)
        checks.append('returned event chain')
        await client.disconnect();await client.connect()
        assert client.state['count']==14,client.state
        checks.append('reconnect state retained')
        async with Client(backend) as isolated:
            assert isolated.public()==initial(backend.rows)
        checks.append('session isolation')
    return checks


async def main():
    for kind in ('nano','reflex'):
        async with Backend(kind,10) as backend:
            try:print(kind,await contract(backend),flush=True)
            except BaseException:
                print('\n'.join(backend.logs));raise

if __name__=='__main__':asyncio.run(main())
