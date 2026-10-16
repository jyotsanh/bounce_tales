# usage: finish_tmx.py ascii.txt in.tmx out.tmx title music par seed [flags...]
# flags: cave (back layer for enclosed spaces), a theme: mine / machine /
# circus (that tileset, its decorations and backdrop), bgcave (cave
# backdrop), dark (only the ball's surroundings lit)
import sys, re
asc, tin, tout, title, music, par, seed = sys.argv[1:8]
flags = set(sys.argv[8:])
cave = 'cave' in flags
# theme -> (tileset, deco tileset, backdrop, deco gid picks by roll 0..9)
THEMES = {
    'mine': ('mine', 'minedeco', 'cave', {0: 102, 1: 103, 2: 106, 3: 106, 4: 108, 5: 107, 6: 109}),
    'machine': ('machine', 'machdeco', 'factory', {0: 101, 1: 102, 2: 103, 3: 104, 4: 105, 5: 106, 6: 107}),
    'circus': ('circus', 'circusdeco', 'circus', {0: 101, 1: 102, 2: 103, 3: 104, 4: 105, 6: 107}),
}
theme = next((THEMES[t] for t in THEMES if t in flags), None)
mine = 'mine' in flags
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
        if theme:
            pick = theme[3].get(k)
            if pick: objs.append((pick, c*16, y))
        elif k<3: objs.append((FR if k<2 else FY, c*16, y))
        elif k==3: objs.append((TU, c*16, y))
        elif k==4: objs.append((RO, c*16, y))
        elif k==5: objs+= [(BL,c*16,y),(BR,c*16+16,y)]; c+=1
        elif k==6: objs.append((MU, c*16, y))
        c+= 3 + rnd(c+1000)%4
    else:
        c+=1
if mine:
    # Lanterns hanging under ceilings now and then.
    for c in range(3, W-3, 1):
        for r in range(1, H-2):
            if rows[r-1][c]=='#' and rows[r][c]=='.' and rows[r+1][c]=='.' and rnd(c*31+r)%9==0:
                objs.append((101, c*16, (r+1)*16))
# Crushers can't carry properties in ASCII: stagger them by order.
k = 0
def stagger(m):
    global k
    d = (k * 0.5) % 2.0
    k += 1
    return m.group(0).rstrip('/>').rstrip() + f'>\n   <properties>\n    <property name="delay" type="float" value="{d}"/>\n   </properties>\n  </object>'
s = re.sub(r'<object id="\d+" name="crusher" type="crusher"[^>]*/>', stagger, s)
xml="".join(f'  <object id="{nid+i}" gid="{g}" x="{x}" y="{y}" width="16" height="16"/>\n' for i,(g,x,y) in enumerate(objs))
s=s.replace('nextobjectid="%d"'%nid, 'nextobjectid="%d"'%(nid+len(objs)))
s=s.replace(' <tileset firstgid="1" source="../gen/forest.tsx"/>\n', ' <properties>\n  <property name="title" value="%s"/>\n  <property name="music" value="%s"/>\n  <property name="par" type="int" value="%s"/>\n </properties>\n <tileset firstgid="1" source="../gen/forest.tsx"/>\n <tileset firstgid="101" source="../gen/deco.tsx"/>\n'%(title,music,par),1)
extra_props = ''
backdrop = theme[2] if theme else ('cave' if 'bgcave' in flags else None)
if backdrop: extra_props += f'  <property name="background" value="{backdrop}"/>\n'
if 'dark' in flags: extra_props += '  <property name="dark" type="bool" value="true"/>\n'
s=s.replace(' </properties>\n', extra_props + ' </properties>\n', 1)
if theme:
    s=s.replace('source="../gen/forest.tsx"',f'source="../gen/{theme[0]}.tsx"').replace('source="../gen/deco.tsx"',f'source="../gen/{theme[1]}.tsx"')
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
