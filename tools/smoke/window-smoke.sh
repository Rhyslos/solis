#!/usr/bin/env bash
# Window smoke test: opens the real editor on a virtual screen and checks the
# Phase 0 gate as far as a script can: the window opens, the viewport is
# drawn in the right colours, a tool key picks a tool, a panel can be dragged
# to a new place, File > Quit quits, and the next launch puts the panel back
# where it was dragged.
#
# Usage (needs Xvfb, xdotool, ImageMagick and a GPU driver, such as Mesa's
# software one):
#   xvfb-run -s "-screen 0 1440x900x24" tools/smoke/window-smoke.sh <binary> <window title> [output folder]
#
# The window title starts with the program's name (APP_NAME in
# crates/app/src/branding.rs); CI passes it in, so this script never names
# the program. Screenshots land in the output folder for looking at in CI.
#
# Positions are for the Default layout on a 1440x900 screen.
set -euo pipefail

BIN="${1:?usage: window-smoke.sh <binary> <window title> [output folder]}"
TITLE="${2:?usage: window-smoke.sh <binary> <window title> [output folder]}"
OUT="${3:-smoke-output}"
mkdir -p "$OUT"
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"

# A throwaway home folder, so the test starts like a first launch and never
# touches real settings.
TEST_HOME="$(mktemp -d)"
export HOME="$TEST_HOME"
export XDG_CONFIG_HOME="$TEST_HOME/.config"
export XDG_DATA_HOME="$TEST_HOME/.local/share"

PID=""
fail() {
    echo "SMOKE TEST FAILED: $1"
    echo "--- Program output ---"
    cat "$OUT/app.log" || true
    if [ -n "${GITHUB_ACTIONS:-}" ]; then
        local msg
        msg="$1"$'\n'"Last lines of the program's output:"$'\n'"$(tail -n 25 "$OUT/app.log" 2>/dev/null)"
        msg="${msg//'%'/'%25'}"
        msg="${msg//$'\r'/'%0D'}"
        msg="${msg//$'\n'/'%0A'}"
        echo "::error title=Window smoke test::$msg"
    fi
    [ -n "$PID" ] && kill "$PID" 2>/dev/null || true
    exit 1
}

# Starts the program and waits up to 60 seconds for its window.
launch() {
    "$BIN" >>"$OUT/app.log" 2>&1 </dev/null &
    PID=$!
    for _ in $(seq 1 60); do
        if xdotool search --name "$TITLE" >/dev/null 2>&1; then break; fi
        kill -0 "$PID" 2>/dev/null || fail "the program exited before its window appeared"
        sleep 1
    done
    WINDOW_ID="$(xdotool search --name "$TITLE" 2>/dev/null | head -1)" || true
    [ -n "$WINDOW_ID" ] || fail "no window after 60 seconds"
    # Let the first frames draw (software rendering is slow).
    sleep 6
    xdotool windowfocus --sync "$WINDOW_ID" 2>/dev/null || true
}

shot() {
    xdotool mousemove 1300 880
    sleep 1
    import -window root "$OUT/$1.png"
}

# Colour of one screen pixel, as "R G B".
pixel() {
    convert "$OUT/$3.png" -format "%[fx:int(255*p{$1,$2}.r)] %[fx:int(255*p{$1,$2}.g)] %[fx:int(255*p{$1,$2}.b)]" info:
}

# Number of pixels that differ between two screenshots, within an optional
# region (WxH+X+Y).
changed_pixels() {
    local a="$OUT/$1.png" b="$OUT/$2.png"
    if [ -n "${3:-}" ]; then
        convert "$a" -crop "$3" +repage "$OUT/crop-a.png"
        convert "$b" -crop "$3" +repage "$OUT/crop-b.png"
        a="$OUT/crop-a.png" b="$OUT/crop-b.png"
    fi
    { compare -metric AE -fuzz 4% "$a" "$b" null: 2>&1 || true; } | awk '{print int($1)}'
}

