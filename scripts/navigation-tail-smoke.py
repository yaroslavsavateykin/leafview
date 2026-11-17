#!/usr/bin/env python3
"""Long-PDF PTY regression: burst repeats must not produce a document-long tail.
Uses fallback graphics, generated PDF, and only the Python standard library.
"""
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
parser.add_argument('--binary', default='target/debug/leafview')
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]


def make_pdf(path, pages=200):
    objects = [b'<< /Type /Catalog /Pages 2 0 R >>', b'']
    children = []
    for page in range(pages):
        number = len(objects) + 1
        children.append(f'{number} 0 R')
        stream = f'BT /F1 24 Tf 30 100 Td (Page {page + 1}) Tj ET'.encode()
        objects.extend([
            f'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents {number + 1} 0 R >>'.encode(),
            f'<< /Length {len(stream)} >>\nstream\n'.encode() + stream + b'\nendstream',
        ])
    objects[1] = f'<< /Type /Pages /Count {pages} /Kids [{" ".join(children)}] >>'.encode()
    data = bytearray(b'%PDF-1.4\n')
    offsets = [0]
    for number, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f'{number} 0 obj\n'.encode() + obj + b'\nendobj\n')
    xref = len(data)
    data.extend(f'xref\n0 {len(offsets)}\n0000000000 65535 f \n'.encode())
    for offset in offsets[1:]:
        data.extend(f'{offset:010} 00000 n \n'.encode())
    data.extend(f'trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode())
    path.write_bytes(data)


with tempfile.TemporaryDirectory(prefix='leafview-navigation-') as tmp:
    tmp = Path(tmp)
    pdf, trace = tmp / 'long.pdf', tmp / 'trace.jsonl'
    make_pdf(pdf)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 80, 0, 0))
    before = termios.tcgetattr(slave)
    env = {**os.environ, 'TERM': 'xterm-256color', 'LEAFVIEW_GRAPHICS': 'safe',
           'LEAFVIEW_TRACE': str(trace), 'XDG_STATE_HOME': str(tmp)}
    for key in ('TMUX', 'TMUX_PANE', 'TERM_PROGRAM'):
        env.pop(key, None)
    proc = subprocess.Popen([str(root / args.binary), str(pdf)], stdin=slave,
                            stdout=slave, stderr=slave, env=env, start_new_session=True)
    output = bytearray()

    def pump(seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([master], [], [], .01)[0]:
                output.extend(os.read(master, 1024 * 1024))
            assert proc.poll() is None, output[-2000:]

    def frames():
        return [json.loads(line) for line in trace.read_text().splitlines()]

    try:
        pump(1)
        assert frames()[-1]['page'] == 0
        os.write(master, b'\x1b[6~' * 200)
        pump(2)
        shown = frames()
        assert 0 < shown[-1]['page'] < 15, ('repeat backlog', shown[-1])
        pump(1)
        assert frames() == shown, 'navigation continued after input stopped'
        os.write(master, b'\x1b[5~')
        pump(.5)
        assert frames()[-1]['page'] == shown[-1]['page'] - 1, 'reverse input stuck'
        os.write(master, b'\x03')
        proc.wait(timeout=3)
        assert proc.returncode == 0
        assert termios.tcgetattr(slave) == before
        print(f'PASS: 200 repeats bounded at page {shown[-1]["page"] + 1}; idle stable; reverse responsive; TTY restored')
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
