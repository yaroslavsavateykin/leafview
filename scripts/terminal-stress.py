#!/usr/bin/env python3
"""Run INSIDE a disposable real terminal (optionally tmux), never a user's pane.
Proxy a PTY to that terminal, including real capability replies. Save measurements
outside the repository. This exercises the emulator, unlike pty-smoke.py.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import termios
import time
import tty

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('pdf')
parser.add_argument('--binary', default=str(Path(__file__).resolve().parents[1] / 'target/release/leafview'))
parser.add_argument('--result', required=True)
parser.add_argument('--seconds', type=float, default=35)
parser.add_argument('--mode', default='native')
parser.add_argument('--quit-signal', type=int)
parser.add_argument('--resize', action='store_true', help='resize the proxied PTY during navigation')
args = parser.parse_args()
master, slave = pty.openpty()
original = termios.tcgetattr(0)
size = fcntl.ioctl(0, termios.TIOCGWINSZ, bytes(8))
fcntl.ioctl(slave, termios.TIOCSWINSZ, size)
before = termios.tcgetattr(slave)
env = {**os.environ, 'LEAFVIEW_GRAPHICS': args.mode}
proc = subprocess.Popen([args.binary, args.pdf], stdin=slave, stdout=slave, stderr=slave,
                        env=env, start_new_session=True)
start = time.monotonic()
peak = threads = children = total = transfers = 0
last_keys = 0
quit_at = None
samples = []
restored = False
try:
    tty.setraw(0)
    while time.monotonic() - start < args.seconds + 8:
        elapsed = time.monotonic() - start
        ready, _, _ = select.select([master, 0], [], [], .01)
        if master in ready:
            data = os.read(master, 1024 * 1024)
            total += len(data)
            transfers += data.count(b'a=T')
            view = memoryview(data)
            while view:
                view = view[os.write(1, view):]
        if 0 in ready:
            data = os.read(0, 65536)
            if data:
                os.write(master, data)
        if elapsed > 3 and elapsed < args.seconds and elapsed - last_keys > .09:
            keys = [b'jjkkHL', b'lh', b'+', b'-', b'Gg', b'z', b'3\r'][int(elapsed * 2) % 7]
            os.write(master, keys)
            if args.resize and int(elapsed * 10) % 9 == 0:
                rows, cols, xp, yp = struct.unpack('HHHH', size)
                delta = int(elapsed) % 4
                fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', max(2, rows - delta), max(5, cols - delta * 3), xp, yp))
                os.kill(proc.pid, signal.SIGWINCH)
            last_keys = elapsed
        if elapsed >= args.seconds and quit_at is None:
            quit_at = time.monotonic()
            if args.quit_signal:
                os.kill(proc.pid, args.quit_signal)
            else:
                os.write(master, b'q')
        try:
            status = dict(line.split(':', 1) for line in Path(f'/proc/{proc.pid}/status').read_text().splitlines() if ':' in line)
            peak = max(peak, int(status.get('VmHWM', '0').split()[0]))
            threads = max(threads, int(status['Threads']))
            child_ids = Path(f'/proc/{proc.pid}/task/{proc.pid}/children').read_text().split()
            children = max(children, len(child_ids))
            if not samples or elapsed - samples[-1]['seconds'] > 1:
                samples.append({'seconds': round(elapsed, 2), 'rss_kib': int(status.get('VmRSS', '0').split()[0]), 'cpu_ticks': Path(f'/proc/{proc.pid}/stat').read_text().split()[13:15]})
        except (FileNotFoundError, ProcessLookupError):
            pass
        if proc.poll() is not None:
            restored = termios.tcgetattr(slave) == before
            break
finally:
    if proc.poll() is None:
        proc.kill()
    code = proc.wait()
    termios.tcsetattr(0, termios.TCSANOW, original)
    os.close(master)
    os.close(slave)
    Path(args.result).write_text(json.dumps({'exit': code, 'tty_restored': restored, 'peak_kib': peak,
        'threads': threads, 'children': children, 'output_bytes': total, 'transfers': transfers,
        'duration': time.monotonic() - start, 'quit_seconds': None if quit_at is None else time.monotonic() - quit_at,
        'samples': samples}, indent=2))