# File > Quit with the mouse, then wait for the program to end.
quit() {
    xdotool mousemove 19 16 click 1
    sleep 1
    xdotool mousemove 40 "${QUIT_Y:-134}" click 1
    for _ in $(seq 1 15); do
        kill -0 "$PID" 2>/dev/null || break
        sleep 1
    done
    kill -0 "$PID" 2>/dev/null && fail "File > Quit did not quit"
    wait "$PID" || fail "the program exited with an error"
    PID=""
}

launch
shot first

# 1. The viewport is drawn: near its top is the sky, a dark blue-grey. A
#    wrong colour setting shows as black or washed out.
read -r R G B <<<"$(pixel 300 90 first)"
echo "Sky at (300, 90): $R $G $B (expected a dark blue-grey)"
[ "$B" -ge 35 ] && [ "$B" -le 80 ] || fail "the sky is not a dark blue-grey: ($R, $G, $B)"
[ "$B" -gt "$R" ] || fail "the sky is not blue-grey: ($R, $G, $B)"
grep -q "No 3D view" "$OUT/app.log" && fail "the 3D view could not be shown"

# 2. W picks the Move tool: its button in the tool bar lights up.
xdotool key --window "$WINDOW_ID" w || true
shot move-tool
TOOL_CHANGE="$(changed_pixels first move-tool 200x50+450+630)"
echo "Pixels changed in the tool bar by pressing W: $TOOL_CHANGE"
[ "$TOOL_CHANGE" -gt 100 ] || fail "W did not pick the Move tool"

# 3. Fly: hold the right mouse button in the viewport, turn a little and
#    press W. The view changes, and W moves the camera instead of picking a
#    tool (Move is still the tool).
xdotool mousemove 500 300
sleep 0.3
xdotool mousedown 3
sleep 0.3
for i in $(seq 1 10); do
    xdotool mousemove_relative -- 6 0
    sleep 0.05
done
xdotool keydown w
sleep 1.5
xdotool keyup w
xdotool mouseup 3
shot flown
FLY_CHANGE="$(changed_pixels move-tool flown 1000x500+50+120)"
echo "Pixels changed in the viewport by flying: $FLY_CHANGE"
[ "$FLY_CHANGE" -gt 5000 ] || fail "right mouse and W did not fly the camera"
TOOL_AFTER="$(changed_pixels move-tool flown 200x50+450+630)"
echo "Pixels changed in the tool bar by flying: $TOOL_AFTER"
[ "$TOOL_AFTER" -lt 100 ] || fail "W picked a tool while flying"

# 4. Drag the Console tab onto the right-hand column, below Properties.
xdotool mousemove 721 689
sleep 0.3
xdotool mousedown 1
for i in $(seq 1 20); do
    xdotool mousemove $((721 + i * 28)) $((689 + i * 6))
    sleep 0.1
done
sleep 0.5
xdotool mouseup 1
shot dragged
DRAG_CHANGE="$(changed_pixels flown dragged)"
echo "Pixels changed by dragging the Console: $DRAG_CHANGE"
[ "$DRAG_CHANGE" -gt 5000 ] || fail "dragging the Console tab moved nothing"

# 5. Quit, and the layout was saved.
quit
SAVED="$(grep -rl "panel_layout" "$TEST_HOME" 2>/dev/null | head -1 || true)"
[ -n "$SAVED" ] || fail "the panel layout was not saved"
echo "Layout saved in: ${SAVED#"$TEST_HOME"/}"

# 6. The next launch puts the Console back where it was dragged and the
#    camera where it flew: the screen matches the dragged one, not the
#    first one.
launch
shot reopened
SAME="$(changed_pixels dragged reopened)"
FIRST="$(changed_pixels first reopened)"
echo "Pixels different from the dragged layout: $SAME; from the first launch: $FIRST"
[ "$SAME" -lt "$FIRST" ] || fail "the dragged layout was not restored"
[ "$SAME" -lt 20000 ] || fail "the reopened editor does not match the dragged layout"
quit

echo "Window smoke test passed."
