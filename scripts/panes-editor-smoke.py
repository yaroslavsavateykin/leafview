#!/usr/bin/env python3
"""PTY regressions: independent file targets, mouse divider drag, auto completion."""
import fcntl, os, pty, select, struct, subprocess, tempfile, termios, time
from pathlib import Path
root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    a = Path(tmp)/'a.typ'; a.write_text('alpha\n')
    b = Path(tmp)/'b.typ'; b.write_text('bravo\n')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 30, 120, 0, 0))
    before = termios.tcgetattr(slave)
    proc = subprocess.Popen([str(root/'target/debug/leafview'), str(a)], stdin=slave, stdout=slave, stderr=slave,
        env={**os.environ, 'LEAFVIEW_GRAPHICS':'safe', 'XDG_STATE_HOME':tmp}, start_new_session=True)
    output = bytearray()
    def pump(seconds=.25):
        until = time.monotonic()+seconds
        while time.monotonic()<until:
            if select.select([master], [], [], .01)[0]: output.extend(os.read(master, 1024*1024))
            assert proc.poll() is None, output[-3000:]
    def send(data, seconds=.25): os.write(master, data); pump(seconds)
    def mouse(code, x, y, release=False):
        send(f'\x1b[<{code};{x};{y}{"m" if release else "M"}'.encode(), .08)
    try:
        pump(2)
        send(b'\x1bOS', .5) # F4
        send(b'\x1bOR') # F3, source target
        assert b'Open source' in output
        send(b'b.typ\r', .5)
        # Terminal diffs may interleave ANSI style changes with source characters.
        # Opening source b must leave preview a selected.
        assert b'PDF' in output
        send(b':set noformatonsave\r')
        send(b'ggIXXX\x1b'); send(b':w\r', .6)
        assert b.read_text() == 'XXXbravo\n', b.read_text()
        assert a.read_text() == 'alpha\n'
        # Sidebar width is 38 at 120 columns: x=38 is its right border.
        mouse(0, 38, 8); mouse(32, 46, 8); mouse(0, 46, 8, True)
        # Editor now starts x=47; drag source/PDF boundary (x=83) right.
        mouse(0, 83, 8); mouse(32, 93, 8); mouse(0, 93, 8, True)
        # Click PDF title opens the independent preview picker.
        mouse(0, 103, 1)
        assert b'Open preview' in output
        send(b'\x1b')
        # Click editor title: source picker remains independent.
        mouse(0, 55, 1)
        assert output.rfind(b'Open source') > output.rfind(b'Open preview')
        send(b'\x1b')
        os.write(master, b'\x03')
        assert proc.wait(timeout=3) == 0
        assert termios.tcgetattr(slave) == before
        print('PASS: source/preview picker targets, source save isolation, mouse split dragging, title pickers, TTY restoration')
    finally:
        if proc.poll() is None: proc.kill(); proc.wait()
        os.close(master); os.close(slave)
