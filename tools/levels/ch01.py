W,H=87,20
OFF=3
g=[['.']*W for _ in range(H)]
def fill(c0,c1,r0,r1,ch='#'):
    r0 = 0 if r0 == 0 else r0+OFF
    r1 = r1 if r1 == H-1 else r1+OFF
    for r in range(r0,r1+1):
        for c in range(c0,c1+1): g[r][c]=ch
def ground(c0,c1,top):  # solid from top row down to the bottom
    fill(c0,c1,top,H-1)
def put(c,r,ch): g[r+OFF][c]=ch
# walls
fill(0,2,0,H-1); fill(86,86,0,H-1)
# tower
fill(11,11,0,6)                 # inner wall
fill(8,10,3,3)                  # ledge A
fill(3,7,5,5)                   # ledge B
fill(7,11,7,7)                  # ledge C
put(9,2,'o')
ground(3,11,12)                 # tower floor
ground(12,14,10)                # block
put(15,10,'L'); put(16,10,'l'); ground(15,16,11)
ground(17,18,11)
# dip (the broken bridge)
put(19,11,'L'); put(20,11,'l'); ground(19,20,12)
put(21,12,'L'); put(22,12,'l'); ground(21,22,13)
ground(23,23,13)
put(24,12,'/'); ground(24,24,13)
ground(25,26,12)
put(27,11,'/'); ground(27,27,12)
ground(28,45,11)
# pillar + switch ledge + stepping ledge
fill(25,25,0,6); fill(25,29,7,7); put(27,6,'S')
fill(30,32,9,9)
# ceiling over the barrier, and the barrier
fill(35,35,0,1); fill(36,36,0,2); fill(37,37,0,3); fill(38,42,0,5)
for r in range(6,11): put(40,r,'D')
put(44,10,'C')
# down to the meadow
put(46,11,'L'); put(47,11,'l'); ground(46,47,12)
put(48,12,'L'); put(49,12,'l'); ground(48,49,13)
ground(50,52,13)
put(53,12,'/'); ground(53,53,13)
ground(54,69,12)
put(63,11,'T'); put(63,6,'e'); put(66,11,'h')
put(70,12,'L'); put(71,12,'l'); ground(70,71,13)
ground(72,80,13)
put(81,12,'/'); ground(81,81,13)
ground(82,85,12)
put(84,11,'E')
# gems (the reference's yellow eggs)
for c,r in [(4,4),(4,11),(10,11),(12,8),(14,8),(17,10),(21,11),(22,8),(23,7),(24,8),
            (28,10),(28,8),(31,8),(28,6),(42,10),(48,11),(52,12),(56,11),(59,10),(61,9),
            (68,10),(70,11),(74,12),(80,12)]:
    if g[r+OFF][c]=='.': put(c,r,'*')
    else: print('gem blocked',c,r)
open("tools/levels/ascii/ch01.txt","w").write("\n".join("".join(r) for r in g)+"\n")
print("\n".join("".join(r) for r in g))
