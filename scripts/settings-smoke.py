#!/usr/bin/env python3
"""Real Typst settings/flags/restart/browser PTY tests; no external Typst CLI."""
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
with tempfile.TemporaryDirectory(prefix='leafview-settings-') as tmp:
    tmp = Path(tmp)
    main = tmp / 'main.typ'
    main.write_text('#sys.inputs.at("mode", default: "DEFAULT")\n')
    shutil.copy(root / 'tests/fixtures/sample.pdf', tmp / 'a.pdf')
    config = tmp / '.leafview.json'
    config.write_text(json.dumps({'inputs': {'mode': 'TEACHER'}, 'jobs': 1}))
    trace = tmp / 'trace.jsonl'
    history = tmp / 'state/leafview/recent.json'
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 38, 110, 0, 0))
    before = termios.tcgetattr(slave)
    proc = subprocess.Popen([str(root / 'target/release/leafview'), str(main), '--input', 'cli=preserved'],
        stdin=slave, stdout=slave, stderr=slave, start_new_session=True,
        env={**os.environ, 'LEAFVIEW_GRAPHICS': 'safe', 'LEAFVIEW_TRACE': str(trace),
             'XDG_CONFIG_HOME': str(tmp / 'global'), 'XDG_STATE_HOME': str(tmp / 'state')})
    output = bytearray()
    children_seen = set()
    def descendants(pid):
        result = set()
        try:
            for c in Path(f'/proc/{pid}/task/{pid}/children').read_text().split():
                c = int(c)
                result.add(c)
                result.update(descendants(c))
        except FileNotFoundError:
            pass
        return result
    def pump(seconds=.25):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([master], [], [], .01)[0]:
                output.extend(os.read(master, 1024 * 1024))
            children_seen.update(descendants(proc.pid))
            assert proc.poll() is None, output[-3000:]
    def send(keys):
        os.write(master, keys)
        pump()
    def wait(predicate, timeout=15):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            pump(.05)
            if predicate():
                return
        raise AssertionError(output[-4000:])
    def revision():
        return json.loads(trace.read_text().splitlines()[-1])['revision'] if trace.exists() else 0
    def pdf_text():
        return subprocess.check_output(['pdftotext', str(main.with_suffix('.pdf')), '-']).decode()
    def edit(index, value):
        send(b'o' + b'\t' * index + b'\r\x1b[3~' + value.encode() + b'\r')
    def apply_from(index, action=18):
        old = revision()
        send(b'\t' * (action - index) + b'\r')
        wait(lambda: revision() > old)
    try:
        wait(lambda: revision() > 0)
        # CLI inputs replace inherited inputs; runtime edits override CLI.
        send(b'+')
        edit(1, '{"mode":"RUNTIME"}')
        apply_from(1)
        assert 'RUNTIME' in pdf_text()
        edit(3, '2')
        apply_from(3, 19)
        global_path = tmp / 'global/leafview/config.json'
        shared = json.loads(global_path.read_text())
        assert set(shared) == {'files'}
        assert shared['files'][str(main)]['jobs'] == 2
        assert 'profiles' not in shared['files'][str(main)]
        assert abs(json.loads(trace.read_text().splitlines()[-1])['zoom'] - 1.05) < .001
        # Change parameters during an expensive build; obsolete helper must die.
        main.write_text('#for i in range(10000000) { [Slow content #i ] }\n')
        pump(.3)
        previous_helpers = {p for p in descendants(proc.pid) if Path(f'/proc/{p}/cmdline').exists()
            and b'--typst-compiler' in Path(f'/proc/{p}/cmdline').read_bytes()}
        edit(3, '1')
        send(b'\t' * 15 + b'\r')
        main.write_text('#sys.inputs.at("mode", default: "DEFAULT")\n')
        old = revision()
        send(b'r')
        wait(lambda: revision() > old)
        assert all(not Path(f'/proc/{p}').exists() for p in previous_helpers)
        good = main.with_suffix('.pdf').read_bytes()
        # Disable publication, save per-file flags.
        send(b'o' + b'\t' * 15 + b'\r')
        apply_from(15, 19)
        assert main.with_suffix('.pdf').read_bytes() == good
        assert json.loads(global_path.read_text())['files'][str(main)]['write_pdf'] is False
        # Disable auto compile, then build manually.
        send(b'o' + b'\t' * 16 + b'\r')
        apply_from(16)
        old = revision()
        main.write_text('Manual only\n')
        pump(.7)
        assert revision() == old
        send(b'r')
        wait(lambda: revision() > old)
        edit(0, '"/does/not/exist"')
        send(b'\t' * 18 + b'\r')
        assert b'root:' in output
        send(b'\x1b')
        assert main.with_suffix('.pdf').read_bytes() == good
        old = revision()
        main.write_text('#let broken = (\n')
        send(b'r')
        wait(lambda: b'compilation error' in output)
        assert revision() == old
        main.write_text('Recovered\n')
        send(b'r')
        wait(lambda: revision() > old)
        # Switch PDF/Typst; file flags load again from the shared config.
        send(b'\x1b')
        send(b'\x0brGkk\r')
        wait(lambda: history.exists() and Path(json.loads(history.read_text())[0]).name == 'a.pdf')
        send(b'\x08\x0bG\r')
        wait(lambda: Path(json.loads(history.read_text())[0]).name == 'main.typ')
        pump(1)
        send(b'\x08\x0brGkk\r')
        wait(lambda: Path(json.loads(history.read_text())[0]).name == 'a.pdf')
        assert not any(b'--typst-compiler' in Path(f'/proc/{p}/cmdline').read_bytes()
                       for p in descendants(proc.pid) if Path(f'/proc/{p}/cmdline').exists())
        os.write(master, b'\x03')
        proc.wait(timeout=4)
        assert proc.returncode == 0
        assert termios.tcgetattr(slave) == before
        time.sleep(.1)
        assert not any(Path(f'/proc/{p}').exists() for p in children_seen)
        print('PASS: flat flags / per-file shared JSON / runtime inputs / restart cancellation / no-pdf / manual build / error retention / browser / cleanup')
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
