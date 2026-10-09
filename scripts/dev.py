#!/usr/bin/env python3
"""Run both development processes and stop their complete process groups."""
import os
import signal
import subprocess
import time

interrupted = False


def interrupt(_signum, _frame):
    global interrupted
    interrupted = True


def stop(process):
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass


def main():
    signal.signal(signal.SIGINT, interrupt)
    signal.signal(signal.SIGTERM, interrupt)
    processes = []
    try:
        for recipe in ('server', 'web'):
            processes.append(subprocess.Popen(['just', recipe], start_new_session=True))
        while not interrupted and all(process.poll() is None for process in processes):
            time.sleep(0.1)
    finally:
        for process in processes:
            stop(process)
        for process in processes:
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                process.wait()
    return 0 if interrupted else int(any(process.returncode for process in processes))


if __name__ == '__main__':
    raise SystemExit(main())
