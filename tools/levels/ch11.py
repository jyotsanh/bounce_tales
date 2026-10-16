import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(132, 22)
g.walls()
F = 16
# A: big top entrance with a clown (jumper)
g.ground(2, 16, F); g.put(4, F - 1, 'o'); g.put(11, F - 1, 'J')
g.gems([(7, 13), (14, 12)])
# B: trampoline up to a plateau over a spiked pit
g.ground(17, 19, F); g.put(18, F - 1, 'j')
g.fill(20, 31, 12, 12); g.ground(26, 31, 12)
g.ground(20, 25, F + 3)
for c in range(20, 26): g.g[F + 2][c] = '^'
g.gems([(21, 10), (23, 10)])
# C: catapult over a bottomless pit (as in "launchers": 3 tiles before a
# 6-wide pit, landing at the same height)
g.put(28, 11, '>')
g.ground(38, 41, 12)
g.gems([(33, 8), (36, 8)])
# D: big wheel over a pit (as in "wheels": 2 rows above the floor, pit 9
# wide from 4 tiles left of it); the egg above its top car
g.ground(42, 43, F)
g.put(48, F - 2, 'O')
g.put(48, 8, 'e')
g.ground(53, 60, F)
g.put(56, F - 1, 'C')
# E: spinner set in a pit
g.put(63, F, 'Z')
g.ground(65, 72, F)
g.gems([(62, 11), (64, 11)])
# F: balloon over the tall wall (spiked ledge on the way), normal on top
g.put(68, F - 1, '2')
g.fill(76, 79, 3, F); g.fill(72, 74, 6, 6); g.fill(72, 74, 7, 7, 'v')
g.put(78, 2, '0'); g.ground(73, 79, F)
g.ground(80, 98, F)
g.gems([(84, 13), (89, 13), (94, 13), (70, 6)])
g.gems([(69, 10), (75, 4), (77, 1)])
# G: a cannon fires you up to the exit platform (as in "wheels": 6 rows up,
# 5 tiles on)
g.ground(99, 104, F); g.put(100, F - 1, 'K')
g.ground(105, 129, 10)
g.gems([(103, 7), (110, 8), (116, 8)])
g.put(122, 9, 'E')
g.save('tools/levels/ascii/ch11.txt')
