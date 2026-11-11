printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
mkdir -p $XDG_STATE_HOME/tinyhand && touch $XDG_STATE_HOME/tinyhand/hint-shown
FAIL=0
check() { if "${@:2}"; then echo "ok   $1"; else echo "FAIL $1"; FAIL=1; fi; }
overlay() { windows | python3 -c "import json,sys; w=[x for x in json.load(sys.stdin) if x['title'].startswith('TinyHand Overlay')]; print(json.dumps(w[0]) if w else '')"; }
field() { overlay | python3 -c "import json,sys; t=sys.stdin.read().strip(); print(json.loads(t)['$1'] if t else 'none')"; }
combo() { keydown "$1"; key "$2"; keyup "$1"; sleep 1; }
healthy() { th status >/dev/null && [ "$(field top)" = True ] && [ "$(field rect)" = "[0, 32, 1440, 928]" ]; }
TH_TIMEOUT=80 th > $E/th.log 2>&1 &
sleep 1.5
th on; sleep 1.2; cur 600 400; sleep 0.3
echo "rect=$(field rect)"
check "healthy at start" healthy
combo 0xffeb 0x68; check "Super+H (hide) leaves it usable" healthy; echo "  rect=$(field rect) top=$(field top)"
th off; sleep 0.5; th on; sleep 1.2; cur 600 400
combo 0xffeb 0xff54; check "Super+Down (restore) leaves it usable" healthy; echo "  rect=$(field rect)"
th off; sleep 0.5; th on; sleep 1.2; cur 600 400
combo 0xffeb 0xff51; check "Super+Left (tile) leaves it usable" healthy; echo "  rect=$(field rect)"
th off; sleep 0.5; th on; sleep 1.2; cur 600 400
combo 0xffe9 0xffc1; th status >/dev/null; check "Alt+F4 turns the pointer off cleanly" test "$?/$(field top)" = 1/none
th on; sleep 1.2; cur 600 400
check "works again after Alt+F4" healthy
th quit
echo "RESULT=$FAIL"
