import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(124, 28)
g.walls()
F = 18
# start; the red switch sits at the top of a climb
g.ground(2, 25, F); g.put(4, F - 1, 'o')
g.fill(10, 13, 16, 16); g.fill(15, 18, 14, 14); g.fill(20, 23, 12, 12)
g.put(22, 11, 'a')
g.fill(16, 18, 9, 9); g.put(17, 8, 'e')         # nook above the climb
g.gems([(11, 14), (16, 12), (21, 10), (7, 16)])
# the red door
g.fill(26, 26, 0, 13)
for r in range(14, F): g.put(26, r, 'A')
g.ground(26, 31, F)
# a hole down to the flooded lower cave with the blue switch
g.ground(32, 34, 25)
g.ground(35, 46, 25); g.fill(35, 46, 22, 24, '~')
g.ground(47, 49, 24); g.put(48, 23, 'u')
g.ground(50, 51, 22); g.ground(52, 53, 20)          # steps back up
g.ground(54, 60, F)
g.fill(35, 49, F, F)                                 # floor over the cave; the hole is 32..34
g.gems([(38, 21), (42, 21), (46, 21), (50, 20), (52, 18)])
# the blue door
g.fill(58, 58, 0, 13)
for r in range(14, F): g.put(58, r, 'U')
g.put(60, F - 1, 'C')
# a moving platform over a pit to the green switch's island
g.put(61, F, 'H')
g.ground(72, 90, F); g.put(74, F - 1, 'n')
g.gems([(64, 15), (67, 15), (70, 15)])
g.put(106, F - 1, 'w')               # past the green door, away from the platform's landing
# the green door, a flyer and the exit
g.fill(86, 86, 0, 13)
for r in range(14, F): g.put(86, r, 'N')
g.ground(91, 121, F)
g.put(96, 12, 'y')
g.gems([(89, 16), (95, 15), (102, 16), (108, 16), (113, 16)])
g.put(116, F - 1, 'E')
g.save('tools/levels/ascii/ch12.txt')
