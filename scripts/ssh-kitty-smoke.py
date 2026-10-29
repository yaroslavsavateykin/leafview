#!/usr/bin/env python3
"""PTY Kitty client emulator: delayed SSH negotiation, inline PNG, reconnect.
No SSH daemon or real GPU terminal involved; does not certify visual output.
"""
import argparse
import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import tempfile
import termios
import time

parser = argparse.ArgumentParser()
parser.add_argument('--binary', default='target/release/leafview')
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
for wrapped in (False, True):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
    before = termios.tcgetattr(slave)
    with tempfile.TemporaryDirectory(prefix='leafview-ssh-test-') as tmp:
        env = {**os.environ, 'SSH_CONNECTION': '192.0.2.1 50000 192.0.2.2 22',
               'TERM': 'tmux-256color' if wrapped else 'xterm-kitty',
               'LEAFVIEW_GRAPHICS': 'auto', 'LEAFVIEW_DEBUG': '1',
               'XDG_STATE_HOME': tmp}
        # TERM identifies the virtual tmux protocol without invoking a real
        # tmux server or changing a user's passthrough configuration.
        for name in ('TMUX', 'WEZTERM_EXECUTABLE', 'KONSOLE_VERSION', 'LEAFVIEW_QUERY_TIMEOUT_MS'):
            env.pop(name, None)
        proc = subprocess.Popen([str(root / args.binary), str(root / 'tests/fixtures/sample.pdf')],
                                stdin=slave, stdout=slave, stderr=slave, env=env,
                                start_new_session=True)
        output = bytearray()
        def pump(seconds):
            end = time.monotonic() + seconds
            while time.monotonic() < end:
                if select.select([master], [], [], .01)[0]:
                    chunk = os.read(master, 1024 * 1024)
                    output.extend(chunk)
                    if b'\x1b[6n' in chunk:
                        os.write(master, b'\x1b[1;1R')
                assert proc.poll() is None, output[-2000:]
        try:
            end = time.monotonic() + 5
            while b'a=q' not in output and time.monotonic() < end:
                pump(.02)
            assert b'a=q' in output, output[-2000:]
            # Simulate slow round-trip; must not silently downgrade to halfblocks.
            pump(1.2)
            os.write(master, b'\x1b_Gi=31;OK\x1b\\\x1b[6;20;10t\x1b[0n')
            pump(1.5)
            assert b'protocol=Kitty' in output, output[-3000:]
            assert b'f=100,t=d' in output, 'missing inline SSH-safe PNG'
            assert b't=f' not in output and b't=s' not in output, 'remote file transport used'
            if wrapped:
                assert b'\x1bPtmux;' in output, 'missing tmux wrapping'
            output.clear()
            os.write(master, b'\x1b[I')  # focus: attaching from a new SSH client
            pump(2)
            assert b'f=100,t=d' in output, ('focus failed to re-upload terminal cache', output[-2000:])
            os.write(master, b'\x03')
            proc.wait(timeout=3)
            assert proc.returncode == 0
            assert termios.tcgetattr(slave) == before
            print(f'PASS: delayed SSH negotiation / inline PNG / reconnect / cleanup; tmux={wrapped}')
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
            os.close(master)
            os.close(slave)
