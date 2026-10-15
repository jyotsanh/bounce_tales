import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(38, 64)
g.fill(0, 3, 0, 63); g.fill(34, 37, 0, 63)      # thick side walls
g.fill(0, 37, 0, 2)                             # roof
g.ground(4, 33, 61)
g.put(6, 60, 'o')
g.gems([(11, 57), (18, 55), (25, 53), (8, 59)])
# climbing ledges (normal ball)
g.fill(9, 13, 59, 59); g.fill(16, 20, 57, 57); g.fill(23, 27, 55, 55); g.fill(28, 33, 53, 53)
g.put(31, 52, '2')                              # balloon transformer
# the stalkway shaft: float up, steering round spiked ledges
g.fill(22, 33, 44, 44); g.fill(22, 33, 45, 45, 'v')
g.gems([(8, 47), (8, 42), (12, 38)])
g.fill(4, 17, 34, 34); g.fill(4, 17, 35, 35, 'v')
g.gems([(26, 32), (26, 27)])
g.fill(19, 33, 25, 25); g.fill(19, 33, 26, 26, 'v')
# secret alcove in the left wall with the egg
g.fill(1, 3, 39, 41, '.'); g.put(2, 41, 'e')
g.fill(1, 3, 42, 42)
# top landing (from above): back to normal
g.fill(4, 11, 20, 20)
g.put(5, 19, '0')
g.gems([(9, 17), (14, 12), (8, 14)])
# the stalk: small leaves zig-zagging up to the exit
g.fill(13, 15, 18, 18); g.fill(18, 20, 16, 16); g.fill(23, 25, 14, 14)
g.fill(28, 33, 12, 12)
g.put(31, 11, 'E')
g.gems([(14, 16), (19, 14), (24, 12)])
g.save('tools/levels/ascii/ch05.txt')
