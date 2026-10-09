#!/usr/bin/env python3
"""Retained-state demonstration, restricted to this checkout's fixture."""
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / 'server/scripts/db-fixture.sh'


def fixture(action):
    return subprocess.check_output(['sh', str(FIXTURE), action], cwd=ROOT, text=True).strip()


def wait_ready(port):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/readyz', timeout=2) as response:
                if response.status == 200 and json.load(response) == {'status': 'ready'}:
                    return
        except (OSError, urllib.error.URLError):
            pass
        time.sleep(0.1)
    raise RuntimeError('demo readiness failed within 30 seconds; inspect .workspace/ logs')


def interrupted(_signum, _frame):
    raise KeyboardInterrupt


def main():
    (ROOT / '.workspace').mkdir(exist_ok=True)
    fixture('up')
    subprocess.run(['cargo', 'build', '-p', 'sidereal-server', '--manifest-path', 'server/Cargo.toml'], cwd=ROOT, check=True)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--manifest-path', 'server/Cargo.toml', '--no-deps', '--format-version', '1'], cwd=ROOT))
    binary = Path(metadata['target_directory']) / 'debug/sidereal-server'
    with socket.socket() as selected:
        selected.bind(('127.0.0.1', 0))
        port = selected.getsockname()[1]
    environment = dict(os.environ, DATABASE_URL=fixture('demo-url'), PORT=str(port))
    server = None
    signal.signal(signal.SIGINT, interrupted)
    signal.signal(signal.SIGTERM, interrupted)
    with tempfile.TemporaryDirectory(prefix='db-demo-', dir=ROOT / '.workspace') as scratch:
        log = open(Path(scratch) / 'server.log', 'w')

        def start():
            return subprocess.Popen([str(binary)], env=environment, cwd=ROOT, stdout=log, stderr=log)

        def stop():
            if server is not None and server.poll() is None:
                server.terminate()
                try:
                    server.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    server.kill()
                    server.wait(timeout=5)

        try:
            server = start()
            wait_ready(port)
            before = fixture('demo-records')
            Path(scratch, 'initial.json').write_text(before)
            stop()
            server = start()
            wait_ready(port)
            assert fixture('demo-records') == before, 'migration records changed after server restart'
            # Prove database outage changes readiness while liveness stays healthy.
            fixture('stop')
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/healthz', timeout=2) as response:
                assert response.status == 200 and json.load(response) == {'status': 'ok'}
            started = time.monotonic()
            try:
                urllib.request.urlopen(f'http://127.0.0.1:{port}/readyz', timeout=2.25)
                raise AssertionError('readiness incorrectly succeeded during database outage')
            except urllib.error.HTTPError as error:
                assert error.code == 503 and json.load(error) == {'status': 'not_ready'}
            assert time.monotonic() - started < 2.25, 'outage readiness exceeded deadline'
            fixture('up')
            recovered = time.monotonic()
            wait_ready(port)
            assert time.monotonic() - recovered < 5, 'readiness recovery exceeded five seconds'
            assert fixture('demo-records') == before, 'migration records changed after PostgreSQL restart'
            # Also prove down/up retains the named volume and the complete history.
            stop()
            fixture('down')
            fixture('up')
            server = start()
            wait_ready(port)
            assert fixture('demo-records') == before, 'fixture stop/start changed migration records'
            print('Database demo passed: identical migration records after server and PostgreSQL restarts.')
        finally:
            stop()
            log.close()


if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        raise SystemExit(130)
