printf 'sound = false\n' > $TINYHAND_CONFIG
mkdir -p $XDG_STATE_HOME/tinyhand && touch $XDG_STATE_HOME/tinyhand/hint-shown
FAIL=0
check() { if "${@:2}"; then echo "ok   $1"; else echo "FAIL $1"; FAIL=1; fi; }
running() { gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus --method org.freedesktop.DBus.NameHasOwner io.github.xdinterface.TinyHand | grep -q true; }
TH_TIMEOUT=60 th > $E/th.log 2>&1 &
sleep 2.5
check "app is running" running
shot before
cur 1305 16; sleep 0.3; down; up; sleep 0.8
cur 1265 193; sleep 0.2; down; up; sleep 1.5
check "Quit from the top-bar menu ends the app" eval '! running'
shot after
check "top-bar icon is gone after Quit" eval "test \"$(magick $E/after.png -crop 20x20+1295+6 +repage -format %[fx:mean] info:)\" \\< 0.35"
TH_TIMEOUT=20 th on > /dev/null 2>&1 &
sleep 2.5
check "app comes back with tinyhand on" running
shot back
check "top-bar icon returns" eval "test \"$(magick $E/back.png -crop 20x20+1295+6 +repage -format %[fx:mean] info:)\" \\> 0.35"
th quit; sleep 1
grep -iE "JS (ERROR|WARNING)" $E/shell.log | head -5
echo "RESULT=$FAIL"
