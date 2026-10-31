#!/usr/bin/env python3
"""Send X11 keys only to the disposable showcase display (never the desktop)."""
import os
import sys
from Xlib import X, XK, display
from Xlib.ext import xtest

name = os.environ.get('SHOWCASE_DISPLAY', ':97')
if name in (':0', ':1', os.environ.get('DISPLAY')):
    raise SystemExit('Refusing desktop display; use a dedicated Xvfb display')
d = display.Display(name)
root = d.screen().root
windows = root.query_tree().children
if not windows:
    raise SystemExit('No showcase terminal')
d.set_input_focus(windows[-1], X.RevertToParent, X.CurrentTime)
for chord in sys.argv[1:]:
    codes = [d.keysym_to_keycode(XK.string_to_keysym(k)) for k in chord.split('+')]
    if not all(codes):
        raise SystemExit(f'Unknown key: {chord}')
    for code in codes:
        xtest.fake_input(d, X.KeyPress, code)
    for code in reversed(codes):
        xtest.fake_input(d, X.KeyRelease, code)
d.sync()
