printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
mkdir -p $XDG_STATE_HOME/tinyhand && touch $XDG_STATE_HOME/tinyhand/hint-shown
FAIL=0
check() { if "${@:2}"; then echo "ok   $1"; else echo "FAIL $1"; FAIL=1; fi; }
cursor() { gdbus call --session --dest org.tinyhand.Test --object-path /org/tinyhand/Test --method org.tinyhand.Test.Cursor | sed "s/^('//; s/',)$//"; }
TH_TIMEOUT=60 th > $E/th.log 2>&1 &
sleep 1.5
cur 600 400; sleep 0.3
echo "before on: $(cursor)"
th on; sleep 1.2; cur 610 410; sleep 0.4
echo "hover screen: $(cursor)"
check "no cursor over the screen" test "$(cursor)" = none
down; cur 640 420; sleep 0.4
echo "drag screen: $(cursor)"
check "no cursor while dragging" test "$(cursor)" = none
up; sleep 0.3
cur 720 925; sleep 0.8; shot toolbar
echo "hover toolbar: $(cursor)"
check "no cursor over the toolbar" test "$(cursor)" = none
down; sleep 0.4
echo "press toolbar: $(cursor)"
check "no cursor while pressing a toolbar button" test "$(cursor)" = none
up; sleep 0.3
th quit
echo "RESULT=$FAIL"
