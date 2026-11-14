#!/usr/bin/env bash
# Takes the user guide's pictures from the real editor (decision 0113):
# every picture in pictures.list, into docs/manual/pictures/, then writes
# the stamp of the interface they show (interface.txt) and builds the
# guide again. Run it whenever the interface changes; a test says when.
#
# Usage, from anywhere (needs Xvfb, xdotool, ImageMagick and a GPU driver,
# such as Mesa's software one; builds nothing itself):
#   cargo build -p app
#   tools/guide-pictures/take-pictures.sh [only-this-picture.png]
#
# The program and its window's title come from crates/app/src/branding.rs
# (APP_ID and APP_NAME), so this script never names the program. Set BIN
# to use another build. Without a screen it runs itself on a virtual one,
# 1440 x 900, with Mesa's software driver.
#
# Each session of the list starts the editor afresh, in a throwaway home
# folder (so it starts like a first launch and never touches real
# settings), on a new copy of one of File > Learn's samples made from the
# command line (--sample). Its steps press keys and click at places on
# the 1440 x 900 screen of the standard layout; a picture is taken once
# the screen holds still, cropped to what it shows, and made small.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BRANDING="$ROOT/crates/app/src/branding.rs"
constant() { sed -n "s/^pub const $1: &str = \"\(.*\)\";$/\1/p" "$BRANDING"; }
APP_ID="$(constant APP_ID)"
TITLE="$(constant APP_NAME)"
BIN="${BIN:-${CARGO_TARGET_DIR:-$ROOT/target}/debug/$APP_ID}"
LIST="${LIST:-$HERE/pictures.list}"
OUT="${OUT:-$ROOT/docs/manual/pictures}"
FONT="$ROOT/crates/editor/assets/fonts/Inter-SemiBold.ttf"
ONLY="${1:-}"

if [ -z "${DISPLAY:-}" ]; then
    exec xvfb-run -a -s "-screen 0 1440x900x24" "$0" "$@"
fi
export WGPU_BACKEND="${WGPU_BACKEND:-gl}"
[ -x "$BIN" ] || { echo "No program at $BIN: build it first (cargo build -p app)."; exit 1; }
mkdir -p "$OUT"
# A short, plain folder: the launcher's pictures show where projects are.
WORK="/tmp/guide-pictures"
rm -rf "$WORK"
mkdir -p "$WORK"
SESSIONS=0
PID=""
# A double-click on a code file, or Documentation, opens another program
# on a real desktop; here nothing is opened.
mkdir -p "$WORK/bin"
for opener in xdg-open gio gnome-open kde-open www-browser; do
    printf '#!/bin/sh\nexit 0\n' >"$WORK/bin/$opener"
    chmod +x "$WORK/bin/$opener"
done
export PATH="$WORK/bin:$PATH" BROWSER=true
trap 'stop_editor; rm -rf "$WORK"' EXIT

say() { echo "  $*"; }
fail() { echo "PICTURES FAILED: $*"; [ -f "$WORK/app.log" ] && tail -n 20 "$WORK/app.log"; exit 1; }

# The editor, its window, and the home folder it started in.
stop_editor() {
    if [ -n "$PID" ]; then
        kill "$PID" 2>/dev/null || true
        for _ in $(seq 1 20); do kill -0 "$PID" 2>/dev/null || break; sleep 0.25; done
        kill -9 "$PID" 2>/dev/null || true
        PID=""
    fi
}
start_editor() {
    "$BIN" "$@" >>"$WORK/app.log" 2>&1 </dev/null &
    PID=$!
    for _ in $(seq 1 90); do
        xdotool search --name "^$TITLE" >/dev/null 2>&1 && break
        kill -0 "$PID" 2>/dev/null || fail "the editor ended before its window opened"
        sleep 1
    done
    WINDOW="$(xdotool search --name "^$TITLE" | head -1)"
    [ -n "$WINDOW" ] || fail "no window after 90 seconds"
    xdotool windowfocus --sync "$WINDOW" 2>/dev/null || true
    hold_still
}
session() {
    stop_editor
    export HOME="$WORK/home-$((SESSIONS += 1))"
    export XDG_CONFIG_HOME="$HOME/.config" XDG_DATA_HOME="$HOME/.local/share"
    mkdir -p "$HOME"
    # A short, plain place, since the launcher shows it.
    PROJECT="$HOME/Games/$1"
    case "$1" in
        none) start_editor ;;
        *) start_editor --sample "$1" "$PROJECT" ;;
    esac
}
# The editor started again in the same home folder, with no project open
# or on a new copy of a sample. Quit it first with File > Exit (its steps
# in the list), so it keeps what it knows, such as the projects the
# launcher shows.
restart() {
    for _ in $(seq 1 40); do kill -0 "$PID" 2>/dev/null || break; sleep 0.25; done
    stop_editor
    if [ -n "${1:-}" ]; then
        PROJECT="$HOME/Games/$1"
        start_editor --sample "$1" "$PROJECT"
    else
        start_editor
    fi
}

