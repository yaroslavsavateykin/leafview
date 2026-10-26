#!/usr/bin/env python3
"""Headless PTY tests: fallback display, keys, resize, atomic reload, TTY cleanup.
Not a substitute for visual Kitty/Ghostty validation. Linux + Python standard library.
"""
import fcntl
import os
import json
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="leafview-test-") as tmp:
    source = Path(tmp) / "live.pdf"
    fixture = (root / "tests/fixtures/sample.pdf").read_bytes()
    source.write_bytes(fixture)
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 90, 0, 0))
    before = termios.tcgetattr(slave)
    opener = Path(tmp) / "xdg-open"
    opened = Path(tmp) / "opened-url"
    opener.write_text('#!/usr/bin/env python3\nimport sys\nfrom pathlib import Path\nPath(' + repr(str(opened)) + ').write_text(sys.argv[1])\n')
    opener.chmod(0o755)
    trace = Path(tmp) / 'trace.jsonl'
    env = {**os.environ, "TERM": "xterm-256color", "PATH": tmp + os.pathsep + os.environ['PATH'],
           "LEAFVIEW_TRACE": str(trace), "LEAFVIEW_GRAPHICS": "safe"}
    for key in ["TMUX", "TERM_PROGRAM", "KITTY_WINDOW_ID"]:
        env.pop(key, None)
    proc = subprocess.Popen([str(root / "target/release/leafview"), str(source)],
                            stdin=slave, stdout=slave, stderr=slave, env=env,
                            start_new_session=True)
    output = bytearray()
    peak = 0

    def pump(seconds):
        global peak
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([master], [], [], 0.01)[0]:
                output.extend(os.read(master, 1024 * 1024))
            try:
                for line in Path(f"/proc/{proc.pid}/status").read_text().splitlines():
                    if line.startswith("VmHWM:"):
                        peak = max(peak, int(line.split()[1]))
            except FileNotFoundError:
                pass

    try:
        pump(3)
        assert proc.poll() is None, output[-1000:]
        # Device-status query must precede mouse reporting, or unsolicited mouse
        # events can interfere with the graphics capability parser.
        assert b'\x1b[?1003h' in output
        os.write(master, b'\x1bOQ')  # F2: legacy full-width document flow
        pump(.5)
        assert trace.exists(), 'initial PDF must render'
        assert abs(json.loads(trace.read_text().splitlines()[-1])['zoom'] - 1.0) < .001
        # Plain arrows/hjkl scroll without changing pages; Shift turns pages.
        for key in (b'h', b'j', b'k', b'l', b'\x1b[A', b'\x1b[B', b'\x1b[C', b'\x1b[D'):
            os.write(master, b'zG')
            pump(.3)
            os.write(master, b'g' + key)
            pump(.5)
            assert json.loads(trace.read_text().splitlines()[-1])['page'] == 0
        for key in (b'L', b'J', b'\x1b[1;2C', b'\x1b[1;2B'):
            os.write(master, b'g')
            pump(.3)
            os.write(master, key)
            pump(.5)
            assert json.loads(trace.read_text().splitlines()[-1])['page'] == 1
        for key in (b'H', b'K', b'\x1b[1;2D', b'\x1b[1;2A'):
            os.write(master, b'G')
            pump(.3)
            os.write(master, key)
            pump(.5)
            assert json.loads(trace.read_text().splitlines()[-1])['page'] == 1
        # Over 1000 key events; then finish on page 3 at 105%.
        os.write(master, b"lh+-jkHL" * 140)
        pump(6)
        start = len(output)
        os.write(master, b"zG+")
        pump(2)
        assert proc.poll() is None
        assert abs(json.loads(trace.read_text().splitlines()[-1])['zoom'] - 1.05) < .001
        start = len(output)
        os.write(master, b"-")
        pump(.3)
        assert abs(json.loads(trace.read_text().splitlines()[-1])['zoom'] - 1.0) < .001
        # A failed reload must retain the previous document.
        source.write_bytes(b"broken PDF")
        pump(1)
        assert b"Reload:" in output, "no auto-reload error"
        replacement = source.with_suffix(".new")
        replacement.write_bytes(fixture)
        replacement.replace(source)
        pump(2)
        assert proc.poll() is None
        # Resize repeatedly, including an unusably small viewport.
        for rows, cols in [(1, 1), (15, 40), (40, 120), (30, 90)] * 5:
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            os.kill(proc.pid, signal.SIGWINCH)
            pump(0.04)
        pump(1)
        # Real SGR mouse reports, using the checked-in Typst annotation fixture.
        source.write_bytes((root / 'tests/fixtures/typst-links.pdf').read_bytes())
        pump(2)
        os.write(master, b'zg')
        pump(2)
        output.clear()
        os.write(master, b'\x1b[<35;16;11M')  # move over DOI, one-based cells
        pump(0.3)
        assert b'doi.org/10.1000/182' in output, output[-1500:]
        assert not opened.exists(), 'hover must not open browser'
        before_hover = len(output)
        for _ in range(50):
            os.write(master, b'\x1b[<35;16;11M')
        pump(0.3)
        assert len(output) == before_hover, 'unchanged hover should not redraw'
        os.write(master, b'\x1b[<0;16;11M\x1b[<0;16;11m')
        pump(0.5)
        assert opened.read_text() == 'https://doi.org/10.1000/182'
        opener.write_text('#!/usr/bin/env python3\nraise SystemExit(1)\n')
        output.clear()
        os.write(master, b'\x1b[<0;16;11M\x1b[<0;16;11m')
        pump(0.5)
        os.write(master, b'\x1b[<35;80;25M')  # leave link to expose error status
        pump(0.2)
        assert b'Browser:' in output, 'failed launcher should report error'
        output.clear()
        os.write(master, b'\x1b[<35;16;17M')
        pump(0.2)
        assert b'Page 2' in output, output[-1000:]
        os.write(master, b'\x1b[<0;16;17M\x1b[<0;16;17m')
        pump(1)
        output.clear()
        os.write(master, b'\x1b[<35;16;11M')
        pump(0.2)
        assert b'Page 1' in output, 'internal click did not reach destination page'
        os.write(master, b'b')
        pump(1)
        os.write(master, b'\x1b[<35;80;25M')
        pump(0.2)
        output.clear()
        os.write(master, b'\x1b[<35;16;11M')
        pump(0.2)
        assert b'doi.org/10.1000/182' in output, 'history did not restore source position'
        start = time.monotonic()
        os.write(master, b"q")
        while proc.poll() is None and time.monotonic() - start < 3:
            pump(0.02)
        assert proc.wait(timeout=1) == 0, output[-2000:]
        assert termios.tcgetattr(slave) == before, "TTY flags not restored"
        assert b"\x1b[?1049l" in output, "alternate screen not restored"
        assert b'\x1b[?1003l' in output, 'mouse capture not disabled'
        print(f"PASS: PTY display/navigation/auto-reload/resize/quit; peak RSS {peak} KiB; quit {time.monotonic()-start:.3f}s; output {len(output)} bytes")
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)

    # Termination during rendering must use the normal resource/TTY cleanup path.
    for sig in [signal.SIGTERM, signal.SIGHUP, signal.SIGINT]:
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 90, 0, 0))
        before = termios.tcgetattr(slave)
        proc = subprocess.Popen([str(root / "target/release/leafview"), str(source)],
                                stdin=slave, stdout=slave, stderr=slave,
                                env={**env, "LEAFVIEW_GRAPHICS": "safe"}, start_new_session=True)
        captured = bytearray()
        try:
            deadline = time.monotonic() + 5
            while b'PDF files' not in captured and time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    captured.extend(os.read(master, 65536))
            assert b'PDF files' in captured, 'signal test did not initialize'
            # Let signal registration finish after the first frame.
            time.sleep(.1)
            os.write(master, b'\t' + b'+jlHL' * 20)
            os.kill(proc.pid, sig)
            deadline = time.monotonic() + 3
            while proc.poll() is None and time.monotonic() < deadline:
                if select.select([master], [], [], .02)[0]:
                    captured.extend(os.read(master, 65536))
            assert proc.wait(timeout=1) == 0, f'signal {sig}: abnormal exit'
            assert termios.tcgetattr(slave) == before, f'signal {sig}: TTY not restored'
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()
            os.close(master)
            os.close(slave)
    print('PASS: SIGTERM/SIGHUP/SIGINT during navigation restore TTY')

    # Explicit compatibility mode skips graphics queries even inside tmux.
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 20, 70, 0, 0))
    before = termios.tcgetattr(slave)
    env = {**os.environ, "TERM": "tmux-256color", "TMUX": "/tmp/leafview-test",
           "TERM_PROGRAM": "ghostty", "LEAFVIEW_GRAPHICS": "safe"}
    proc = subprocess.Popen([str(root / "target/release/leafview"), str(source)],
                            stdin=slave, stdout=slave, stderr=slave, env=env,
                            start_new_session=True)
    captured = bytearray()
    try:
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline and b"PDF files" not in captured:
            if select.select([master], [], [], 0.05)[0]:
                captured.extend(os.read(master, 65536))
            assert proc.poll() is None, captured[-1000:]
        assert b"PDF files" in captured, captured[-2000:]
        assert b"\x1b_G" not in captured, "unexpected Kitty graphics data in safe mode"
        assert b"\x1b[5n" not in captured, "safe mode must not query graphics"
        os.write(master, b"\t" + b"jlHL" * 100 + b"q")
        deadline = time.monotonic() + 5
        while proc.poll() is None and time.monotonic() < deadline:
            if select.select([master], [], [], 0.02)[0]:
                captured.extend(os.read(master, 65536))
        assert proc.wait(timeout=1) == 0
        assert termios.tcgetattr(slave) == before, "safe mode did not restore TTY"
        assert b"\x1b[?1049l" in captured, "safe mode did not exit alternate screen"
        print("PASS: explicit compatibility startup, scroll storm, terminal restoration")
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        os.close(master)
        os.close(slave)
