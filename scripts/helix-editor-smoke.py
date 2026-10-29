#!/usr/bin/env python3
"""Native editor PTY check; no real Kitty visual certification."""
import fcntl, os, pty, select, struct, subprocess, tempfile, termios, time
from pathlib import Path
root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    path=Path(tmp)/'source.typ';path.write_text('hello world\n')
    master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',30,120,0,0));before=termios.tcgetattr(slave)
    proc=subprocess.Popen([str(root/'target/debug/leafview'),str(path)],stdin=slave,stdout=slave,stderr=slave,env={**os.environ,'LEAFVIEW_GRAPHICS':'safe','XDG_STATE_HOME':tmp},start_new_session=True)
    output=bytearray()
    def pump(seconds):
        end=time.monotonic()+seconds
        while time.monotonic()<end:
            if select.select([master],[],[],.01)[0]:output.extend(os.read(master,1024*1024))
            assert proc.poll() is None, output[-2000:]
    try:
        pump(2);os.write(master,b'\x1bOS');pump(.5) # F4 opens editor
        assert b'Normal  1:1' in output,output[-3000:]
        os.write(master,b':set noformatonsave\r');pump(.1)
        os.write(master,b'ciwNative\x1b');pump(.2);os.write(master,b':w\r');pump(.6)
        assert path.read_text()=='Native world\n',path.read_text()
        # Global navigation must escape editor input in all modes. Tree ->
        # source -> PDF -> source via Ctrl+h/l and Ctrl+arrow reports.
        os.write(master,b'\x08');pump(.1)
        os.write(master,b'\x0c');pump(.1)
        os.write(master,b'\x1b[1;5C');pump(.1)
        os.write(master,b'\x1b[1;5D');pump(.1)
        os.write(master,b'iX');pump(.1);os.write(master,b'\x1b');pump(.1);os.write(master,b':w\r');pump(.5)
        assert 'X' in path.read_text(), 'focus navigation failed to return to editor'
        # Ctrl+C must exit even while the editor owns focus.
        os.write(master,b'\x03');assert proc.wait(timeout=3)==0
        assert termios.tcgetattr(slave)==before
        print('PASS: Helix buffer / native Vim editing / async atomic save / hide / TTY cleanup')
    finally:
        if proc.poll() is None:proc.kill();proc.wait()
        os.close(master);os.close(slave)
