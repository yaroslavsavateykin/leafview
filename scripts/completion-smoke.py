#!/usr/bin/env python3
"""Real PTY regression: tinymist filtering, non-mutating navigation, signature.
Requires cargo build --features typst,editor and tinymist on PATH.
"""
import fcntl, os, pty, select, struct, subprocess, tempfile, termios, time
from pathlib import Path
root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    path = Path(tmp) / 'main.typ'
    path.write_text('#page\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 35, 140, 0, 0))
    proc = subprocess.Popen([str(root/'target/debug/leafview'), str(path)], stdin=slave,
        stdout=slave, stderr=slave, start_new_session=True,
        env={**os.environ, 'LEAFVIEW_GRAPHICS':'safe', 'XDG_STATE_HOME':tmp,
             'LEAFVIEW_COMPLETION_CONFIG':str(Path(tmp)/'completion.json')})
    output = bytearray()
    def pump(seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([master], [], [], .02)[0]:
                output.extend(os.read(master, 1024*1024))
            assert proc.poll() is None, output[-2000:]
    def send(data, wait=.3):
        os.write(master, data); pump(wait)
    try:
        pump(2); send(b'\x1bOS', .5)  # F4
        send(b':set noformatonsave\r')
        send(b'A\x00', 2)  # Insert end of #page; Ctrl-Space
        assert b'pagebreak' in output, 'real tinymist menu missing'
        send(b'\t\t\x10')  # Tab/Tab/Ctrl-P must navigate, not open picker
        send(b'\x1b'); send(b':w\r', .5)
        assert path.read_text() == '#page\n', path.read_text()
        output.clear()
        send(b'A(', 2)
        assert b'Signature' in output, 'automatic signature missing'
        send(b'\x1b'); send(b':w\r', .5)
        assert path.read_text() == '#page()\n', path.read_text()
        os.write(master, b'\x03')
        assert proc.wait(timeout=3) == 0
        print('PASS: tinymist menu / Tab and Ctrl-P do not edit / automatic signature / paired call')
    finally:
        if proc.poll() is None:
            proc.kill(); proc.wait()
        os.close(master); os.close(slave)
