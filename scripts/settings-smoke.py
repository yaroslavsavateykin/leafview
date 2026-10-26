#!/usr/bin/env python3
"""Real Typst settings/profile/restart/browser PTY tests; no external Typst CLI."""
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
    config.write_text(json.dumps({'version': 1, 'default_profile': 'teacher', 'profiles': {
        'teacher': {'inputs': {'mode': 'TEACHER'}, 'jobs': 1},
        'student': {'inputs': {'mode': 'STUDENT'}, 'write_pdf': False}}}))
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
    def apply_from(index, action=19):
        old = revision()
        send(b'\t' * (action - index) + b'\r')
        wait(lambda: revision() > old)
    try:
        wait(lambda: revision() > 0)
        assert 'TEACHER' in pdf_text()
        send(b'+')
        # Runtime inputs override CLI/config and remain isolated until Apply.
        edit(2, '{"mode":"RUNTIME"}')
        assert 'TEACHER' in pdf_text()
        apply_from(2)
        assert 'RUNTIME' in pdf_text()
        assert abs(json.loads(trace.read_text().splitlines()[-1])['zoom'] - 1.05) < .001
        # Save global, retaining project configuration and profile state.
        edit(4, '2')
        apply_from(4, 21)
        global_path = tmp / 'global/leafview/config.json'
        assert json.loads(global_path.read_text())['profiles']['teacher']['jobs'] == 2
        # Change jobs mid-build repeatedly; only latest generation may publish.
        for jobs in (1, 2, 3, 1):
            edit(4, str(jobs))
            send(b'\t' * 15 + b'\r')
        pump(1)
        # Supersede an expensive build while it is running, then verify recovery.
        main.write_text('#for i in range(10000000) { [Slow content #i ] }\n')
        pump(.3)
        previous_helpers = {p for p in descendants(proc.pid) if Path(f'/proc/{p}/cmdline').exists()
            and b'--typst-compiler' in Path(f'/proc/{p}/cmdline').read_bytes()}
        edit(4, '2')
        send(b'\t' * 15 + b'\r')
        main.write_text('#sys.inputs.at("mode", default: "DEFAULT")\n')
        old = revision()
        send(b'r')
        wait(lambda: revision() > old)
        pump(.2)
        assert all(not Path(f'/proc/{p}').exists() for p in previous_helpers), 'superseded compiler not reaped'
        assert 'RUNTIME' in pdf_text()
        good = main.with_suffix('.pdf').read_bytes()
        # Switch student profile and save project. no-pdf must preserve last disk PDF.
        send(b'o\r\x1b[3~student\r')
        apply_from(0, 20)
        assert main.with_suffix('.pdf').read_bytes() == good
        assert json.loads(config.read_text())['default_profile'] == 'student'
        # Save only this file in the shared global files map.
        edit(4, '2')
        apply_from(4, 22)
        shared = json.loads(global_path.read_text())
        assert shared['files'][str(main)]['default_profile'] == 'student'
        assert shared['files'][str(main)]['profiles']['student']['jobs'] == 2
        # New profile is a draft until Apply; create it and return to student.
        send(b'o\r\x1b[3~custom\r')
        apply_from(0)
        send(b'o\r\x1b[3~student\r')
        apply_from(0)
        good = main.with_suffix('.pdf').read_bytes()
        # Disable automatic compile; edits wait until manual r.
        send(b'o' + b'\t' * 17 + b'\r')
        apply_from(17)
        old = revision()
        main.write_text('#sys.inputs.at("mode", default: "DEFAULT")\nManual only\n')
        pump(.7)
        assert revision() == old
        send(b'r')
        wait(lambda: revision() > old)
        # Invalid root stays in modal; Esc cancels without damaging the viewer.
        edit(1, '"/does/not/exist"')
        send(b'\t' * 18 + b'\r')
        assert b'project' in output and b'root:' in output, output[-3500:]
        send(b'\x1b')
        assert main.with_suffix('.pdf').read_bytes() == good
        # Compiler errors retain last document, then recover.
        old = revision()
        main.write_text('#let broken = (\n')
        send(b'r')
        wait(lambda: b'compilation error' in output)
        assert revision() == old
        main.write_text('Recovered\n')
        send(b'r')
        wait(lambda: revision() > old)
        # PDF -> Typst -> PDF inside one instance; old helpers must be reaped.
        send(b'\x1b')
        send(b'\x0brGkk\r')
        wait(lambda: history.exists() and Path(json.loads(history.read_text())[0]).name == 'a.pdf')
        send(b'\x08\x0bG\r')
        wait(lambda: Path(json.loads(history.read_text())[0]).name == 'main.typ')
        pump(1)
        send(b'o' + b'\t' * 4 + b'\r')
        # Reopened Typst loaded the per-file jobs value; cancel the field/panel.
        assert b'2' in output
        send(b'\x1b')
        send(b'\x1b')
        send(b'\x08\x0brGkk\r')
        wait(lambda: history.exists() and Path(json.loads(history.read_text())[0]).name == 'a.pdf')
        assert not any(' --typst-compiler ' in Path(f'/proc/{p}/cmdline').read_bytes().replace(b'\0', b' ').decode(errors='replace')
                       for p in descendants(proc.pid) if Path(f'/proc/{p}/cmdline').exists())
        os.write(master, b'\x03')
        proc.wait(timeout=4)
        assert proc.returncode == 0
        assert termios.tcgetattr(slave) == before
        time.sleep(.1)
        assert not any(Path(f'/proc/{p}').exists() for p in children_seen), 'leaked child process'
        print('PASS: implicit .typ / runtime inputs / profiles / jobs restart / atomic save / no-pdf / manual compile / invalid root / error retention / browser switching / child cleanup')
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
