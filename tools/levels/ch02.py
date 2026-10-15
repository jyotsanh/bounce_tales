W,H=112,22
g=[['.']*W for _ in range(H)]
def fill(c0,c1,r0,r1,ch='#'):
    for r in range(r0,r1+1):
        for c in range(c0,c1+1): g[r][c]=ch
def ground(c0,c1,top): fill(c0,c1,top,H-1)
def put(c,r,ch):
    assert g[r][c]=='.', (c,r,g[r][c]); g[r][c]=ch
fill(0,1,0,H-1); fill(W-2,W-1,0,H-1)
# 1. meadow start, a walker to stomp (gems above it show the way)
ground(2,19,17); put(4,16,'o')
put(13,16,'w')
for c in (11,13,15): put(c,13,'*')
# 2. pit with a flyer gliding over it, then a hill
ground(24,27,17)              # far bank
put(21,12,'y')               # flyer glides right over the pit
for c in (20,22,25,27): put(c,14,'*')
ground(28,31,17)
put(32,16,'r'); put(33,16,'R'); ground(32,33,17)
put(34,15,'r'); put(35,15,'R'); ground(34,35,16)
ground(36,40,15)
# 3. two-level stretch: lower path with a jumper, upper ledge with gems
ground(41,58,17)              # lower floor (drop down from the hill)
fill(43,50,13,13)             # upper ledge
put(47,16,'J')
for c in (45,47,49): put(c,12,'*')
# mole field
put(52,16,'T'); put(55,16,'T')
put(53,13,'*'); put(56,13,'*')
put(59,16,'C'); ground(59,62,17)
# 4. valley with walkers; a spring up to a ledge with the blue egg and a machine
put(63,17,'L'); put(64,17,'l'); ground(63,64,18)
ground(65,76,18)
put(69,17,'w')
put(77,17,'r'); put(78,17,'R'); ground(77,78,18)
ground(79,84,17)
put(80,16,'j')                # spring
fill(82,88,11,11)             # high ledge
put(86,10,'h'); put(86,5,'e')
for c in (83,84): put(c,10,'*')
# 5. spikes between steps, a shooter at the top, the exit
ground(85,86,17)
for c in (87,88,89): put(c,17,'^')
ground(87,89,18)
ground(90,93,16)
ground(94,97,15)
put(97,14,'G')                # shooter facing left down the steps
ground(98,109,14)
put(105,13,'E')
for c,r in [(5,15),(8,15),(17,15),(29,15),(38,12),(42,15),(61,15),(67,14),(72,14),(75,16),(91,14),(95,12),(101,12)]:
    put(c,r,'*')
open("tools/levels/ascii/ch02.txt","w").write("\n".join("".join(r) for r in g)+"\n")
print("\n".join("".join(r) for r in g))
