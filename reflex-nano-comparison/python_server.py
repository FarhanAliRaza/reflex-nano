import os
from pathlib import Path
from reflex_nano import App
app=App.from_json(Path(os.environ['NANO_MANIFEST']).read_text())
app.run(port=int(os.environ['NANO_BENCH_PORT']),renderer=os.environ['NANO_RENDERER'])
