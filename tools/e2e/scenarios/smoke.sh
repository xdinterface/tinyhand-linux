rm -rf $XDG_STATE_HOME $XDG_DATA_HOME; printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
th status; echo "status-before=$?"
TH_TIMEOUT=40 th > $E/th.log 2>&1 &
sleep 1
th status; echo "status-resident=$?"
cur set 700 400
th on; sleep 0.6
th status; echo "status-on=$?"
swaymsg -t get_tree >/dev/null; swaymsg -r -t get_outputs | head -c 0
cur move 5 5; sleep 0.5
shot s1-on
th off; sleep 0.3; th status; echo "status-off=$?"
shot s1-off
th quit; sleep 0.3; th status; echo "status-after-quit=$?"
cat $E/th.log
