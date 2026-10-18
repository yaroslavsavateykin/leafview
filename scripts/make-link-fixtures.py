#!/usr/bin/env python3
"""Deterministic annotation fixtures; no external Python dependencies."""
from pathlib import Path
root = Path(__file__).resolve().parents[1] / 'tests/fixtures'
objects = [
 b'<< /Type /Catalog /Pages 2 0 R /Names << /Dests 9 0 R >> /Dests << /legacy [4 0 R /Fit] >> >>',
 b'<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R 6 0 R 7 0 R] /Count 5 /MediaBox [10 20 610 820] >>',
 b'<< /Type /Page /Parent 2 0 R /Contents 8 0 R /Annots [10 0 R 11 0 R 12 0 R 13 0 R 14 0 R 15 0 R 16 0 R 17 0 R] >>',
 b'<< /Type /Page /Parent 2 0 R /Contents 8 0 R >>',
 b'<< /Type /Page /Parent 2 0 R /Contents 8 0 R /Rotate 90 /CropBox [110 120 510 720] /Annots [10 0 R] >>',
 b'<< /Type /Page /Parent 2 0 R /Contents 8 0 R /Rotate 180 /Annots [10 0 R] >>',
 b'<< /Type /Page /Parent 2 0 R /Contents 8 0 R /Rotate 270 /Annots [10 0 R] >>',
 b'<< /Length 28 >>\nstream\n1 0 0 rg 70 700 180 60 re f\n\nendstream',
 b'<< /Kids [18 0 R] >>',
 b'<< /Type /Annot /Subtype /Link /Rect [70 700 250 760] /A << /S /URI /URI (https://example.org/a?x=1&y=2) >> >>',
 b'<< /Type /Annot /Subtype /Link /Rect [70 600 250 660] /Dest [4 0 R /XYZ 110 700 1.25] >>',
 b'<< /Type /Annot /Subtype /Link /Rect [70 500 250 560] /A << /S /GoTo /D (chapter) >> >>',
 b'<< /Type /Annot /Subtype /Link /Rect [70 400 250 460] /Dest /legacy >>',
 b'<< /Type /Annot /Subtype /Link /Rect [70 300 250 360] /A << /S /JavaScript /JS (bad) >> >>',
 b'<< /Type /Annot /Subtype /Link /Rect [70 200 250 260] /A << /S /Launch /F (evil) >> >>',
 b'<< /Type /Annot /Subtype /Link /Rect [70 100 250 160] /A << /S /URI /URI (file:///etc/passwd) >> >>',
 b'<< /Type /Annot /Subtype /Link /Rect [71 71 72 72] /Dest [4 0 R /FitR 110 120 310 320] >>',
 b'<< /Names [(chapter) 19 0 R] >>',
 b'<< /D [4 0 R /FitH 650] >>',
]
data = bytearray(b'%PDF-1.7\n')
offsets=[0]
for i,obj in enumerate(objects,1):
 offsets.append(len(data));data.extend(f'{i} 0 obj\n'.encode()+obj+b'\nendobj\n')
xref=len(data);data.extend(f'xref\n0 {len(objects)+1}\n0000000000 65535 f \n'.encode())
for offset in offsets[1:]: data.extend(f'{offset:010} 00000 n \n'.encode())
data.extend(f'trailer\n<< /Size {len(objects)+1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n'.encode())
(root/'links.pdf').write_bytes(data)
print(root/'links.pdf')
