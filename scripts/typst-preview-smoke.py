#!/usr/bin/env python3
"""Real Typst -> PDF -> raster -> Kitty PNG PTY regression (not GPU certification)."""
import argparse, base64, fcntl, io, json, os, pty, re, select, struct, subprocess, tempfile, termios, time
from pathlib import Path
from PIL import Image
parser=argparse.ArgumentParser()
parser.add_argument('--binary',default='target/debug/leafview')
args=parser.parse_args()
root=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='leafview-preview-') as tmp:
    tmp=Path(tmp);source=tmp/'main.typ';trace=tmp/'trace.jsonl'
    def write(n):
        text=f'''#set page(width: 180pt, height: 240pt, margin: 10pt)
#set text(size: 20pt)
#rect(width: 100pt, height: 80pt, fill: rgb("#{n:02x}4080"))
Generation {n}
#pagebreak()
#rect(width: 100pt, height: 80pt, fill: red)
Second page {n}
'''
        staged=tmp/'main.new';staged.write_text(text);staged.replace(source)
    write(1)
    master,slave=pty.openpty();fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',30,100,0,0))
    proc=subprocess.Popen([str(root/args.binary),str(source)],stdin=slave,stdout=slave,stderr=slave,start_new_session=True,env={**os.environ,'LEAFVIEW_GRAPHICS':'kitty','LEAFVIEW_QUERY_TIMEOUT_MS':'100','LEAFVIEW_TRACE':str(trace),'XDG_STATE_HOME':str(tmp),'LEAFVIEW_KEYBINDINGS':str(tmp/'keys.toml')})
    output=bytearray()
    def pump(seconds):
        end=time.monotonic()+seconds
        while time.monotonic()<end:
            if select.select([master],[],[],.01)[0]:
                chunk=os.read(master,1024*1024);output.extend(chunk)
                if b'\x1b[6n' in chunk:os.write(master,b'\x1b[1;1R')
            assert proc.poll() is None,output[-2000:]
    def current():
        try:
            return json.loads(trace.read_text().splitlines()[-1]) if trace.exists() else None
        except (ValueError, IndexError):
            return None
    def wait_frame(old=None):
        deadline=time.monotonic()+15
        while time.monotonic()<deadline:
            pump(.05)
            frame=current()
            if frame and (old is None or frame['revision']>old):return frame
        raise AssertionError(('no fresh frame',old,current(),output[-1000:]))
    def pngs(data):
        payload=bytearray();images=[]
        for match in re.finditer(rb'\x1b_G([^;]*);(.*?)\x1b\\',bytes(data),re.S):
            header,chunk=match.groups()
            if b'f=100' in header:payload.clear()
            if b'm=' not in header:continue
            payload.extend(base64.b64decode(chunk))
            if b'm=0' in header:
                image=Image.open(io.BytesIO(payload));image.load();images.append(image.convert('RGB'));payload.clear()
        return images
    try:
        frame=wait_frame();pump(.2)
        assert frame['raster'][0]>0 and frame['raster'][1]>0
        for n in range(2,9):
            old=frame['revision'];write(n);frame=wait_frame(old);pump(.15)
            # Alternate pages, fit and full-width mode during recompilation.
            os.write(master,b'gg' if n%2 else b'G');pump(.25)
            os.write(master,b'z' if n%2 else b'0');pump(.25)
        for n in range(9,35):write(n);pump(.01)
        frame=wait_frame(frame['revision']);pump(.4)
        # Idle is stable: no timer-driven clears, transfers or new frames.
        pump(.5);old_frame=current();start=len(output);pump(16)
        idle=output[start:]
        assert b'f=100,t=d' not in idle, 'idle unexpectedly uploaded PNG'
        assert b'\x1b[2J' not in idle, 'idle unexpectedly cleared screen'
        assert current()==old_frame, 'idle must not compile or rasterize'
        frame=current()
        # Repeated edits on the active slide, without navigation, must publish
        # fresh PNGs and placeholders even when image slots are reused.
        for n in range(35,71):
            old=frame['revision'];start=len(output);write(n)
            frame=wait_frame(old);pump(.08)
            assert b'f=100,t=d' in output[start:], ('missing edited slide PNG',n)
            assert '\U0010eeee'.encode() in output[start:], ('missing edited slide placement',n)
        # Warm both zoom cache entries, then revisit 100% repeatedly. Cached
        # images must remain terminal-resident: no clears or repeated uploads.
        os.write(master,b'z');pump(.4)
        for _ in range(4):
            os.write(master,b'-');pump(.35)
            start=len(output);os.write(master,b'+');pump(.5)
            wire=bytes(output[start:])
            assert b'\x1b[2J' not in wire, 'cached zoom cleared the visible PDF'
            assert b'f=100,t=d' not in wire, 'cached zoom uploaded PNG again'
            assert '\U0010eeee'.encode() in wire, 'missing 100% placement'
        images=pngs(output)
        assert len(images)>=8,('missing transfers',len(images))
        for i,image in enumerate(images):
            assert image.width>0 and image.height>0
            sample=image.resize((64,64))
            pixels=list(sample.get_flattened_data() if hasattr(sample,'get_flattened_data') else sample.getdata())
            dark=sum(min(pixel)<180 for pixel in pixels)
            assert dark>10,('blank PNG',i,image.size,dark)
        os.write(master,b'\x03');assert proc.wait(timeout=5)==0
        print(f'PASS: {len(images)} decoded nonblank PNGs / both pages / fit + full width / 69 atomic saves / stable idle (no timer refresh) / active-slide slot reuse / cached 95%–100% round trips / latest frame')
    finally:
        if proc.poll() is None:proc.kill();proc.wait()
        os.close(master);os.close(slave)
