#!/usr/bin/env python3
"""Linux PTY regression: owned compiler stops on graceful and abrupt exit."""
import argparse
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, default=Path('target/debug/leafview'))
args = parser.parse_args()


def alive(pid):
    try:
        # An orphan zombie is already stopped; reaping belongs to the system init.
        return Path(f'/proc/{pid}/stat').read_text().split(') ', 1)[1][0] != 'Z'
    except FileNotFoundError:
        return False


for mode in ('Ctrl-C', 'SIGTERM', 'SIGHUP', 'SIGKILL'):
    with tempfile.TemporaryDirectory(prefix='leafview-shutdown-') as tmp:
        source = Path(tmp) / 'main.typ'
        source.write_text('Hello LeafView')
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 80, 0, 0))
        proc = subprocess.Popen(
            [str(args.binary.resolve()), str(source)],
            stdin=slave, stdout=slave, stderr=slave, start_new_session=True,
            env={**os.environ, 'LEAFVIEW_GRAPHICS': 'safe'},
        )
        children = set()
        output = bytearray()

        def pump():
            if select.select([master], [], [], .01)[0]:
                output.extend(os.read(master, 1024 * 1024))

        try:
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                pump()
                assert proc.poll() is None, output[-2000:]
                for task in Path(f'/proc/{proc.pid}/task').iterdir():
                    for pid in (task / 'children').read_text().split():
                        cmd = Path(f'/proc/{pid}/cmdline')
                        if cmd.exists() and b'--typst-compiler' in cmd.read_bytes():
                            children.add(int(pid))
            assert children, 'no owned compiler observed'
            assert b'Compiling' in output and b'Typst watch' not in output
            if mode == 'Ctrl-C':
                os.write(master, b'\x03')
            else:
                proc.send_signal(getattr(signal, mode))
            deadline = time.monotonic() + 5
            while proc.poll() is None and time.monotonic() < deadline:
                pump()
            assert proc.poll() == (-signal.SIGKILL if mode == 'SIGKILL' else 0)
            while any(alive(pid) for pid in children) and time.monotonic() < deadline:
                time.sleep(.01)
            assert not any(alive(pid) for pid in children), children
            if mode != 'SIGKILL':
                assert all(not Path(f'/proc/{pid}').exists() for pid in children)
            print(f'PASS {mode}: owned compiler stopped')
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
            os.close(master)
            os.close(slave)
