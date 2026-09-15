import argparse
import importlib.util
from pathlib import Path
import sys
import os


def main():
    parser = argparse.ArgumentParser(prog="nano", description="Run a Reflex Nano Python app")
    parser.add_argument("command", choices=["run"])
    parser.add_argument("file", help="Python file exporting an 'app' object")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=3000)
    parser.add_argument("--renderer", choices=["html","react"], default=os.environ.get('NANO_RENDERER'))
    args = parser.parse_args()
    path = Path(args.file).resolve()
    sys.path.insert(0, str(path.parent))
    spec = importlib.util.spec_from_file_location("nano_user_app", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    module.app.run(args.host, args.port, renderer=args.renderer)


if __name__ == "__main__": main()
