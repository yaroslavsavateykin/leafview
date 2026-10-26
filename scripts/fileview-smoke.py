#!/usr/bin/env python3
"""Headless PTY checks for PDF tree, history, failed open, focus and watcher switching."""
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

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='leafview-files-') as tmp:
    tmp = Path(tmp)
    project = tmp / 'PDF library'
    project.mkdir()
    (project / 'chapters').mkdir()
    shutil.copy(root / 'tests/fixtures/sample.pdf', project / 'a.pdf')
    shutil.copy(root / 'tests/fixtures/typst-links.pdf', project / 'chapters/b.PDF')
    (project / 'z-broken.pdf').write_text('not a pdf')
    (project / 'hidden.tex').write_text('not shown')
    trace = tmp / 'trace.jsonl'
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 32, 110, 0, 0))
    before = termios.tcgetattr(slave)
    proc = subprocess.Popen([str(root / 'target/release/leafview'), str(project)],
        stdin=slave, stdout=slave, stderr=slave, start_new_session=True,
        env={**os.environ, 'LEAFVIEW_GRAPHICS': 'safe', 'LEAFVIEW_TRACE': str(trace),
             'XDG_STATE_HOME': str(tmp / 'state')})
    output = bytearray()
    def pump(seconds=.5):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([master], [], [], .01)[0]:
                output.extend(os.read(master, 1024 * 1024))
            assert proc.poll() is None, output[-2000:]
    def send(keys):
        os.write(master, keys)
        pump()
    def current():
        return json.loads(trace.read_text().splitlines()[-1])
    def wait(predicate):
        for _ in range(60):
            pump(.1)
            if predicate():
                return
        raise AssertionError(output[-3000:])
    history = tmp / 'state/leafview/recent.json'
    try:
        pump(1)
        assert b'PDF files' in output
        assert b'hidden.tex' not in output
        # Root -> chapters -> expand -> b.PDF -> open.
        send(b'jl')
        send(b'j\r')
        wait(lambda: history.exists())
        assert Path(json.loads(history.read_text())[0]).name == 'b.PDF'
        assert current()['revision'] > 0
        # Document navigation remains active after open; 5% keyboard zoom.
        send(b'+')
        assert abs(current()['zoom'] - 1.05) < .001
        # A broken PDF keeps the previous document and records no history.
        send(b'\tG\r')
        wait(lambda: b'ot open PDF:' in output or b'Cannot open PDF' in output)
        assert Path(json.loads(history.read_text())[0]).name == 'b.PDF'
        send(b'\t')
        assert abs(current()['zoom'] - 1.05) < .001
        # Open another PDF and confirm its parent watcher follows it.
        send(b'\tGk\r')
        wait(lambda: Path(json.loads(history.read_text())[0]).name == 'a.pdf')
        revision = current()['revision']
        replacement = project / 'replacement.pdf'
        shutil.copy(root / 'tests/fixtures/typst-links.pdf', replacement)
        replacement.replace(project / 'a.pdf')
        wait(lambda: current()['revision'] > revision)
        # Sidebar toggle changes document viewport, then restores it.
        width = current()['raster'][0]
        send(b'\x1bOQ')  # F2
        assert current()['raster'][0] > width
        send(b'\x1bOQ')
        assert current()['raster'][0] == width
        os.write(master, b'q')
        proc.wait(timeout=3)
        assert proc.returncode == 0
        assert termios.tcgetattr(slave) == before
        print('PASS: PDF-only tree / nested open / focus / 5% zoom / failed open retention / recent history / switched watcher / F2 / TTY cleanup')
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
