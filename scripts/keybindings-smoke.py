#!/usr/bin/env python3
"""PTY: TOML remap, leader/WhichKey, palette and keyboard settings."""
import fcntl, os, pty, select, struct, subprocess, tempfile, termios, time
from pathlib import Path
root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    path=Path(tmp)/'main.typ';path.write_text('alpha beta\n')
    config=Path(tmp)/'keys.toml'
    config.write_text('[keybindings]\ntimeout_ms=800\n[keybindings.normal]\nx="editor.cursor.right"\n')
    master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',35,140,0,0))
    proc=subprocess.Popen([str(root/'target/debug/leafview'),str(path)],stdin=slave,stdout=slave,stderr=slave,start_new_session=True,env={**os.environ,'LEAFVIEW_GRAPHICS':'safe','XDG_STATE_HOME':tmp,'LEAFVIEW_KEYBINDINGS':str(config)})
    output=bytearray()
    def pump(seconds):
        end=time.monotonic()+seconds
        while time.monotonic()<end:
            if select.select([master],[],[],.02)[0]:output.extend(os.read(master,1024*1024))
            assert proc.poll() is None,output[-2000:]
    def send(data,seconds=.25):os.write(master,data);pump(seconds)
    try:
        pump(2);send(b'\x1bOS',.5);send(b':set noformatonsave\r')
        send(b'x');send(b'iX\x1b');send(b' w',.6)
        assert path.read_text()=='aXlpha beta\n',path.read_text()
        output.clear();send(b' ',.2);assert b'WhichKey' in output
        send(b'\x1b');send(b'\x1b[20~');send(b'editor.save') # F9 palette
        assert b'Command Palette' in output
        send(b'\x1b');send(b'\x1b[21~');send(b'editor.save') # F10 settings
        assert b'Keyboard Shortcuts' in output
        send(b'\x1b');os.write(master,b'\x03');assert proc.wait(timeout=3)==0
        print('PASS: TOML remap / leader save / WhichKey / Command Palette / Keyboard Shortcuts')
    finally:
        if proc.poll() is None:proc.kill();proc.wait()
        os.close(master);os.close(slave)
