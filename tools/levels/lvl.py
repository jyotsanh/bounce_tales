class Grid:
    def __init__(s, w, h):
        s.w, s.h = w, h
        s.g = [['.'] * w for _ in range(h)]
    def fill(s, c0, c1, r0, r1, ch='#'):
        for r in range(r0, r1 + 1):
            for c in range(c0, c1 + 1):
                s.g[r][c] = ch
    def ground(s, c0, c1, top):
        s.fill(c0, c1, top, s.h - 1)
    def put(s, c, r, ch):
        assert s.g[r][c] == '.', (c, r, s.g[r][c], ch)
        s.g[r][c] = ch
    def gems(s, cells):
        for c, r in cells:
            s.put(c, r, '*')
    def walls(s):
        s.fill(0, 1, 0, s.h - 1)
        s.fill(s.w - 2, s.w - 1, 0, s.h - 1)
    def save(s, path):
        text = "\n".join("".join(r) for r in s.g) + "\n"
        open(path, "w").write(text)
        print(text)
