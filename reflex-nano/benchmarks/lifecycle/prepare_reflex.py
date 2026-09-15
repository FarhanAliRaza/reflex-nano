"""Untimed frontend/dependency setup; run before measuring application builds."""
import shutil
from pathlib import Path
from reflex_base import constants
from reflex.utils import frontend_skeleton as fs

if not Path('.web/utils/state.js').exists():
    shutil.copytree(constants.Templates.Dirs.WEB_TEMPLATE, '.web', dirs_exist_ok=True)
    fs.initialize_vite_config()
    fs.update_react_router_config()
    fs.init_reflex_json(project_hash=314159265)
# npm rewrites whitespace; Reflex compares rendered package.json byte-for-byte
# and would otherwise invalidate its dependency cache on every app compile.
# This invokes the stock initializer; it changes no dependency versions.
fs.initialize_package_json()
