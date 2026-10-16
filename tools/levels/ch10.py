import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(132, 24)
g.fill(0, 131, 0, 23)
def carve(c0, c1, r0, r1): g.fill(c0, c1, r0, r1, '.')
F = 16
# A: start and a trail of gems
carve(2, 18, 10, F - 1); g.put(4, F - 1, 'o')
g.gems([(7, 14), (9, 13), (11, 12), (13, 12), (15, 13), (17, 14)])
# B: flooded corridor (swim; gems over the surface)
carve(19, 40, 11, F + 2); g.fill(19, 40, F + 3, F + 3)
g.fill(19, 40, F, F + 2, '~')
g.gems([(22, F - 1), (26, F - 1), (30, F - 1), (34, F - 1), (38, F - 1)])
# C: a crate to push under the switch ledge; the door further on
carve(41, 66, 8, F - 1)
g.put(45, F - 2, 'X')
g.put(45, 9, 'e')                          # above the crate: hop on it, then jump
g.fill(52, 56, 12, 13); g.put(54, 11, 'S')     # switch ledge (crate + jump)
g.gems([(53, 10), (55, 10), (48, 13)])
g.fill(62, 62, 8, 11)
for r in range(12, F): g.put(62, r, 'D')
g.put(65, F - 1, 'C')
# D: stone to ram the cracked wall into the treasure room, normal again
carve(67, 96, 10, F - 1)
g.put(69, F - 1, '1')
g.fill(76, 76, 10, F - 1, 'k')
g.gems([(78, 12), (80, 11), (82, 12), (84, 11), (86, 12), (80, 14), (84, 14)])
g.put(86, 14, '+')
g.put(79, F - 1, '0')                       # normal again just inside the room
# E: mole tunnel to the exit
carve(97, 128, 11, F - 1)
g.put(103, F - 1, 'T'); g.put(110, F - 1, 'T'); g.put(117, F - 1, 'w')
g.gems([(100, 13), (106, 13), (113, 13), (120, 13)])
g.put(125, F - 1, 'E')
g.save('tools/levels/ascii/ch10.txt')
