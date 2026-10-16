# Builds the app icon from the red ball sprite (assets/gen/ball.png,
# `roll_0`): crisp nearest-neighbour PNGs at the usual sizes plus a Windows
# .ico and a macOS .icns. Needs Pillow. Run from the repo root:
#   python3 packaging/icon/make_icons.py
from PIL import Image

idx = dict(
    (l.split()[0], tuple(map(int, l.split()[1:5])))
    for l in open('assets/gen/ball.idx')
    if l.strip()
)
x, y, w, h = idx['roll_0']
sprite = Image.open('assets/gen/ball.png').convert('RGBA').crop((x, y, x + w, y + h))

def icon(size):
    # The 16 px ball scaled by whole steps where possible, centred.
    k = max(1, size // 16)
    ball = sprite.resize((16 * k, 16 * k), Image.NEAREST)
    if ball.size[0] > size:
        ball = sprite.resize((size, size), Image.NEAREST)
    out = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    out.paste(ball, ((size - ball.size[0]) // 2, (size - ball.size[1]) // 2))
    return out

sizes = [16, 32, 48, 64, 128, 256, 512, 1024]
for s in sizes:
    icon(s).save(f'packaging/icon/icon_{s}.png', optimize=True)
icon(256).save('packaging/icon/bounce_tales.ico', sizes=[(s, s) for s in (16, 32, 48, 64, 128, 256)])
icon(1024).save('packaging/icon/bounce_tales.icns')
print('icons written to packaging/icon')
