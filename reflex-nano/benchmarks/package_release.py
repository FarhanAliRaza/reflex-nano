"""Package source, release artifacts, tests and recorded results with checksums."""
import hashlib
from pathlib import Path
import zipfile
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[1]
subprocess.run([sys.executable, str(ROOT/'benchmarks/verify_frontend.py')],check=True)
excluded={'target','node_modules','__pycache__','.pytest_cache','.venv','.web','.git','logs'}
def include(path):
    relative=path.relative_to(ROOT)
    if any(part in excluded for part in relative.parts): return False
    if path.suffix in {'.pyc','.so'} or 'pilot' in path.name: return False
    if relative.as_posix() in {'SHA256SUMS','benchmarks/before-optimization.json'}: return False
    if relative.parts[0]=='dist' and path.name not in {
        'nano','reflex_nano-0.4.0-cp312-cp312-manylinux_2_34_x86_64.whl'}: return False
    return path.is_file()

files=sorted(p for p in ROOT.rglob('*') if include(p))
checksums=''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(ROOT).as_posix()}\n' for p in files)
(ROOT/'SHA256SUMS').write_text(checksums)
files.append(ROOT/'SHA256SUMS')
archive=ROOT.parent/'reflex-nano-0.4.0.zip'
with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as z:
    for path in files:z.write(path,Path('reflex-nano')/path.relative_to(ROOT))
with zipfile.ZipFile(archive) as z:
    assert z.testzip() is None
    for line in checksums.splitlines():
        digest,name=line.split('  ',1)
        assert hashlib.sha256(z.read('reflex-nano/'+name)).hexdigest()==digest
print(f'{archive}\n{len(files)} files\n{archive.stat().st_size} bytes\nSHA-256 {hashlib.sha256(archive.read_bytes()).hexdigest()}')
