import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(126, 25)
g.fill(0, 125, 0, 24)
def carve(c0, c1, r0, r1): g.fill(c0, c1, r0, r1, '.')
F = 18  # factory floor row
# A: start on a conveyor carrying you right
carve(2, 20, 12, F - 1); g.put(4, F - 1, 'o')
for c in range(8, 19): g.g[F][c] = '}'
g.gems([(9, 15), (12, 15), (15, 15)])
# B: crusher row (staggered by the finishing script); roll under while up
carve(21, 41, 12, F - 1)
for c in (24, 30, 36): g.put(c, 12, 'P')
g.gems([(27, 16), (33, 16), (39, 16)])
# C: a pit with a fan blowing up to a gantry
carve(42, 47, 12, 21); g.fill(42, 47, 22, 22)
g.put(44, 21, 'F')
carve(48, 58, 6, 15); g.fill(48, 58, 16, 16)
g.gems([(45, 13), (43, 16), (51, 14), (55, 14)])
# D: back down; a conveyor running against you to a hypno machine
carve(59, 82, 10, F - 1)
g.fill(59, 61, 16, 17)                     # steps down from the gantry
for c in range(64, 77): g.g[F][c] = '{'
g.put(79, F - 1, 'h')
g.put(79, 11, 'e')                       # the machine's bounce reaches it
g.gems([(67, 15), (71, 15), (75, 15)])
# E: two crushers over a conveyor helping you on, then the exit
carve(83, 121, 12, F - 1)
g.put(85, F - 1, 'C'); g.put(87, F - 2, '+')   # a spare ball before the last crushers
for c in range(90, 108): g.g[F][c] = '}'
for c in (93, 101): g.put(c, 12, 'P')
g.gems([(97, 16), (105, 16), (112, 15)])
g.put(117, F - 1, 'E')
g.save('tools/levels/ascii/ch09.txt')
