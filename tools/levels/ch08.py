import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(124, 22)
g.fill(0, 123, 0, 21)
def carve(c0, c1, r0, r1): g.fill(c0, c1, r0, r1, '.')
F = 16   # main floor row
# A: start and a first lone stalactite to learn from
carve(2, 20, 10, F - 1); g.put(4, F - 1, 'o')
g.put(14, 10, 'I')
g.gems([(8, 13), (11, 13), (17, 13)])
# B: the rumbling corridor: a row of stalactites; keep rolling
carve(21, 46, 11, F - 1)
for c in range(24, 45, 3): g.put(c, 11, 'I')
g.gems([(25, 14), (31, 14), (37, 14), (43, 14)])
# C: crumbling floor over a pit, stalactites over it too
carve(47, 58, 10, F - 1)
carve(49, 56, F, 20)
for c in range(49, 57): g.put(c, F, 'x')
for c in range(49, 57): g.g[20][c] = '^'
g.put(50, 10, 'I'); g.put(55, 10, 'I')
g.gems([(52, 13), (54, 13)])
carve(59, 64, 10, F - 1); g.put(60, F - 1, 'C')
# D: seesaw cavern; a spring up to the ledge with the egg (optional)
carve(65, 86, 5, F - 1)
g.put(67, F - 1, 'W'); g.put(76, F - 1, 'j')     # spring up to the egg ledge
g.fill(78, 86, 10, 10); g.put(84, 9, 'e'); g.gems([(80, 9), (82, 9)])
g.put(74, 5, 'I')
g.gems([(68, 13), (75, 12), (77, 14)])
# E: steps down past a shooter, then a last rumbling run to the exit
carve(87, 120, 11, F + 1); g.fill(87, 120, F + 2, F + 2)
carve(87, 90, 7, 10)              # the egg ledge drops into this passage
g.fill(87, 92, F, F + 1); g.fill(93, 97, F + 1, F + 1)
g.put(100, F + 1, 'G')        # shooter facing you as you come down the steps
for c in range(104, 116, 3): g.put(c, 11, 'I')
g.gems([(90, 14), (95, 15), (106, 15), (112, 15)])
g.put(118, F + 1, 'E')
g.save('tools/levels/ascii/ch08.txt')
