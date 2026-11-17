#!/usr/bin/env python3
"""PTY regression: automatic source following, opt-out, and changing page counts."""
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

parser = argparse.ArgumentParser()
parser.add_argument('--binary', default='target/debug/leafview')
binary = Path(parser.parse_args().binary).resolve()
for follow in (True, False):
    with tempfile.TemporaryDirectory(prefix='leafview-follow-') as directory:
        tmp = Path(directory)
        source, trace = tmp / 'main.typ', tmp / 'trace'
        (tmp / '.leafview.json').write_text(json.dumps({'auto_sync': follow}))

        def save(text):
            staged = tmp / 'staged'
            staged.write_text(text)
            staged.replace(source)

        prefix = '#set page(width: 300pt, height: 400pt)\nFirst page\n#pagebreak()\n'
        save(prefix + 'Second page original\n')
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
        env = {**os.environ, 'LEAFVIEW_GRAPHICS': 'kitty',
               'LEAFVIEW_QUERY_TIMEOUT_MS': '100', 'LEAFVIEW_TRACE': str(trace),
               'XDG_CONFIG_HOME': str(tmp), 'XDG_STATE_HOME': str(tmp),
               'LEAFVIEW_KEYBINDINGS': str(tmp / 'keys.toml')}
        for key in ('TMUX', 'TMUX_PANE', 'WEZTERM_EXECUTABLE', 'KONSOLE_VERSION'):
            env.pop(key, None)
        proc = subprocess.Popen([str(binary), str(source)], stdin=slave,
                                stdout=slave, stderr=slave, env=env, start_new_session=True)
        wire = bytearray()

        def pump():
            if select.select([master], [], [], .02)[0]:
                chunk = os.read(master, 1048576)
                wire.extend(chunk)
                if b'\x1b[6n' in chunk:
                    os.write(master, b'\x1b[1;1R')
            assert proc.poll() is None, wire[-1000:]

        def wait_frame(revision=0, page=None):
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                pump()
                try:
                    frame = json.loads(trace.read_text().splitlines()[-1])
                except (FileNotFoundError, ValueError, IndexError):
                    continue
                if frame['revision'] > revision and (page is None or frame['page'] == page):
                    return frame
            raise AssertionError(('missing frame', follow, revision, page, wire[-1000:]))

        try:
            frame = wait_frame()
            # Explicitly start on page 1, regardless of initial source mapping.
            os.write(master, b'\x1b[<0;80;10M\x1b[<0;80;10m')
            time.sleep(.1)
            os.write(master, b'g')
            wait_frame(page=0)
            save(prefix + 'Second page modified text\n')
            frame = wait_frame(frame['revision'], 1 if follow else 0)
            # Grow to three pages; the edited token maps to the new last page.
            save(prefix + 'Second page modified text\n#pagebreak()\nThird page new text\n')
            frame = wait_frame(frame['revision'], 2 if follow else 0)
            deadline = time.monotonic() + .5
            while time.monotonic() < deadline:
                pump()
            assert b'/3' in wire, 'page count did not grow'
            wire.clear()
            save('#set page(width: 300pt, height: 400pt)\nOnly surviving page\n')
            wait_frame(frame['revision'], 0)
            deadline = time.monotonic() + .5
            while time.monotonic() < deadline:
                pump()
            assert b'1/1' in wire, 'page count did not shrink'
            print(f'PASS: auto_sync={follow} / edited page / grow to 3 / shrink to 1')
        finally:
            proc.kill()
            proc.wait()
            os.close(master)
            os.close(slave)
