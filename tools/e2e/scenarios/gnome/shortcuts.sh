# run with NO_HELPER=1 so the app has to switch the helper extension on itself
printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
mkdir -p $XDG_STATE_HOME/tinyhand && touch $XDG_STATE_HOME/tinyhand/hint-shown
FAIL=0
check() { if "${@:2}"; then echo "ok   $1"; else echo "FAIL $1"; FAIL=1; fi; }
chord() { keydown 0xffe3; keydown 0xffe9; key "$1"; keyup 0xffe9; keyup 0xffe3; sleep 0.8; }
status() { th status >/dev/null; echo $?; }
check "helper starts disabled" eval '! gnome-extensions info tinyhand@xdinterface.github.io | grep -q "Enabled: Yes"'
TH_TIMEOUT=60 th > $E/th.log 2>&1 &
sleep 2.5
check "app switches the helper on" eval 'gnome-extensions info tinyhand@xdinterface.github.io | grep -q "State: ACTIVE"'
check "marker written" test -e $XDG_STATE_HOME/tinyhand/gnome-extension-enabled
chord 0x68
check "Ctrl+Alt+H turns it on" test "$(status)" = 0
chord 0x68
check "Ctrl+Alt+H turns it off" test "$(status)" = 1
printf 'toggle-shortcut = "ctrl+alt+j"\n' >> $TINYHAND_CONFIG; sleep 1
chord 0x68
check "old shortcut is released" test "$(status)" = 1
chord 0x6a
check "new shortcut works" test "$(status)" = 0
th off
sed -i 's/ctrl+alt+j/off/' $TINYHAND_CONFIG; sleep 1
chord 0x6a
check "shortcut can be switched off" test "$(status)" = 1
th quit
grep -v "^$" $E/th.log | head -5
grep -iE "JS (ERROR|WARNING)" $E/shell.log | head -5
echo "RESULT=$FAIL"
