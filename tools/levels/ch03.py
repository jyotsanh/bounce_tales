import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(112, 22); g.walls()
# start meadow
g.ground(2, 12, 17); g.put(4, 16, 'o')
# pool 1: swim across (gems just over the surface)
g.ground(13, 22, 20); g.fill(13, 22, 17, 19, '~')
g.gems([(15, 15), (18, 15), (21, 15)])
g.ground(23, 28, 17); g.put(10, 16, 'w')
# bottomless pit crossed on a moving platform
g.put(29, 17, 'H')
g.gems([(31, 13), (34, 13), (37, 13), (40, 13)])
g.ground(43, 77, 17)
g.put(45, 16, 'C')
# lift up to the switch ledge; the egg nook above the lift's top
g.put(48, 16, 'V')
g.fill(51, 57, 12, 12); g.put(54, 11, 'S'); g.gems([(52, 11), (56, 11)])
g.fill(43, 46, 10, 10); g.fill(43, 43, 7, 9); g.put(44, 9, 'e')
# the door in the floor path, wall above it
g.fill(61, 61, 0, 12)
for r in range(13, 17): g.put(61, r, 'D')
g.gems([(59, 15), (63, 15)])
# crate to push against the ledge and climb
g.put(65, 15, 'X')
g.ground(72, 80, 14)
g.gems([(67, 12), (74, 12), (78, 12)])
# pool 2 under a gliding flyer, then the exit bank
g.ground(81, 92, 19); g.fill(81, 92, 15, 18, '~')
g.put(84, 11, 'y')
g.gems([(83, 13), (86, 13), (89, 13), (92, 13)])
g.ground(93, 109, 16)
g.put(105, 15, 'E')
g.gems([(8, 13), (11, 13), (25, 14), (50, 15), (70, 15), (96, 14), (100, 14)])
g.save('tools/levels/ascii/ch03.txt')
