"""Python only starts the Rust-defined application; no Python state or handlers."""
import os
from reflex_nano import App

app=App.benchmark(int(os.environ.get("NANO_BENCH_ROWS","100")))
if __name__=="__main__":
    app.run(port=int(os.environ["NANO_BENCH_PORT"]))
