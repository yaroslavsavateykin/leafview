#!/usr/bin/env python3
"""Real Tectonic/SyncTeX PTY integration. Requires a warmed Tectonic bundle.
Uses target/release/leafview; --no-pdf checks non-publication.
Not a visual terminal-emulator test.
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
with tempfile.TemporaryDirectory(prefix='leafview-tectonic-') as tmp:
    project = Path(tmp) / 'project with spaces'
    shutil.copytree(root / 'tests/fixtures/tectonic-live', project)
    # Input names with spaces must survive both Tectonic and Makefile rules.
    main = project / 'main document.tex'
    (project / 'main.tex').rename(main)
    target = project / 'chapters/section.tex'
    source = target.read_text()
    pdf = main.with_suffix('.pdf')
    sentinel = b'unchanged existing PDF'
    if args.no_pdf:
        pdf.write_bytes(sentinel)
    trace = Path(tmp) / 'trace.jsonl'
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 90, 0, 0))
    before = termios.tcgetattr(slave)
    command = [str(root / 'target/release/leafview'), 'tectonic', str(main)]
    if args.no_pdf:
        command.append('--no-pdf')
    proc = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave,
        env={**os.environ, 'LEAFVIEW_GRAPHICS': 'safe', 'LEAFVIEW_TRACE': str(trace)}, start_new_session=True)
    output = bytearray()
    def pump(seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([master], [], [], .01)[0]:
                output.extend(os.read(master, 1024 * 1024))
            assert proc.poll() is None, output[-4000:]
    def current():
        return json.loads(trace.read_text().splitlines()[-1])
    def wait_for(predicate, timeout=15):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            pump(.05)
            if predicate():
                return
        raise AssertionError((current() if trace.exists() else None, output[-4000:]))
    def save(path, text):
        replacement = path.with_suffix('.save')
        replacement.write_text(text)
        replacement.replace(path)
    def revision():
        return current()['revision']
    def changed(path, text):
        old = revision()
        save(path, text)
        wait_for(lambda: revision() > old)
    def check_pdf():
        assert pdf.read_bytes() == sentinel if args.no_pdf else pdf.read_bytes().startswith(b'%PDF')
    try:
        wait_for(lambda: trace.exists() and revision() > 0, 60)
        check_pdf()
        os.write(master, b'+')
        pump(.7)
        assert current()['zoom'] == 1.25
        changed(target, source.replace('original paragraph', 'changed paragraph'))
        if shutil.which('synctex'):
            assert current()['page'] == 1, current()
        assert current()['zoom'] == 1.25
        # Adding/deleting paragraphs; latest hunk in an included source.
        changed(target, source + '\nAn added paragraph.\\par\n')
        changed(target, source)
        old_position = (current()['page'], current()['zoom'], current()['crop'])
        bib = project / 'references.bib'
        changed(bib, bib.read_text().replace('2024', '2025'))
        assert (current()['page'], current()['zoom'], current()['crop']) == old_position
        old = revision()
        image = project / 'figures/example.png'
        replacement = image.with_suffix('.new')
        # Replace the actual pixels (valid PNG), not only its timestamp.
        import zlib
        def chunk(kind, data):
            return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
        replacement.write_bytes(b"\x89PNG\r\n\x1a\n" +
            chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 2, 0, 0, 0)) +
            chunk(b"IDAT", zlib.compress((b"\0" + b"\0\xff\0" * 2) * 2)) + chunk(b"IEND", b""))
        replacement.replace(image)
        wait_for(lambda: revision() > old)
        check_pdf()
        good = pdf.read_bytes()
        old = revision()
        output.clear()
        save(target, '\\undefinedLeafViewCommand\n')
        wait_for(lambda: b'compilation error' in output)
        assert revision() == old
        assert pdf.read_bytes() == good
        assert b'section.tex' in output
        assert b'\x1b[?1003l' in output, 'error must release terminal mouse selection'
        output.clear()
        os.write(master, b'y')
        wait_for(lambda: b']52;c;' in output)
        import base64
        payload = bytes(output).split(b']52;c;', 1)[1].split(b'\x07', 1)[0]
        copied = base64.b64decode(payload).decode()
        assert 'section.tex' in copied and 'undefined' in copied.lower(), copied
        for n in range(30):
            save(target, source.replace('original', f'edit {n}'))
            pump(.01)
        save(target, source.replace('original', 'final'))
        wait_for(lambda: revision() > old)
        assert current()['zoom'] == 1.25
        assert b'\x1b[?1003h' in output, 'recovery must restore viewer mouse navigation'
        os.write(master, b'sg')
        pump(.6)
        changed(target, source.replace('original', 'sync disabled'))
        assert current()['page'] == 0
        if shutil.which('synctex'):
            os.write(master, b'f')
            wait_for(lambda: current()['page'] == 1)
        old = revision()
        os.write(master, b'r')
        wait_for(lambda: revision() > old)
        os.write(master, b'q')
        end = time.monotonic() + 3
        while proc.poll() is None and time.monotonic() < end:
            if select.select([master], [], [], .01)[0]:
                os.read(master, 1024 * 1024)
        assert proc.wait(timeout=1) == 0
        assert termios.tcgetattr(slave) == before
        check_pdf()
        print('PASS: Tectonic initial/text/paragraphs/include/bib/image/error/recovery/rapid atomic saves/zoom/sync/force/quit/spaces' + (' --no-pdf' if args.no_pdf else ''))
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
