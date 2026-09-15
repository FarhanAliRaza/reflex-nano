"""The application is defined in crates/nano-core/src/demo.rs."""
from reflex_nano import App

app = App.dashboard()

if __name__ == "__main__":
    app.run()
