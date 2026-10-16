import sys; sys.path.insert(0, 'tools/levels')
from lvl import Grid
g = Grid(116, 24)
g.fill(0, 115, 0, 23)
def carve(c0, c1, r0, r1): g.fill(c0, c1, r0, r1, '.')
# A: start cavern (floor row 17)
carve(2, 16, 11, 16); g.put(4, 16, 'o')
g.gems([(7, 14), (10, 14), (13, 14)])
# B: spike patches on the floor (gems mark the jumps)
carve(17, 40, 12, 16)
for c in (20, 21, 28, 29, 36, 37): g.g[16][c] = '^'
g.gems([(20, 13), (21, 12), (29, 12), (36, 12), (37, 13)])
# C: step down into a pool, swim under a low ceiling with spikes
carve(41, 44, 12, 18); g.fill(41, 44, 19, 19)      # landing step
carve(45, 58, 16, 20); g.fill(45, 58, 18, 20, '~'); g.fill(45, 58, 21, 21)
for c in range(48, 56): g.g[15][c] = '#'; g.g[16][c] = 'v'
g.gems([(47, 17), (51, 17), (55, 17)])
carve(59, 64, 13, 17); g.fill(59, 64, 18, 18)      # out of the pool
g.put(62, 17, 'C'); g.put(60, 16, '+')       # a spare ball in the dark
# D: mole run (two moles, a walker)
carve(65, 84, 12, 16)
g.put(69, 16, 'T'); g.put(76, 16, 'T'); g.put(82, 16, 'w')
g.gems([(67, 13), (72, 13), (79, 13)])
# E: climb out: ledges up a dark chamber
carve(85, 100, 4, 16)
g.fill(87, 90, 15, 15); g.fill(92, 95, 13, 13); g.fill(87, 90, 11, 11); g.fill(92, 95, 9, 9)
g.fill(97, 100, 7, 7)
g.gems([(89, 13), (94, 11), (89, 9), (94, 7)])
g.put(85, 10, 'e'); g.fill(85, 85, 11, 11)   # dark corner ledge: roll left off the ledge at 87..90
# F: final passage to the exit
carve(101, 112, 3, 5)

g.put(110, 5, 'E')
g.gems([(103, 4), (107, 4)])
g.save('tools/levels/ascii/ch07.txt')
