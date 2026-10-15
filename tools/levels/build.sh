#!/bin/sh
# Rebuilds the chapter levels from their generator scripts:
#   tools/levels/build.sh            # all chapters
#   tools/levels/build.sh ch03       # one chapter
# Each chNN.py writes tools/levels/ascii/chNN.txt (the collision map with
# object markers, see level.rs), then ascii2tmx turns it into Tiled XML and
# finish_tmx.py adds the map properties, decorations and (for caves) the
# back-wall layer, writing assets/levels/chNN.tmx. Hand edits made in Tiled
# are overwritten, so change the script instead (or stop using it for that
# level).
set -e
cd "$(dirname "$0")/../.."
OUT=$(mktemp -d)
build() { # name title music par seed [cave]
    python3 tools/levels/$1.py > /dev/null
    cargo run -q --bin ascii2tmx -- tools/levels/ascii/$1.txt "$OUT/$1.tmx" > /dev/null
    python3 tools/levels/finish_tmx.py tools/levels/ascii/$1.txt "$OUT/$1.tmx" assets/levels/$1.tmx "$2" "$3" "$4" "$5" "$6"
}
want() { [ -z "$ONLY" ] || [ "$ONLY" = "$1" ]; }
ONLY=$1
want ch01 && build ch01 "Misty Morning" forest 90 1
want ch02 && build ch02 "Unfriendly Friends" forest 120 2
want ch03 && build ch03 "Seeking Answers" forest 120 3
want ch04 && build ch04 "Bumpy Cracks" forest 120 4 cave
want ch05 && build ch05 "Secret Stalkway" forest 100 5
rm -rf "$OUT"
