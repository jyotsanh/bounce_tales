# Builds the recoloured tilesets from assets/art/forest.txt: same tiles in
# the same order (levels and the auto-tiler can swap one for another),
# different palette. Run from the repo root, then `cargo run -p artgen`:
#   python3 tools/levels/recolor.py
import re

PALETTES = {
    'mine': ('rock, gravel and timber', {
        'G': '#a8a4b4', 'g': '#86829a', 'V': '#5e5a72', 'D': '#7a6a5c', 'd': '#5c5048',
        'K': '#43392f', 'O': '#2a221c', 'c': '#3a322c', 'b': '#2c2520', 'B': '#1c1714',
        'h': '#4a3c2c', 'U': '#c8a46a', 'H': '#a07a44', 'u': '#7a5a30'}),
    'machine': ('riveted steel', {
        'G': '#c4ccd8', 'g': '#96a0ae', 'V': '#66707e', 'D': '#7a8290', 'd': '#5c6470',
        'K': '#424854', 'O': '#2a2e36', 'c': '#2e323a', 'b': '#24272e', 'B': '#1a1c22',
        'h': '#3c4048', 'U': '#e0c050', 'H': '#b89a30', 'u': '#806a20'}),
    'circus': ('red carpet and painted boards', {
        'G': '#ff6a6a', 'g': '#e03a3a', 'V': '#9a1c2c', 'D': '#d09a5a', 'd': '#a8743e',
        'K': '#784a22', 'O': '#3e2612', 'c': '#4c2c54', 'b': '#3c2244', 'B': '#2a1832',
        'h': '#5a3a1a', 'U': '#ffd84a', 'H': '#e0a830', 'u': '#a87820'}),
}

src = open('assets/art/forest.txt').read()
head, rest = src.split('\nend\n', 1)
for name, (what, colours) in PALETTES.items():
    h = head.replace('; Forest tileset.', f'; {name.capitalize()} tileset ({what}): generated from forest.txt by\n; tools/levels/recolor.py. Do not edit; edit forest.txt and rerun.\n; Forest tileset notes:', 1)
    h = h.replace('sheet forest cell', f'sheet {name} cell', 1)
    h = re.sub(r'  (\S) (#[0-9a-fA-F]+)', lambda m: f'  {m.group(1)} {colours[m.group(1)]}' if m.group(1) in colours else m.group(0), h)
    open(f'assets/art/{name}.txt', 'w').write(h + '\nend\n' + rest)
    print('wrote', name)
