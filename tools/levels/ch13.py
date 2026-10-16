import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(150, 27)
g.fill(0, 149, 0, 26)
def carve(c0, c1, r0, r1): g.fill(c0, c1, r0, r1, '.')
F = 18
# A: crushers (plain floor: a belt would carry you into them while you
# wait), then a short conveyor helping you on
carve(2, 34, 12, F - 1); g.put(4, F - 1, 'o')
for c in range(30, 35): g.g[F][c] = '}'
for c in (14, 21, 28): g.put(c, 12, 'P')
g.gems([(8, 16), (18, 16), (25, 16), (32, 16)])
# B: spike patches (2 wide), then crumbling planks over spikes
carve(35, 64, 12, F - 1)
for c in (39, 40, 46, 47): g.g[F - 1][c] = '^'
carve(52, 59, F, 22)
for c in range(52, 60): g.put(c, F, 'x')
for c in range(52, 60): g.g[22][c] = '^'
g.gems([(39, 14), (46, 14), (55, 15), (62, 16)])
g.put(63, F - 1, 'C')
# C: moving platform over a spiked chasm
carve(65, 80, 12, F - 1)
carve(67, 77, F, 23)
for c in range(67, 78): g.g[23][c] = '^'
g.put(67, F, 'H')
g.gems([(70, 15), (73, 15), (76, 15)])
# D: zig-zag climb (4-wide ledges, 1-tile gaps) up to the upper hall
carve(81, 89, 4, F - 1)
g.fill(86, 89, 16, 16); g.fill(81, 84, 14, 14); g.fill(86, 89, 12, 12)
g.fill(81, 84, 10, 10); g.fill(86, 89, 8, 8)
g.gems([(87, 14), (82, 12), (87, 10), (82, 8)])
# E: upper hall: shooter, hypno machine (egg above), extra life, exit
carve(90, 146, 4, 7)
g.ground(90, 146, 8)
g.put(95, 7, 'C')
g.put(110, 7, 'G')                 # out of range of the climb's top ledge
g.put(118, 7, 'h'); g.put(118, 4, 'e')
g.put(124, 7, '+')
g.put(132, 4, 'y')
g.gems([(100, 6), (106, 6), (114, 6), (128, 6), (136, 6), (139, 6)])
g.put(142, 7, 'E')
g.save('tools/levels/ascii/ch13.txt')