# Waits until two looks at the screen in a row are the same and aren't
# empty (software drawing is slow), up to about 30 seconds.
hold_still() {
    local last="" now
    for _ in $(seq 1 60); do
        import -window root "$WORK/look.png" 2>/dev/null || true
        if [ -s "$WORK/look.png" ] &&
            [ "$(convert "$WORK/look.png" -format '%[fx:standard_deviation>0]' info:)" = 1 ]; then
            now="$(md5sum <"$WORK/look.png")"
            [ "$now" = "$last" ] && return 0
            last="$now"
        fi
        sleep 0.5
    done
    say "the screen never held still; taking it as it is"
}

# The steps a picture's lines may use.
key() { xdotool key --window "$WINDOW" "$@"; sleep 0.6; }
type_text() { xdotool type --delay 60 --window "$WINDOW" "$*"; sleep 0.6; }
move() { xdotool mousemove "$1" "$2"; sleep 0.4; }
click() { xdotool mousemove "$1" "$2" click 1; sleep 0.8; }
double() { xdotool mousemove "$1" "$2" click --repeat 2 --delay 120 1; sleep 0.8; }
right() { xdotool mousemove "$1" "$2" click 3; sleep 0.8; }
drag() {
    local x="$1" y="$2" to_x="$3" to_y="$4" steps=12 i
    xdotool mousemove "$x" "$y" mousedown 1
    sleep 0.3
    for i in $(seq 1 $steps); do
        xdotool mousemove $((x + (to_x - x) * i / steps)) $((y + (to_y - y) * i / steps))
        sleep 0.08
    done
    sleep 0.3
    xdotool mouseup 1
    sleep 0.8
}
wait_s() { sleep "$1"; }
# A file of the project, written by the script (`write assets/x.lst text`),
# with \n for a new line.
write_file() {
    local path="$PROJECT/$1"
    shift
    mkdir -p "$(dirname "$path")"
    printf '%b' "$*" >"$path"
    sleep 1
}

# Puts text at the start of a project's file (`prepend assets/main.lst
# text`), with \n for a new line.
prepend_file() {
    local path="$PROJECT/$1"
    shift
    { printf '%b' "$*"; cat "$path"; } >"$path.new" && mv "$path.new" "$path"
    sleep 1
}

# The picture being made: its name, page, what it shows, and its labels.
NAME=""
PAGE=""
SHOWS=""
LABELS=()
begin_picture() {
    NAME="$1"
    PAGE=""
    SHOWS=""
    LABELS=()
}
# Takes the picture: the screen once it holds still, cropped to x y w h,
# labelled, scaled to `width` pixels across when given, and made small.
shot() {
    local x="$1" y="$2" w="$3" h="$4" width="${5:-}"
    hold_still
    local args=(-crop "${w}x${h}+${x}+${y}" +repage)
    for label in "${LABELS[@]}"; do
        # `label X Y N`: a numbered mark at X Y, in the screen's pixels.
        read -r lx ly text <<<"$label"
        lx=$((lx - x))
        ly=$((ly - y))
        args+=(-fill '#2a62c9' -stroke white -strokewidth 2
            -draw "circle $lx,$ly $((lx + 13)),$ly"
            -fill white -stroke none -font "$FONT" -pointsize 16
            -draw "text $((lx - 5 * ${#text})),$((ly + 6)) '$text'")
    done
    [ -n "$width" ] && args+=(-resize "${width}x")
    convert "$WORK/look.png" "${args[@]}" -strip "$WORK/shot.png"
    # Fewer colours keep the file small; the interface's flat colours
    # don't miss them.
    convert "$WORK/shot.png" -dither None -colors 192 -define png:compression-level=9 \
        "PNG8:$OUT/$NAME"
    say "$NAME ($(($(stat -c %s "$OUT/$NAME") / 1024)) KB): $SHOWS"
}

# Reads the list: a session line starts the editor; a picture line starts
# a picture; the indented lines under it are its page, what it shows, and
# its steps, each one of the functions above.
run_list() {
    local line word rest skipping=false
    while IFS= read -r line || [ -n "$line" ]; do
        line="${line%%#*}"
        [ -z "${line// /}" ] && continue
        read -r word rest <<<"$line"
        case "$word" in
            session)
                echo "Session: $rest"
                session $rest
                ;;
            picture)
                begin_picture "$rest"
                skipping=false
                [ -n "$ONLY" ] && [ "$ONLY" != "$rest" ] && skipping=true
                ;;
            page) PAGE="$rest" ;;
            shows) SHOWS="$rest" ;;
            label) LABELS+=("$rest") ;;
            shot) $skipping || shot $rest ;;
            key) key $rest ;;
            type) type_text "$rest" ;;
            move) move $rest ;;
            click) click $rest ;;
            double) double $rest ;;
            right) right $rest ;;
            drag) drag $rest ;;
            wait) wait_s $rest ;;
            still) hold_still ;;
            write) write_file $rest ;;
            prepend) prepend_file $rest ;;
            restart) restart $rest ;;
            *) fail "pictures.list: unknown step \"$word\"" ;;
        esac
    done <"$LIST"
}

echo "Taking the guide's pictures with $BIN"
run_list
stop_editor
# A trial run (one picture, or into another folder) leaves the stamp and
# the guide alone.
if [ -z "$ONLY" ] && [ "$OUT" = "$ROOT/docs/manual/pictures" ]; then
    (cd "$ROOT" && cargo run -q -p user-guide -- --stamp-pictures && cargo run -q -p user-guide)
fi
echo "Done. Look at the pictures in docs/manual/pictures, and commit them with the stamp and the guide."
