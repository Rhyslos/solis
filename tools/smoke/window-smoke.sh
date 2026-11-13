#!/usr/bin/env bash
# Window smoke test: opens the real editor on a virtual screen and checks
# what a script can: the window opens, the viewport is drawn in the right
# colours, a tool key picks a tool, the camera flies (right mouse and W),
# the shelf's grip and the editor groups' divider can be dragged, File >
# Exit quits, the next launch puts the tab, the split and the camera back
# where they were, and (with SAMPLE_MODEL set) an imported model is drawn.
#
# Usage (needs Xvfb, xdotool, ImageMagick and a GPU driver, such as Mesa's
# software one):
#   xvfb-run -s "-screen 0 1440x900x24" tools/smoke/window-smoke.sh <binary> <window title> [output folder]
#
# The window title starts with the program's name (APP_NAME in
# crates/app/src/branding.rs); CI passes it in, so this script never names
# the program. Screenshots land in the output folder for looking at in CI.
#
# Positions are for the design's frame (decisions 0099 and 0104) on a
# 1440x900 screen: the tab bar and ribbon along the top, the area's group
# under them (its tab strip at 100, the viewport from 130 to 542), the
# shelf's bar at 542 and its columns under it, the status bar at 876.
#
# Nothing here waits a fixed time for the program: after each step it
# waits until the screen shows what the step does (`wait_for`: the File
# page open, the tool lit, the view moved), then until the screen holds
# still, or until the program has ended. A screen that has only stopped
# changing isn't enough: right after a click, a slow software-drawn frame
# still shows the screen as it was (the review, TEST-03). An empty picture
# (one colour all over, before the window is first drawn) never counts
# (TEST-08). The short pauses that are left pace the mouse and keys
# themselves, as a hand does.
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

# Starts the program (with any arguments given) and waits up to 60 seconds
# for its window.
launch() {
    "$BIN" "$@" >>"$OUT/app.log" 2>&1 </dev/null &
    PID=$!
    for _ in $(seq 1 60); do
        if xdotool search --name "$TITLE" >/dev/null 2>&1; then break; fi
        kill -0 "$PID" 2>/dev/null || fail "the program exited before its window appeared"
        sleep 1
    done
    WINDOW_ID="$(xdotool search --name "$TITLE" 2>/dev/null | head -1)" || true
    [ -n "$WINDOW_ID" ] || fail "no window after 60 seconds"
    # The first frames are drawn (software rendering is slow).
    wait_for drawn || fail "the window stayed empty"
    xdotool windowfocus --sync "$WINDOW_ID" 2>/dev/null || true
    # On File › Recent projects (decision 0105), Esc carries on without a
    # project, into the editor.
    if tab_open "${FILE_TAB_X:-64}"; then
        xdotool key --window "$WINDOW_ID" Escape 2>/dev/null || true
    fi
    wait_for editor_shown || fail "the editor isn't shown (still on the File page)"
}

# A picture of the screen, as `$1`.
snap() {
    import -window root "$OUT/$1.png" 2>/dev/null || rm -f "$OUT/$1.png"
}

# Whether the picture `$1` is empty: missing, or one colour all over (the
# screen before the window is first drawn).
empty() {
    [ ! -s "$OUT/$1.png" ] ||
        [ "$(convert "$OUT/$1.png" -format '%[fx:standard_deviation>0]' info:)" = 0 ]
}

# Checks run by `wait_for` on the newest picture, `now`.
drawn() { true; }
# The top tab at `$1` is the one open: it's underlined in the accent blue.
tab_open() {
    local r _g b
    read -r r _g b <<<"$(pixel "$1" "${TAB_LINE_Y:-32}" now)"
    [ "$b" -gt 200 ] && [ "$r" -lt 150 ]
}
editor_shown() { ! tab_open "${FILE_TAB_X:-64}"; }
# More than `$2` pixels differ from the picture `$1` (within the region
# `$3`, if given).
differs() { [ "$(changed_pixels "$1" now "${3:-}")" -gt "$2" ]; }

# Takes pictures of the screen (`now`) until the check `$@` says the step
# has shown. An empty picture never counts. Gives up after a minute and
# says so (the check that follows then says what is wrong).
seen() {
    for _ in $(seq 1 240); do
        snap now
        if ! empty now && "$@"; then return 0; fi
        sleep 0.25
    done
    echo "Waited a minute for: $*"
    return 1
}

