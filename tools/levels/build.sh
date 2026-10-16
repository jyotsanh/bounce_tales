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
build() { # name title music par seed [flags: cave mine bgcave dark]
    python3 tools/levels/$1.py > /dev/null
    cargo run -q --bin ascii2tmx -- tools/levels/ascii/$1.txt "$OUT/$1.tmx" > /dev/null
    n=$1; shift
    python3 tools/levels/finish_tmx.py tools/levels/ascii/$n.txt "$OUT/$n.tmx" assets/levels/$n.tmx "$@"
}
want() { [ -z "$ONLY" ] || [ "$ONLY" = "$1" ]; }
ONLY=$1
want ch01 && build ch01 "Misty Morning" forest 35 1
want ch02 && build ch02 "Unfriendly Friends" forest 30 2
want ch03 && build ch03 "Seeking Answers" forest 45 3
want ch04 && build ch04 "Bumpy Cracks" forest 40 4 cave
want ch05 && build ch05 "Secret Stalkway" forest 60 5
want ch06 && build ch06 "Into the Mines" cave 55 6 cave mine
want ch07 && build ch07 "A Gloomy Path" cave 45 7 cave mine dark
want ch08 && build ch08 "Rumbling Sounds" cave 35 8 cave mine
want ch09 && build ch09 "Trapped in Machine" castle 80 9 cave machine
want ch10 && build ch10 "Tunnel of Treasures" cave 40 10 cave mine
want ch11 && build ch11 "Wicked Circus" castle 35 11 circus
want ch12 && build ch12 "Hunting Colours" forest 50 12
want ch13 && build ch13 "Almost There" castle 120 13 cave machine
want ch14 && build ch14 "Final Ride" boss 95 14 machine
want ch15 && build ch15 "Fantastic Fair" title 35 15 circus
rm -rf "$OUT"
