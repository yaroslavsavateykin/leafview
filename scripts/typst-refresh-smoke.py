#!/usr/bin/env python3
"""Verify refreshed slide pixels, page retention and Kitty slot lifetime via PTY."""
import argparse
import base64
import fcntl
import io
import json
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import tempfile
import termios
import time
from PIL import Image

parser = argparse.ArgumentParser()
parser.add_argument('--binary', default='target/debug/leafview')
args = parser.parse_args()
binary = Path(args.binary).resolve()
with tempfile.TemporaryDirectory(prefix='leafview-refresh-') as tmp:
    tmp = Path(tmp)
    source, trace = tmp / 'main.typ', tmp / 'trace'

    def save(n):
        color = 'ff0000' if n % 2 else '0000ff'
        staged = tmp / 'staged'
        staged.write_text(f'''#set page(width: 300pt, height: 400pt, margin: 0pt)
#set text(size: 20pt)
First {n}
#pagebreak()
Second {n}
#pagebreak()
#rect(width: 100%, height: 100%, fill: rgb("#{color}"))
''')
        staged.replace(source)

    save(0)
    # This regression intentionally exercises refresh without source following.
    (tmp / '.leafview.json').write_text(json.dumps({'auto_sync': False}))
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 100, 0, 0))
    env = {**os.environ, 'LEAFVIEW_GRAPHICS': 'kitty',
           'LEAFVIEW_QUERY_TIMEOUT_MS': '100', 'LEAFVIEW_TRACE': str(trace),
           'XDG_STATE_HOME': str(tmp), 'LEAFVIEW_KEYBINDINGS': str(tmp / 'keys.toml')}
    for key in ('TMUX', 'WEZTERM_EXECUTABLE', 'KONSOLE_VERSION'):
        env.pop(key, None)
    proc = subprocess.Popen([str(binary), str(source)], stdin=slave, stdout=slave,
                            stderr=slave, start_new_session=True, env=env)
    wire = bytearray()

    def pump(seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([master], [], [], .01)[0]:
                chunk = os.read(master, 1048576)
                wire.extend(chunk)
                if b'\x1b[6n' in chunk:
                    os.write(master, b'\x1b[1;1R')
            assert proc.poll() is None, wire[-1000:]

    def frames():
        result = []
        for line in trace.read_text().splitlines() if trace.exists() else []:
            try:
                result.append(json.loads(line))
            except ValueError:  # Last record can still be in flight.
                pass
        return result

    def wait(predicate):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            pump(.05)
            records = frames()
            if records and predicate(records[-1]):
                pump(.15)  # Trace commit precedes terminal publication.
                return records[-1]
        raise AssertionError(('missing frame', frames(), wire[-500:]))

    def images():
        payload = bytearray()
        result = []
        for header, chunk in re.findall(rb'\x1b_G([^;]*);(.*?)\x1b\\', bytes(wire), re.S):
            if b'f=100' in header:
                payload.clear()
            if b'm=' not in header:
                continue
            payload.extend(base64.b64decode(chunk))
            if b'm=0' in header:
                im = Image.open(io.BytesIO(payload)).convert('RGB')
                result.append(im)
                payload.clear()
        return result

    try:
        frame = wait(lambda f: f['revision'] >= 2)
        # Focus preview: startup intentionally leaves focus in the file tree.
        os.write(master, b'\x1b[<0;80;10M\x1b[<0;80;10m')
        pump(.1)
        os.write(master, b'G')
        frame = wait(lambda f: f['page'] == 2)
        # Recompilation happens while the user's focus is in their editor.
        os.write(master, b'\x1b[O')
        pump(.1)
        for n in range(1, 45):
            revision = frame['revision']
            wire.clear()
            save(n)
            frame = wait(lambda f: f['revision'] > revision)
            assert frame['page'] == 2, (n, frame)
            assert b'\x1b[2J' not in wire, ('cleared visible slide', n)
            decoded = images()
            assert decoded, ('no image transfer', n)
            pixel = decoded[-1].getpixel((decoded[-1].width // 2, decoded[-1].height // 2))
            expected = (255, 0, 0) if n % 2 else (0, 0, 255)
            assert pixel == expected, ('wrong refreshed slide pixels', n, pixel, expected)
            assert '\U0010eeee'.encode() in wire, ('missing placement', n)
        os.write(master, b'\x1b[I')
        pump(.6)
        for key, page in [(b'p', 1), (b'n', 2), (b'p', 1), (b'n', 2)]:
            wire.clear()
            os.write(master, key)
            wait(lambda f: f['page'] == page)
            assert '\U0010eeee'.encode() in wire, ('missing returned page', page)
        print('PASS: 44 background recompiles / page 3 retained / actual red-blue pixels / slot reuse / cached return / no clears')
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