# As `seen`, then waits until the screen holds still, so the picture is a
# whole frame.
wait_for() {
    seen "$@" && hold_still
}

# Waits until two pictures of the screen in a row are the same and aren't
# empty (an animation ending, the pointer's highlight going), up to 40
# looks.
hold_still() {
    local last="" now
    for _ in $(seq 1 40); do
        snap still
        if ! empty still; then
            now="$(md5sum <"$OUT/still.png")"
            if [ "$now" = "$last" ]; then return 0; fi
            last="$now"
        fi
        sleep 0.25
    done
    echo "The screen was still changing after 40 looks; carrying on."
}

# Moves the pointer out of the way and keeps a picture of the screen as
# `$1`, once it holds still.
shot() {
    xdotool mousemove 1300 880
    hold_still
    snap "$1"
    empty "$1" && fail "the screen is empty"
    return 0
}

# Colour of one screen pixel, as "R G B".
pixel() {
    convert "$OUT/$3.png" -format "%[fx:int(255*p{$1,$2}.r)] %[fx:int(255*p{$1,$2}.g)] %[fx:int(255*p{$1,$2}.b)]" info:
}

# The viewport's sky is drawn: near its top, not black.
sky_drawn() {
    local _r _g b
    read -r _r _g b <<<"$(pixel 300 156 now)"
    [ "$b" -ge 15 ]
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

# Waits up to `$1` seconds for the program to end. Says whether it did.
ended_within() {
    for _ in $(seq 1 $(($1 * 4))); do
        kill -0 "$PID" 2>/dev/null || return 0
        sleep 0.25
    done
    return 1
}

# File > Exit with the mouse, then wait for the program to end.
quit() {
    # The File tab is the first tab after the logo; Exit is the last item
    # of the File page's column, under Learn, Updates and About (the
    # column's own sizes are FILE_MENU_* in crates/editor/src/tokens/size.rs).
    xdotool mousemove "${FILE_TAB_X:-64}" "${FILE_TAB_Y:-19}" click 1
    wait_for tab_open "${FILE_TAB_X:-64}" || fail "File did not open the File page"
    snap file-page
    xdotool mousemove "${EXIT_X:-110}" "${EXIT_Y:-534}" click 1
    # With unsaved changes (an imported model), Solis asks first: answer
    # Don't save, on the left of the box in the middle, once it's shown.
    for _ in $(seq 1 240); do
        kill -0 "$PID" 2>/dev/null || break
        snap now
        if ! empty now && differs file-page 1000; then
            hold_still
            xdotool mousemove "${DONT_SAVE_X:-554}" "${DONT_SAVE_Y:-479}" click 1
            break
        fi
        sleep 0.25
    done
    ended_within 15 || fail "File > Exit did not quit"
    wait "$PID" || fail "the program exited with an error"
    PID=""
}

launch
wait_for sky_drawn || true
shot first

# 1. The viewport is drawn: near its top is the sky, a dark neutral grey
#    (the design's backdrop: no tint, red, green and blue the same). A
#    wrong colour setting shows as black or washed out.
read -r R G B <<<"$(pixel 300 156 first)"
echo "Sky at (300, 156): $R $G $B (expected a dark neutral grey)"
[ "$B" -ge 15 ] && [ "$B" -le 80 ] || fail "the sky is not a dark grey: ($R, $G, $B)"
[ $((R > B ? R - B : B - R)) -le 4 ] && [ $((G > B ? G - B : B - G)) -le 4 ] ||
    fail "the sky is tinted, not neutral: ($R, $G, $B)"
grep -q "No 3D view" "$OUT/app.log" && fail "the 3D view could not be shown"

# 2. W picks the Move tool: its button in the toolbox (the column down
#    the viewport's left edge) lights up. Only the toolbox's transform
#    tools are compared, so the viewport moving beside it doesn't count.
xdotool key --window "$WINDOW_ID" w || true
wait_for differs first 100 "34x180+5+${TOOLS_Y:-136}" || true
shot move-tool
TOOL_CHANGE="$(changed_pixels first move-tool 34x180+5+${TOOLS_Y:-136})"
echo "Pixels changed in the toolbox by pressing W: $TOOL_CHANGE"
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
wait_for differs move-tool 5000 800x370+280+140 || true
shot flown
FLY_CHANGE="$(changed_pixels move-tool flown 800x370+280+140)"
echo "Pixels changed in the viewport by flying: $FLY_CHANGE"
[ "$FLY_CHANGE" -gt 5000 ] || fail "right mouse and W did not fly the camera"
TOOL_AFTER="$(changed_pixels move-tool flown 34x180+5+${TOOLS_Y:-136})"
echo "Pixels changed in the toolbox by flying: $TOOL_AFTER"
[ "$TOOL_AFTER" -lt 100 ] || fail "W picked a tool while flying"

# Drags with the left button from ($1, $2) by ($3, $4), in steps, and
# lets go once the screen shows the drag.
drag() {
    snap before-drag
    xdotool mousemove "$1" "$2"
    sleep 0.3
    xdotool mousedown 1
    for i in $(seq 1 20); do
        xdotool mousemove $(($1 + i * $3 / 20)) $(($2 + i * $4 / 20))
        sleep 0.05
    done
    seen differs before-drag 5000 || true
    xdotool mouseup 1
}

# 4. Drag the shelf's grip up: the shelf grows and the viewport shrinks.
drag "${GRIP_X:-28}" "${GRIP_Y:-559}" 0 -100
shot grown
GROW_CHANGE="$(changed_pixels flown grown 1440x200+0+400)"
echo "Pixels changed by dragging the shelf's grip: $GROW_CHANGE"
[ "$GROW_CHANGE" -gt 5000 ] || fail "dragging the shelf's grip moved nothing"

# 5. On the Audio tab (the mixer beside the Bus group, split 60 / 40),
#    drag the divider between the two groups to the left.
xdotool mousemove "${AUDIO_TAB_X:-481}" "${AUDIO_TAB_Y:-19}" click 1
wait_for tab_open "${AUDIO_TAB_X:-481}" || fail "the Audio tab did not open"
shot audio
drag "${DIVIDER_X:-864}" "${DIVIDER_Y:-300}" -200 0
shot dragged
SPLIT_CHANGE="$(changed_pixels audio dragged 1440x400+0+100)"
echo "Pixels changed by dragging the divider: $SPLIT_CHANGE"
[ "$SPLIT_CHANGE" -gt 5000 ] || fail "dragging the groups' divider moved nothing"

# 6. Quit, and the frame was saved under its own key.
quit
SAVED="$(grep -rl '"shell"' "$TEST_HOME" 2>/dev/null | head -1 || true)"
[ -n "$SAVED" ] || fail "the frame was not saved"
echo "Frame saved in: ${SAVED#"$TEST_HOME"/}"

# 7. The next launch opens on Audio with the divider where it was dragged
#    (and the camera where it flew): the screen matches the dragged one,
#    not the first one.
launch
shot reopened
SAME="$(changed_pixels dragged reopened)"
FIRST="$(changed_pixels first reopened)"
echo "Pixels different from the dragged frame: $SAME; from the first launch: $FIRST"
[ "$SAME" -lt "$FIRST" ] || fail "the dragged frame was not restored"
[ "$SAME" -lt 20000 ] || fail "the reopened editor does not match the dragged frame"
quit

# Red pixels (the sample's base) in the middle of the viewport, in the
# picture `$1`.
red_pixels() {
    convert "$OUT/$1.png" -crop 600x400+250+150 +repage -fx 'r>0.6 && g<0.45 && b<0.45 ? 1 : 0' -format '%[fx:int(mean*w*h)]' info:
}
model_drawn() { [ "$(red_pixels now)" -gt 2000 ]; }

# 8. Importing a model: start with --import and the sample model (a
#    checker cube on a red base; SAMPLE_MODEL names the file, made with
#    `cargo run -p assets --example sample -- sample.glb`). The camera
#    frames it, so red fills part of the middle of the viewport. The frame
#    reopens on the tab used last (Audio, step 5), and an import doesn't
#    change the tab: Home shows the 3D view.
if [ -n "${SAMPLE_MODEL:-}" ]; then
    launch --import "$SAMPLE_MODEL"
    xdotool mousemove "${HOME_TAB_X:-114}" "${HOME_TAB_Y:-19}" click 1
    wait_for model_drawn || true
    shot imported
    RED="$(red_pixels imported)"
    echo "Red pixels of the sample's base in the viewport: $RED"
    [ "$RED" -gt 2000 ] || fail "the imported model is not drawn"
    grep -q "No 3D view" "$OUT/app.log" && fail "the 3D view could not be shown"
    quit
fi

echo "Window smoke test passed."
