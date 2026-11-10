printf 'sound = false\ntray = false\n' > $TINYHAND_CONFIG
$BIN > $E/th.log 2>&1 &
PID=$(pgrep -f "tinyhand-wrapped$" | head -1); PID=${PID:-$!}
sleep 2
ticks() { awk '{print $14+$15}' /proc/$1/stat; }
rss() { awk '/VmRSS/{print $2/1024 " MB"}' /proc/$1/status; }
measure() { local a=$(ticks $PID); sleep 3; local b=$(ticks $PID); echo "$1: cpu $(( (b-a) * 100 / 300 ))% rss $(rss $PID)"; }
measure off
th on; sleep 0.5; cur 600 400; sleep 2
measure on-still
( for i in $(seq 300); do cur $((300 + i * 2)) $((300 + (i % 50) * 4)); done ) &
M=$!
measure on-moving
wait $M
th off; sleep 1
measure off-again
th quit
