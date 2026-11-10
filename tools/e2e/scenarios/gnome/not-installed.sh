# run with NO_INSTALL=1: GNOME has not picked up the helper extension yet
printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
FAIL=0
check() { if "${@:2}"; then echo "ok   $1"; else echo "FAIL $1"; FAIL=1; fi; }
titles() { windows | python3 -c "import json,sys; print('|'.join(x['title'] for x in json.load(sys.stdin)))"; }
dialogs() { windows | python3 -c "import json,sys; print(sum(1 for x in json.load(sys.stdin) if x['wmClass'] == 'tinyhand' and not x['title']))"; }
TH_TIMEOUT=40 th > $E/th.log 2>&1 &
sleep 2
th on; sleep 1.5
th status >/dev/null; check "pointer stays off" test $? = 1
check "no overlay window" eval '! titles | grep -q "TinyHand Overlay"'
check "explains what to do" test "$(dialogs)" = 1
shot not-installed
th on; sleep 1
check "asks only once at a time" test "$(dialogs)" = 1
th quit
echo "RESULT=$FAIL"
