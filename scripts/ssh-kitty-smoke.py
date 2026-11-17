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
import signal
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
               'LEAFVIEW_GRAPHICS': 'auto', 'LEAFVIEW_DEBUG': '1', 'LEAFVIEW_LOG': str(Path(tmp) / 'debug.log'),
               'XDG_STATE_HOME': tmp}
        # TERM identifies the virtual tmux protocol without invoking a real
        # tmux server or changing a user's passthrough configuration.
        for name in ('TMUX', 'WEZTERM_EXECUTABLE', 'KONSOLE_VERSION', 'LEAFVIEW_QUERY_TIMEOUT_MS'):
            env.pop(name, None)
        # Model metadata independently of focus/resize events. Never query or
        # modify the user's real tmux server from this protocol emulator.
        env.pop('TMUX_PANE', None)
        state = Path(tmp) / 'tmux-state'
        if wrapped:
            fake_tmux = Path(tmp) / 'tmux'
            fake_tmux.write_text('#!/bin/sh\nexec /bin/cat "$LEAFVIEW_TEST_TMUX_STATE"\n')
            fake_tmux.chmod(0o700)
            state.write_text('1|0|0|0|12|100|/dev/pts/2|1|layout\n')
            env.update(PATH=tmp + os.pathsep + env['PATH'], TMUX_PANE='%999',
                       LEAFVIEW_TEST_TMUX_STATE=str(state))
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
            assert 'protocol=Kitty' in (Path(tmp) / 'debug.log').read_text(), output[-3000:]
            assert b'leafview: protocol=' not in output, 'debug diagnostics leaked into UI'
            assert b'f=100,t=d' in output, 'missing inline SSH-safe PNG'
            assert b't=f' not in output and b't=s' not in output, 'remote file transport used'
            if wrapped:
                assert b'\x1bPtmux;' in output, 'missing tmux wrapping'
            output.clear()
            os.write(master, b'j')  # next page is prepared before publication
            pump(1)
            begin = output.find(b'\x1b[?2026h')
            upload = output.find(b'f=100,t=d')
            end = output.find(b'\x1b[?2026l', upload)
            assert b'\x1b[2J' not in output, 'page replacement cleared the visible PDF'
            assert 0 <= begin < upload < end, ('page replacement is not synchronized', output[-2000:])
            assert output.count(b'f=100,t=d') == 1, 'page was uploaded twice (visible flicker)'
            output.clear()
            pump(.7)
            assert b'f=100,t=d' not in output, 'page was retransmitted after settling'
            if wrapped:
                # Change client while the window remains active, without focus.
                state.write_text('1|0|0|0|14|200|/dev/pts/3|1|layout\n')
                pump(1)
                assert b'f=100,t=d' in output, 'client change without focus lost the image'
                output.clear()
                # Another pane is zoomed; then restore this still-unfocused pane.
                state.write_text('1|1|0|0|14|200|/dev/pts/3|1|layout\n')
                pump(.6)
                assert b'f=100,t=d' not in output, 'hidden pane uploaded graphics'
                state.write_text('1|0|0|0|14|200|/dev/pts/3|1|layout\n')
                pump(1)
                assert b'f=100,t=d' in output, 'unzoom without focus lost the image'
            output.clear()
            os.write(master, b'\x1b[O')  # hidden tmux window
            pump(.3)
            assert b'f=100,t=d' not in output, 'uploaded graphics on focus loss'
            os.write(master, b'\x1b[I')  # return / attach from a new SSH client
            pump(2)
            expected = 0 if wrapped else 2
            assert output.count(b'f=100,t=d') == expected, ('ordinary tmux pane focus must not reupload; direct focus recovers', output[-2000:])
            assert b'\x1b[2J' not in output, 'focus recovery erased the visible image'
            output.clear()
            pump(1)
            assert b'f=100,t=d' not in output, 'focus recovery kept uploading while idle'
            os.write(master, b'\x1b[I\x1b[O')  # leave before delayed replay
            pump(.1)
            output.clear()
            pump(.5)
            assert b'f=100,t=d' not in output, 'delayed replay was not cancelled on focus loss'
            os.write(master, b'\x1b[I')
            pump(.5)
            # A visible PDF can remain unfocused beside an editor. Cache
            # invalidation on same-size resize must still upload before draw.
            os.write(master, b'\x1b[O')
            pump(.3)
            output.clear()
            proc.send_signal(signal.SIGWINCH)
            pump(1)
            assert b'f=100,t=d' in output, 'unfocused resize lost the image'
            os.write(master, b'\x1b[I')
            pump(.5)
            # A tmux reattach can resize without sending FocusGained. Even a
            # same-size SIGWINCH must invalidate the terminal-resident cache.
            output.clear()
            proc.send_signal(signal.SIGWINCH)
            pump(2)
            assert b'f=100,t=d' in output, ('resize failed to re-upload terminal cache', output[-2000:])
            # An actual geometry change makes Ratatui clear its viewport. That
            # clear must precede the first PNG, never invalidate a fresh upload.
            output.clear()
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 32, 104, 0, 0))
            proc.send_signal(signal.SIGWINCH)
            pump(2)
            upload = output.find(b'f=100,t=d')
            clear = output.find(b'\x1b[2J')
            assert 0 <= clear < upload, ('resize cleared after graphics upload', output[-2000:])
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
