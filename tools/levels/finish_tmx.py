# usage: finish_tmx.py ascii.txt in.tmx out.tmx title music par seed
import sys, re
asc, tin, tout, title, music, par, seed = sys.argv[1:8]
cave = len(sys.argv) > 8 and sys.argv[8] == 'cave'
rows=[l.rstrip('\n') for l in open(asc) if l.strip()]
H=len(rows); W=len(rows[0]); seed=int(seed)
s=open(tin).read()
nid=int(re.search(r'nextobjectid="(\d+)"',s).group(1))
# deco gids (firstgid 101: room for forest.tsx to grow): flower_red flower_yellow mushroom tuft rock sign_right bush_l bush_r
FR,FY,MU,TU,RO,SI,BL,BR=range(101,109)
def rnd(i):
    x=(i*2654435761 + seed*97) & 0xffffffff
    x^=x>>13; x=(x*1274126177)&0xffffffff; x^=x>>16
    return x
objs=[]; c=3
while c < W-3:
    # flat ground top at this column, free above
    top=next((r for r in range(1,H) if rows[r][c]=='#' and rows[r-1][c]=='.'), None)
    if top is not None and all(rows[top-1][c+k]=='.' and rows[top][c+k]=='#' for k in (0,1)):
        k=rnd(c)%10
        y=top*16
        if k<3: objs.append((FR if k<2 else FY, c*16, y))
        elif k==3: objs.append((TU, c*16, y))
        elif k==4: objs.append((RO, c*16, y))
        elif k==5: objs+= [(BL,c*16,y),(BR,c*16+16,y)]; c+=1
        elif k==6: objs.append((MU, c*16, y))
        c+= 3 + rnd(c+1000)%4
    else:
        c+=1
xml="".join(f'  <object id="{nid+i}" gid="{g}" x="{x}" y="{y}" width="16" height="16"/>\n' for i,(g,x,y) in enumerate(objs))
s=s.replace('nextobjectid="%d"'%nid, 'nextobjectid="%d"'%(nid+len(objs)))
s=s.replace(' <tileset firstgid="1" source="../gen/forest.tsx"/>\n', ' <properties>\n  <property name="title" value="%s"/>\n  <property name="music" value="%s"/>\n  <property name="par" type="int" value="%s"/>\n </properties>\n <tileset firstgid="1" source="../gen/forest.tsx"/>\n <tileset firstgid="101" source="../gen/deco.tsx"/>\n'%(title,music,par),1)
s=s.replace(' </objectgroup>\n</map>', xml+' </objectgroup>\n</map>')
if cave:
    CAVE_BACK = 21  # forest.tsx tile 20
    solid = lambda r, c: 0 <= r < H and 0 <= c < W and rows[r][c] in '#/\\rRLlkx^v'
    back = []
    for r in range(H):
        for c in range(W):
            enclosed = (not solid(r, c)
                and any(solid(k, c) for k in range(r))
                and any(solid(k, c) for k in range(r + 1, H))
                and any(solid(r, k) for k in range(max(0, c - 30), c))
                and any(solid(r, k) for k in range(c + 1, min(W, c + 31))))
            back.append(CAVE_BACK if enclosed else 0)
    csv = ",\n".join(",".join(str(v) for v in back[r * W:(r + 1) * W]) for r in range(H))
    layer = f' <layer id="9" name="back" width="{W}" height="{H}">\n  <data encoding="csv">\n{csv}\n</data>\n </layer>\n'
    s = s.replace(' <layer id="1" name="ground"', layer + ' <layer id="1" name="ground"', 1)
open(tout,'w').write(s)
print(len(objs),'decorations')
