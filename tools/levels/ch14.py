import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(76, 24)
g.walls()
B = 14   # bank / island row (solid top)
# start ledge
g.ground(2, 7, B); g.put(4, B - 1, 'o')
# the ride: three moving platforms over a long spiked chasm
g.ground(8, 52, 21)
for c in range(8, 53): g.g[20][c] = '^'
g.put(8, B, 'H'); g.ground(19, 21, B)
g.put(22, B, 'H'); g.ground(33, 35, B)
g.put(36, B, 'H'); g.ground(47, 52, B)
g.gems([(11, 11), (14, 11), (17, 11), (25, 11), (28, 11), (31, 11), (39, 11), (42, 11), (45, 11)])
g.put(27, 9, 'e')                       # jump for it from the second platform
g.gems([(20, 12), (34, 12), (48, 12), (51, 12), (55, 16), (61, 16), (67, 16)])
g.put(50, B - 1, 'C')
# down into the Hypno Cube's arena through a low opening
g.ground(53, 57, 18)
g.fill(57, 57, 0, 14)                    # arena left wall above the opening
g.fill(57, 75, 0, 1)                     # arena ceiling
g.fill(71, 75, 2, 23)                    # right wall
g.ground(57, 70, 18)
g.put(64, 5, 'M')
g.save('tools/levels/ascii/ch14.txt')
