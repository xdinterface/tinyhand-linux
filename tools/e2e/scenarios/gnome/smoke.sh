printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
grep -iE "tinyhand|JS ERROR|extension" $E/shell.log | head -5
TH_TIMEOUT=60 th > $E/th.log 2>&1 &
sleep 1.5
cur 600 400
th on; sleep 1.5
cur 610 410; sleep 0.5
windows
shot on
th status; echo "status=$?"
th quit
grep -v "^$" $E/th.log | head -5
grep -iE "tinyhand|JS ERROR" $E/shell.log | head -8
