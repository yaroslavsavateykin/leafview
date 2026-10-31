#!/usr/bin/env python3
"""Capture real Kitty pixels on a dedicated Xvfb display; see README.md."""
import argparse
import os
from pathlib import Path
import signal
import subprocess
import time
from Xlib import X, XK, display
from Xlib.ext import xtest
from PIL import Image

p = argparse.ArgumentParser()
p.add_argument('--root', type=Path, required=True)
p.add_argument('--display', default=':97')
p.add_argument('--output', type=Path, default=Path('docs/assets'))
a = p.parse_args()
if not (a.root / 'environment.sh').is_file() or not str(a.root).startswith('/tmp/leafview-showcase.'):
    p.error('Expected a workspace created by prepare-demo.sh')
if a.display in (':0', ':1', os.environ.get('DISPLAY')):
    p.error('Refusing desktop display')
d = display.Display(a.display)
root = d.screen().root
windows = [w for w in root.query_tree().children if w.get_attributes().map_state == X.IsViewable]
assert len(windows) == 1, 'Use an otherwise empty Xvfb display'
win = windows[0]
d.set_input_focus(win, X.RevertToParent, X.CurrentTime)
xtest.fake_input(d, X.MotionNotify, x=800, y=500)
d.sync()
time.sleep(.3)

def key(name):
    codes = [d.keysym_to_keycode(XK.string_to_keysym(k)) for k in name.split('+')]
    assert all(codes), name
    for c in codes: xtest.fake_input(d, X.KeyPress, c)
    d.sync(); time.sleep(.06)
    for c in reversed(codes): xtest.fake_input(d, X.KeyRelease, c)
    d.sync()

mapping = d.get_keyboard_mapping(8, 248)
def text(s):
    for ch in s:
        sym = ord(ch)
        found = next(((i + 8, row.index(sym)) for i, row in enumerate(mapping) if sym in row and row.index(sym) < 2), None)
        if found is None: raise ValueError(ch)
        code, shift = found
        if shift: xtest.fake_input(d, X.KeyPress, d.keysym_to_keycode(XK.XK_Shift_L))
        xtest.fake_input(d, X.KeyPress, code)
        xtest.fake_input(d, X.KeyRelease, code)
        if shift: xtest.fake_input(d, X.KeyRelease, d.keysym_to_keycode(XK.XK_Shift_L))
        d.sync()
        time.sleep(.11)

def click(x, y):
    xtest.fake_input(d, X.MotionNotify, x=x, y=y)
    xtest.fake_input(d, X.ButtonPress, 1); d.sync(); time.sleep(.1)
    xtest.fake_input(d, X.ButtonRelease, 1); d.sync(); time.sleep(.4)

def command(s):
    text(':' + s); time.sleep(.4); key('Return'); time.sleep(.7)

def shot(name):
    time.sleep(.8)
    pixels = root.get_image(0, 0, 2560, 1440, X.ZPixmap, 0xffffffff)
    im = Image.frombytes('RGB', (2560, 1440), pixels.data, 'raw', 'BGRX')
    im.save(a.output / 'screenshots' / (name + '.png'), optimize=True)

def drag(x, y, xx, yy):
    xtest.fake_input(d, X.MotionNotify, x=x, y=y)
    xtest.fake_input(d, X.ButtonPress, 1); d.sync(); time.sleep(.1)
    xtest.fake_input(d, X.MotionNotify, x=xx, y=yy); d.sync(); time.sleep(.1)
    xtest.fake_input(d, X.ButtonRelease, 1); d.sync()
    xtest.fake_input(d, X.MotionNotify, x=1800, y=1000); d.sync()

for sub in ['screenshots', 'demo']: (a.output / sub).mkdir(parents=True, exist_ok=True)
# Start before launch: actual shell, default LeafView command, then browser.
started = time.monotonic()
def scene(label):
    print(f'{time.monotonic()-started:05.1f}s {label}', flush=True)

rec = subprocess.Popen(['ffmpeg', '-y', '-loglevel', 'error', '-f', 'x11grab', '-draw_mouse', '0',
    '-framerate', '30', '-video_size', '2560x1440', '-i', a.display,
    '-c:v', 'libx264', '-preset', 'fast', '-crf', '24', '-pix_fmt', 'yuv420p',
    '-movflags', '+faststart', str(a.output / 'demo/leafview-demo.mp4')])
try:
    scene('Default launch')
    time.sleep(1); text('leafview'); time.sleep(.6); key('Return'); time.sleep(2)
    shot('welcome')
    scene('Open a PDF from the directory')
    key('Down'); time.sleep(.5); key('Right'); time.sleep(.6)
    key('Down'); time.sleep(.4); key('Down'); time.sleep(.4)
    key('Down'); time.sleep(.5); key('Return'); time.sleep(3)
    scene('PDF: turn pages, zoom in and out')
    key('n'); time.sleep(.8); key('p'); time.sleep(.8)
    key('n'); time.sleep(.8); key('p'); time.sleep(1)
    text('z'); time.sleep(.6); text('++++'); time.sleep(1.5)
    text('----'); time.sleep(1.5)
    text('0'); time.sleep(1); shot('pdf-viewer')
    scene('Navigate the working tree and open Typst')
    key('Control_L+h'); time.sleep(.7)
    key('Up'); time.sleep(.6); key('Up'); time.sleep(.6)
    key('Left'); time.sleep(.6); key('Right'); time.sleep(.6)
    key('Down'); time.sleep(.7); key('Return'); time.sleep(3)
    # The entry imports its style above documents/: establish the root in the
    # isolated project config before launch (not through a hidden CLI argument).
    scene('Open source')
    key('F4'); time.sleep(3)
    drag(548, 500, 385, 500); time.sleep(1)
    click(800, 200); key('Escape'); command('set noformatonsave')
    key('Alt_L+Right'); time.sleep(3); text('gg'); time.sleep(1); shot('hero')
    scene('Edit the title, letter by letter, then save')
    text('5G'); time.sleep(.7); text('A'); time.sleep(.5)
    text(' in Practice'); time.sleep(1); key('Escape'); time.sleep(.5)
    command('w'); time.sleep(3); text('gg'); shot('typst-preview')
    scene('Real completion and item documentation')
    text('8Go'); time.sleep(.5); text('#page'); time.sleep(2)
    key('Control_L+space'); time.sleep(1); shot('completion')
    key('Tab'); time.sleep(1.5); key('Control_L+space'); time.sleep(2)
    key('Tab'); time.sleep(1.5); key('Escape'); time.sleep(.5)
    text('u'); text('gg'); time.sleep(1)
    scene('Compilation settings')
    key('F6'); time.sleep(.5); text('o'); time.sleep(2); shot('settings')
    key('Down'); time.sleep(.6); key('Down'); time.sleep(.6)
    key('Return'); time.sleep(.4); text('2'); time.sleep(.7); key('Escape')
    key('Right'); time.sleep(1.3); key('Right'); time.sleep(1.3)
    key('Escape'); time.sleep(1)
    scene('Open a second source file')
    click(800, 200); key('Escape')
    command('e documents/research-note.typ'); time.sleep(1)
    command('bn'); time.sleep(.5); text('gg'); shot('editor')
    scene('Final workspace')
    time.sleep(2)
finally:
    rec.send_signal(signal.SIGINT)
    rec.wait(timeout=30)
