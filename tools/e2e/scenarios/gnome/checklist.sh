printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
mkdir -p $XDG_STATE_HOME/tinyhand && touch $XDG_STATE_HOME/tinyhand/hint-shown
FAIL=0
check() { if "${@:2}"; then echo "ok   $1"; elif [[ $1 == KNOWN* ]]; then echo "known $1"; else echo "FAIL $1"; FAIL=1; fi; }
overlay() { windows | python3 -c "import json,sys; w=[x for x in json.load(sys.stdin) if x['title'].startswith('TinyHand Overlay')]; print(json.dumps(w[0]) if w else '')"; }
field() { overlay | python3 -c "import json,sys; t=sys.stdin.read().strip(); print(json.loads(t)['$1'] if t else 'none')"; }
settings_page() { magick $E/p.png -crop 300x40+20+${1}+0 +repage -format "%[fx:mean]" info: 2>/dev/null; }
TH_TIMEOUT=80 th > $E/th.log 2>&1 &
sleep 1.5
TH_TIMEOUT=80 th settings > $E/settings.log 2>&1 &
sleep 2.5
th on; sleep 1.2; cur 600 400; sleep 0.3
check "overlay above"  test "$(field above)" = True
check "overlay sticky" test "$(field sticky)" = True
check "overlay on top" test "$(field top)" = True
cur 1040 726; sleep 0.3; down; up; sleep 0.6
check "tap does not reach the settings window" eval '! grep -q "^tray = true" $TINYHAND_CONFIG'
keydown 0xffe9; sleep 0.4
cur 1042 727; sleep 0.2; down; up; sleep 0.6
keyup 0xffe9; sleep 0.5
check "Alt-click reaches the settings window" grep -q "^tray = true" $TINYHAND_CONFIG
th status >/dev/null; check "still on after Alt-click" test $? = 0
cur 600 400; sleep 0.3
check "overlay focused again after Alt-click" test "$(field focused)" = True
check "overlay back on top after Alt-click" test "$(field top)" = True
drv Workspace 1; sleep 0.8
check "overlay visible on workspace 2" test "$(field sticky)" = True
drv Workspace 0; sleep 0.5
key 0xff1b; sleep 0.6
th status >/dev/null; check "Esc turns it off" test $? = 1
th on; sleep 1
drv Overview true; sleep 1.2
th status >/dev/null; check "overview turns it off" test $? = 1
drv Overview false; sleep 0.5
th quit
grep -iE "JS (ERROR|WARNING)" $E/shell.log | head -5
echo "RESULT=$FAIL"
