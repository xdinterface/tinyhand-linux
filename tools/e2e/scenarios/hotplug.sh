# Overlay presence is read from screenshots: a new overlay shows its dark toolbar for 3 s,
# and the hand draws at the pointer. Background is #dfe3dc (mean ~0.88).
printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
mkdir -p $XDG_STATE_HOME/tinyhand && touch $XDG_STATE_HOME/tinyhand/hint-shown
TH_TIMEOUT=60 th > $E/th.log 2>&1 &
sleep 1.5
FAIL=0
mean() { magick "$1" -crop "$2" +repage -colorspace gray -format "%[fx:mean]" info:; }
toolbar() { grim -o "$1" $E/hp.png; [ "$(mean $E/hp.png 480x60+480+890)" \< 0.75 ]; }
hand() { grim -o HEADLESS-$1 $E/hp.png; magick $E/hp.png -crop 140x140+$2+$3 +repage -format "%[fx:standard_deviation]" info: | awk '{exit !($1 > 0.05)}'; }
check() { if "${@:2}"; then echo "ok   $1"; elif [[ $1 == KNOWN* ]]; then echo "known $1"; else echo "FAIL $1"; FAIL=1; fi; }
th on; sleep 0.6
check "on: toolbar on HEADLESS-1" toolbar HEADLESS-1
cur 600 400; sleep 0.2; cur 610 410; sleep 0.4
key -s 150 -P Alt_L -s 1500 -p Alt_L & KEY=$!
sleep 0.6; check "Alt held before any plug: hand hidden" eval "! hand 1 600 400"
wait $KEY; sleep 0.3
swaymsg -q create_output; sleep 1
check "plug while on: toolbar on HEADLESS-2" toolbar HEADLESS-2
swaymsg -q output HEADLESS-2 unplug; sleep 0.8
cur 600 400; sleep 0.2; cur 610 410; sleep 0.5
check "KNOWN(queue) unplug after an earlier Alt hold: hand still draws" hand 1 600 400
th status >/dev/null; check "unplug: still on" test $? = 0
key -s 150 -P Alt_L -s 2500 -p Alt_L & KEY=$!
sleep 0.6; check "KNOWN(queue) Alt held after a plug cycle: hand hidden" eval "! hand 1 600 400"
swaymsg -q create_output; sleep 1
check "KNOWN(queue) plug while Alt held: new output has no toolbar" eval '! toolbar HEADLESS-3'
wait $KEY; sleep 0.3
swaymsg -q output HEADLESS-3 unplug; sleep 0.5
th off; sleep 0.4
check "off: no toolbar" eval '! toolbar HEADLESS-1'
swaymsg -q create_output; sleep 0.5; th on; sleep 0.6
check "on with two: toolbar on HEADLESS-1" toolbar HEADLESS-1
check "on with two: toolbar on HEADLESS-4" toolbar HEADLESS-4
swaymsg -q output HEADLESS-1 unplug; sleep 0.8
th status >/dev/null; check "primary unplugged: still on" test $? = 0
swaymsg -q output HEADLESS-4 unplug; sleep 0.8
th status >/dev/null; check "all unplugged: off" test $? = 1
th quit
echo "RESULT=$FAIL"
