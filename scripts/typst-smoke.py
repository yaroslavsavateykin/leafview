#!/usr/bin/env python3
"""Typst live integration through PTY: atomic include saves, errors, latest wins.
Not a real terminal visual test. Requires installed typst for CLI prerequisite.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import shutil
import struct
import subprocess
import tempfile
import termios
import time

parser = argparse.ArgumentParser()
parser.add_argument('--no-pdf', action='store_true')
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='leafview-live-') as tmp:
    project = Path(tmp) / 'project with spaces'
    shutil.copytree(root / 'tests/fixtures/typst-live', project)
    target = project / 'chapters/theory.typ'
    source = target.read_text()
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 90, 0, 0))
    before = termios.tcgetattr(slave)
    trace = Path(tmp) / 'trace.jsonl'
    pdf = project / 'main.pdf'
    sentinel = b'EXISTING PDF MUST REMAIN UNTOUCHED'
    if args.no_pdf:
        pdf.write_bytes(sentinel)
    command = [str(root / 'target/release/leafview'), str(project / 'main.typ')]
    if not args.no_pdf:
        command.append('--write-pdf')
    proc = subprocess.Popen(command,
                            stdin=slave, stdout=slave, stderr=slave,
                            env={**os.environ, 'LEAFVIEW_GRAPHICS': 'safe', 'LEAFVIEW_TRACE': str(trace)}, start_new_session=True)
    output = bytearray()
    def pump(seconds):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            if select.select([master], [], [], .01)[0]:
                output.extend(os.read(master, 1024 * 1024))
            assert proc.poll() is None, output[-2000:]
    def save(text):
        replacement = target.with_suffix('.tmp')
        replacement.write_text(text)
        replacement.replace(target)
    try:
        pump(2)
        assert trace.exists(), 'initial PDF must render without a bottom status bar'
        assert b'PDF' in output, 'missing PDF frame title'
        assert b'Typst ready' not in output
        def current():
            return json.loads(trace.read_text().splitlines()[-1])
        assert current()['page'] == 0
        assert current()['revision'] > 0
        if args.no_pdf:
            assert pdf.read_bytes() == sentinel
        else:
            assert pdf.read_bytes().startswith(b'%PDF')
        # Real SGR wheel reports: zoom and horizontal page turns.
        os.write(master, b'\x1b[<64;46;15M' * 10)
        pump(.8)
        assert abs(current()['zoom'] - 1.5) < .001, current()
        os.write(master, b'\x1b[<67;46;15M')
        pump(.5)
        assert current()['page'] == 1, current()
        os.write(master, b'\x1b[<66;46;15M')
        pump(.5)
        assert current()['page'] == 0, current()
        output.clear()
        start = time.monotonic()
        save(source.replace('c^2', 'c^3'))
        while time.monotonic() - start < 3:
            pump(.02)
            if current()['page'] == 6:
                break
        assert current()['page'] == 6, current()
        assert abs(current()['zoom'] - 1.5) < .001, current()
        assert current()['crop'][1] > 0, current()
        print(f'PASS: include/formula auto-follow save-to-display {time.monotonic()-start:.3f}s; crop={current()["crop"]}; zoom={current()["zoom"]}')
        os.write(master, b'g')
        pump(.8)
        output.clear()
        pump(.5)
        assert not output, 'manual navigation should settle without forced follow'
        before_error_pdf = pdf.read_bytes()
        save('#let broken = (\n')
        pump(1)
        assert b'Typst:' in output, output[-2000:]
        assert b'compilation error' in output, 'missing centered error panel'
        assert b'theory.typ' in output, 'missing diagnostic source location'
        assert proc.poll() is None
        assert pdf.read_bytes() == before_error_pdf, 'failed compile replaced PDF'
        output.clear()
        # A retry must dismiss stale diagnostics during the 200 ms debounce,
        # before a successful compiler reply can clear them as a side effect.
        save(source)
        pump(.12)
        assert b'Compiling' in output, 'retry did not repaint before compilation'
        assert b'compilation error' not in output, 'retry repainted stale error panel'
        for i in range(40):
            save(source.replace('c^2', 'c^' + str(i + 3)))
            pump(.01)
        save(source.replace('c^2', 'c^99'))
        pump(2)
        assert current()['page'] == 6, current()
        assert abs(current()['zoom'] - 1.5) < .001, current()
        assert current()['crop'][1] > 0, current()
        assert current()['revision'] > 2
        os.write(master, b'sg')
        pump(.5)
        output.clear()
        save(source.replace('c^2', 'c^100'))
        pump(1.5)
        assert current()['page'] == 0, 'sync-off must retain manual page'
        os.write(master, b'f')
        pump(1)
        assert current()['page'] == 6, 'manual follow must work with sync disabled'
        os.write(master, b'\x03')
        deadline = time.monotonic() + 3
        while proc.poll() is None and time.monotonic() < deadline:
            if select.select([master], [], [], .01)[0]:
                output.extend(os.read(master, 65536))
        assert proc.wait(timeout=1) == 0, output[-2000:]
        assert termios.tcgetattr(slave) == before
        if args.no_pdf:
            assert pdf.read_bytes() == sentinel, '--no-pdf overwrote existing file'
        print('PASS: error recovery / 40 rapid atomic saves / manual navigation / sync toggle / manual follow / clean exit')
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
