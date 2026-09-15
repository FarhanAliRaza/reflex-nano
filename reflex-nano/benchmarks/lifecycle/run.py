"""Production compilation, process startup, initial browser load and first reload.

Run from this directory using the benchmark venv. Dependency installation,
browser installation, and rebuilding the Rust framework are separate setup costs.
"""
import argparse
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import random
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def finalize(result, output):
    counts = result['arguments']
    assert len(result['browser_runs']) == len(counts['rows'])*len(counts['frameworks'])*counts['server_runs']
    assert all(len(run['navigations']) == counts['contexts']*2 and not run['errors'] for run in result['browser_runs'])
    sources = [*HERE.glob('*.py'), *HERE.glob('*.js'), *HERE.glob('*.cjs'),
               HERE/'reflex.lock/package.json', HERE/'reflex.lock/package-lock.json',
               ROOT/'dist/nano', ROOT/'benchmarks/baselines/0.2.0/nano', ROOT/'benchmarks/shared.py',
               ROOT/'Cargo.lock', ROOT/'Cargo.toml', ROOT/'rust-toolchain.toml',
               *ROOT.joinpath('crates').rglob('*.rs'), ROOT/'crates/nano-core/src/client.js',
               *ROOT.joinpath('frontend').glob('*.*'),*ROOT.joinpath('crates/nano-core/frontend-dist').glob('*.*')]
    result['sha256'] = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources if p.is_file()}
    result['environment']['frontend_packages'] = {
        name: json.loads((HERE/'.web/node_modules'/name/'package.json').read_text())['version']
        for name in ['react', 'react-dom', 'react-router', '@react-router/dev', 'vite', 'socket.io-client']}
    result['completed'] = True
    output.write_text(json.dumps(result, indent=2))

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--rows', nargs='+', type=int, default=[100, 1000, 10000])
    parser.add_argument('--build-runs', type=int, default=5)
    parser.add_argument('--server-runs', type=int, default=5)
    parser.add_argument('--contexts', type=int, default=2)
    parser.add_argument('--frameworks', nargs='+', choices=['nano','nano_python','nano_react','nano_react_python','reflex','nano_before'],
                        default=['nano','nano_python','reflex','nano_before'])
    parser.add_argument('--output', default='results.json')
    parser.add_argument('--finalize-only', action='store_true', help='Complete provenance for fully recorded trials without rerunning them')
    args = parser.parse_args()
    os.chdir(HERE)
    output = Path(args.output)
    if args.finalize_only:
        result = json.loads(output.read_text())
        result['finalization_note'] = 'Provenance finalization retried after excluding a directory from file hashes; recorded timings were not rerun or modified.'
        finalize(result, output)
        return
    logs = HERE / 'logs'; logs.mkdir(exist_ok=True)
    env = {**os.environ, 'REFLEX_ENV_MODE': 'prod', 'NANO_PYTHON': sys.executable,
           'PYTHONUNBUFFERED': '1'}
    rng = random.Random(20260915)
    result = {'schema_version': 1, 'date_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
        'arguments': vars(args), 'environment': {
            'python': sys.version, 'platform': platform.platform(),
            'cpu': next((line.split(':',1)[1].strip() for line in Path('/proc/cpuinfo').read_text().splitlines() if line.startswith('model name')), None),
            'affinity': sorted(os.sched_getaffinity(0)), 'cpu_quota': Path('/sys/fs/cgroup/cpu.max').read_text().strip(),
            'node': subprocess.check_output(['node', '--version'], text=True).strip(),
            'packages': {name: importlib.metadata.version(name) for name in ['reflex', 'reflex-base', 'reflex-nano', 'granian']},
            'rustc': subprocess.check_output(['rustc', '--version'], cwd=ROOT, text=True).strip(),
            'rust_source_build': {'status': 'separate_measurement', 'path': 'docs/build-timings.json'},
            'nano_before': 'Preserved 0.2.0 release executable; same fixture and current browser probe.',
        }, 'compilations': [], 'browser_runs': []}
    def save():
        output.write_text(json.dumps(result, indent=2))
    def command(command, label, extra=None, timeout=180):
        start = time.perf_counter_ns()
        p = subprocess.run(command, cwd=HERE, env={**env, **(extra or {})},
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=timeout)
        elapsed = (time.perf_counter_ns()-start)/1e6
        (logs / f'{label}.log').write_text(p.stdout)
        if p.returncode:
            raise RuntimeError(f'{label}: {p.stdout[-12000:]}')
        return elapsed, p.stdout
    def python(script, label, extra=None):
        return command([sys.executable, script], label, extra)
    for rows in args.rows:
        env['NANO_BENCH_ROWS'] = str(rows)
        # Populate/verify dependency caches in an unmeasured setup pass.
        python('prepare_reflex.py', f'setup-{rows}')
        python('compile_reflex.py', f'untimed-codegen-{rows}')
        build_kinds = [kind for kind in ['nano', 'nano_react', 'reflex'] if kind in args.frameworks or (kind+'_python') in args.frameworks]
        builds = [(kind, run) for kind in build_kinds for run in range(args.build_runs)]
        rng.shuffle(builds)
        for kind, run in builds:
            if kind == 'reflex':
                python('prepare_reflex.py', f'prepare-{rows}-{run}')
            wall, log = python(f"compile_{'reflex' if kind=='reflex' else 'nano'}.py", f'compile-{kind}-{rows}-{run}', {'NANO_RENDERER':'react' if kind=='nano_react' else 'html'})
            metrics = json.loads(next(line[15:] for line in log.splitlines() if line.startswith('COMPILE_RESULT ')))
            build_ms = 0
            if kind == 'reflex':
                build_ms, _ = python('build_reflex.py', f'build-{rows}-{run}')
            record = {'kind': kind, 'rows': rows, 'run': run,
                      'codegen_process_ms': wall, 'production_build_process_ms': build_ms,
                      'total_app_prepare_ms': wall+build_ms, **metrics}
            result['compilations'].append(record); save()
            print(json.dumps({'stage': 'compile', **record}), flush=True)
        servers = [(kind, run) for kind in args.frameworks for run in range(args.server_runs)]
        rng.shuffle(servers)
        for kind, run in servers:
            destination = logs / f'browser-{kind}-{rows}-{run}.json'
            _, log = command(['node', str(HERE/'browser.cjs')], f'browser-{kind}-{rows}-{run}', {
                'BENCH_KIND': kind, 'BENCH_REPETITIONS': str(args.contexts), 'BENCH_OUTPUT': str(destination)}, timeout=300)
            record = json.loads(destination.read_text()); record['run'] = run
            result['browser_runs'].append(record); save()
            print(log.strip(), flush=True)
    finalize(result, output)


if __name__ == '__main__':
    main()
