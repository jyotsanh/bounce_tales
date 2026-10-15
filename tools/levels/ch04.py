import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(116, 24); g.walls()
F = 14  # upper floor top row
g.fill(2, 113, F, 23)                        # solid mass; carve below
# start; the stone transformer; cracked floor
g.put(4, F - 1, 'o'); g.put(9, F - 1, '1')
g.fill(16, 18, F, F, 'k')
g.gems([(7, F - 3), (13, F - 3), (16, F - 3), (17, F - 4), (18, F - 3)])
# a wall closes the upper path: the way on is under it
g.fill(41, 41, 0, F - 1)
# tunnel rows 15..19 from col 16 to 44, floor row 20
g.fill(16, 18, 15, 15, '.')
g.fill(16, 44, 16, 19, '.')
g.gems([(21, 19), (24, 19), (27, 19)])
# pool in the tunnel (rows 16..21), sloped way out on the right
g.fill(29, 36, 16, 21, '~'); g.fill(29, 36, 20, 21, '~')
g.g[22][33] = 'e'                            # dip in the pool floor
g.g[20][37] = '.'; g.g[21][37] = '/'; g.g[20][38] = '/'
g.put(40, 19, '0')                           # back to normal before the climb
g.gems([(42, 18)])
# climb out: steps to the surface
g.fill(45, 50, 0, 17, '.')
g.fill(45, 47, 18, 18); g.fill(48, 50, 16, 17)
g.gems([(46, 16), (49, 14)])
g.fill(42, 44, 0, F - 1, '.')
# surface: crumbling bridge over a pit
g.fill(57, 65, F, 23, '.')
for c in range(57, 66): g.put(c, F, 'x')
g.gems([(59, F - 2), (62, F - 2), (54, F - 2)])
g.put(68, F - 1, 'C')
# stone again: roll hard into the cracked wall, then normal
g.put(71, F - 1, '1')
g.fill(77, 77, 0, F - 5); g.fill(77, 77, F - 4, F - 1, 'k')
g.put(82, F - 1, '0')
g.gems([(80, F - 2), (85, F - 2), (74, F - 2)])
# bumpy finish: humps with a walker between them, then the exit
for c0 in (93, 100):
    g.put(c0, F - 1, 'r'); g.put(c0 + 1, F - 1, 'R')
    g.put(c0 + 2, F - 1, 'L'); g.put(c0 + 3, F - 1, 'l')
g.put(89, F - 1, 'w')
g.put(110, F - 1, 'E')
g.gems([(24, F - 3), (35, F - 3), (90, F - 3), (95, F - 4), (102, F - 4), (106, F - 2)])
g.save('tools/levels/ascii/ch04.txt')
