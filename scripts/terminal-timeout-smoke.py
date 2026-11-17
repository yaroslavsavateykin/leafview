#!/usr/bin/env python3
"""A non-answering graphics query must never steal subsequent navigation keys."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=Path('target/debug/leafview'))
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
for reply in (False, True):
    with tempfile.TemporaryDirectory(prefix='leafview-query-') as tmp:
        tmp = Path(tmp)
        trace = tmp / 'trace.jsonl'
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
        env = {**os.environ, 'LEAFVIEW_GRAPHICS': 'kitty',
               'LEAFVIEW_QUERY_TIMEOUT_MS': '100', 'LEAFVIEW_TRACE': str(trace),
               'LEAFVIEW_KEYBINDINGS': str(tmp / 'keys.toml'), 'XDG_STATE_HOME': str(tmp)}
        proc = subprocess.Popen([str(args.binary.resolve()), str(root / 'tests/fixtures/sample.pdf')],
                                stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True)
        def pump():
            if select.select([master], [], [], .01)[0]:
                chunk = os.read(master, 1024 * 1024)
                if reply and b'a=q' in chunk:
                    os.write(master, b'\x1b_Gi=31;OK\x1b\\\x1b[6;20;10t\x1b[0n')
                if b'\x1b[6n' in chunk:
                    os.write(master, b'\x1b[1;1R')
            assert proc.poll() is None, 'viewer exited'
        def wait_page(page):
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                pump()
                try:
                    if json.loads(trace.read_text().splitlines()[-1])['page'] == page:
                        return
                except (FileNotFoundError, ValueError, IndexError):
                    pass
            raise AssertionError(f'navigation stuck on page {page}; query replied={reply}')
        try:
            wait_page(0)
            for key, page in ((b'n', 1), (b'p', 0), (b'G', 2), (b'gg', 0)):
                os.write(master, key)
                wait_page(page)
            os.write(master, b'\x03')
            deadline = time.monotonic() + 5
            while proc.poll() is None and time.monotonic() < deadline:
                if select.select([master], [], [], .01)[0]:
                    os.read(master, 1024 * 1024)
            assert proc.poll() == 0
            print(f'PASS graphics query replied={reply}: next / previous / last / first / quit')
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
            os.close(master)
            os.close(slave)
