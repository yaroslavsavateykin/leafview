#!/usr/bin/env python3
"""Generate synthetic public benchmark PDFs outside the repository. Requires Typst/Pillow."""
from pathlib import Path
import subprocess
import sys
from PIL import Image
out = Path(sys.argv[1] if len(sys.argv) > 1 else '/tmp/leafview-bench')
out.mkdir(parents=True, exist_ok=True)
Image.effect_noise((3000, 3000), 80).convert('RGB').save(out/'large.png')
cases = {
    'small': '= Small PDF\nHello Leafview.',
    'long': '#for i in range(120) { [= Page #i\nHello Leafview.] ; if i < 119 { pagebreak() } }',
    'vector': '#for i in range(2000) { line(length: 100pt, angle: (i * 7) * 1deg, stroke: 0.4pt); h(-100pt) }',
    'raster': '#image("large.png", width: 100%)',
    'math': '= Mathematical PDF\n#for i in range(40) { $ integral_0^infinity e^(-x^2) dif x = sqrt(pi)/2 $ }',
}
for name, body in cases.items():
    source = out/f'{name}.typ'
    source.write_text('#set page(width: 210mm, height: 297mm)\n'+body)
    subprocess.run(['typst','compile',str(source),str(out/f'{name}.pdf')],check=True)
print(out)
