import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(122, 27)
g.fill(0, 121, 0, 26)                      # solid rock; carve the mine out
def carve(c0, c1, r0, r1): g.fill(c0, c1, r0, r1, '.')
# A: entrance gallery (floor row 10)
carve(2, 22, 4, 9); g.put(4, 9, 'o'); g.put(14, 9, 'w')
g.gems([(8, 7), (11, 7), (17, 7), (20, 7)])
# B: shaft down to the lower tunnel
carve(21, 26, 4, 21)
g.gems([(23, 12), (24, 15), (23, 18)])
# C: lower tunnel (floor row 22, ceiling row 17)
carve(27, 63, 18, 21)
# crumbling planks over a spike pit
carve(34, 41, 22, 25)
for c in range(34, 42): g.put(c, 22, 'x')
for c in range(34, 42): g.g[25][c] = '^'
g.gems([(36, 19), (39, 19)])
g.put(44, 21, 'w')
# moving platform over a spiked chasm
carve(47, 57, 22, 25)
for c in range(47, 58): g.g[25][c] = '^'
g.put(47, 22, 'H')
g.gems([(49, 19), (52, 19), (55, 19)])
g.put(60, 21, 'C')
# D: zig-zag shaft up to the upper gallery (ledges every 2 rows)
carve(64, 72, 9, 21)
g.fill(69, 72, 20, 20); g.fill(64, 67, 18, 18); g.fill(69, 72, 16, 16)
g.fill(64, 67, 14, 14); g.fill(69, 72, 12, 12)
g.gems([(70, 19), (65, 17), (70, 15), (65, 13)])
# E: upper gallery (floor row 13) to the exit, with bats and a shooter
carve(73, 117, 8, 12)
g.put(80, 9, 'y')

g.gems([(77, 11), (84, 10), (92, 10), (100, 9), (104, 10)])
g.fill(106, 106, 12, 12)            # a step before the shooter
g.put(100, 12, 'G')
g.put(114, 12, 'E')
# the egg: a dead-end alcove above the shaft top, reached from the top ledge
carve(60, 69, 6, 9); g.put(61, 9, 'e'); g.gems([(64, 8), (67, 8)])
g.fill(65, 66, 10, 10)               # stepping ledge towards the alcove
g.save('tools/levels/ascii/ch06.txt')
