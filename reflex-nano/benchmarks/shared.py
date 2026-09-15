"""Reflex baseline. The equivalent Nano application is defined in Rust demo.rs."""
import asyncio
import os


def state_class(rx):
    rows=int(os.environ.get('NANO_BENCH_ROWS','100'))
    class RuntimeState(rx.State):
        count:int=0
        name:str='Nano'
        items:list[dict]=[{'id':i,'label':f'Item {i}','done':False} for i in range(rows)]
        progress:int=0

        @rx.var
        def doubled(self)->int: return self.count*2

        @rx.var
        def remaining(self)->int: return sum(not item['done'] for item in self.items)

        @rx.event
        def increment(self,amount:int): self.count+=amount

        @rx.event
        async def increment_async(self,amount:int):
            await asyncio.sleep(0)
            self.count+=amount

        @rx.event
        def rename(self,name:str): self.name=name

        @rx.event
        def toggle(self,index:int): self.items[index]['done']=not self.items[index]['done']

        @rx.event
        def submit(self,data:dict): self.name=data['name'].strip()

        @rx.event
        def stream(self,steps:int):
            for _ in range(steps):
                self.count+=1
                yield

        @rx.event(background=True)
        async def background(self,steps:int):
            for _ in range(steps):
                await asyncio.sleep(.02)
                async with self:
                    self.progress+=1

        @rx.event
        def chain(self): return [RuntimeState.increment(2),RuntimeState.increment(3)]

    return RuntimeState


ARGUMENTS={'increment':['amount'],'increment_async':['amount'],'rename':['name'],
    'toggle':['index'],'submit':['data'],'stream':['steps'],'background':['steps'],'chain':[]}
PUBLIC_FIELDS=['count','name','items','progress','doubled','remaining']
