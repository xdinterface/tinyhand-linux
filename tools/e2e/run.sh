#!/usr/bin/env bash
# usage: tools/e2e/run.sh <scenario.sh>
set -u
T=$(dirname "$(readlink -f "$0")")
P=$(dirname "$(dirname "$T")")
export E=$T/out
mkdir -p $E
unset LD_LIBRARY_PATH
export TINYHAND_CONFIG=$E/config.toml XDG_STATE_HOME=$E/state XDG_DATA_HOME=$E/data TINYHAND_DATA_DIR=$P/assets
export GTK_A11Y=none GDK_DEBUG=no-portals ADW_DISABLE_PORTAL=1
export BIN=${BIN:-$P/target/debug/tinyhand}
unset WAYLAND_DISPLAY DISPLAY SWAYSOCK NIRI_SOCKET
cat > $E/sway.conf <<CONF
output HEADLESS-1 resolution ${RES:-1440x960} scale ${SCALE:-1} bg #dfe3dc solid_color
${EXTRA_SWAY:-}
CONF
export WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=${WLR_RENDERER:-gles2}
before=$(ls $XDG_RUNTIME_DIR | grep -E '^wayland-[0-9]+$' | sort)
sway -c $E/sway.conf >$E/sway.log 2>&1 &
SWAY=$!
for i in $(seq 50); do
  now=$(ls $XDG_RUNTIME_DIR | grep -E '^wayland-[0-9]+$' | sort)
  new=$(comm -13 <(echo "$before") <(echo "$now") | head -1)
  [ -n "$new" ] && break; sleep 0.1
done
export WAYLAND_DISPLAY=$new
export SWAYSOCK=$(ls $XDG_RUNTIME_DIR/sway-ipc.*.$SWAY.sock 2>/dev/null | head -1)
th()   { timeout ${TH_TIMEOUT:-10} $BIN "$@"; }
shot() { grim "$E/$1.png" && echo "shot $1"; }
cur()  { echo "abs $1 $2" >&3; sleep 0.05; }
down() { echo "down ${1:-272}" >&3; sleep 0.05; }
up()   { echo "up ${1:-272}" >&3; sleep 0.05; }
drag() { local x0=$1 y0=$2 x1=$3 y1=$4 n=${5:-12}; cur $x0 $y0; down; for i in $(seq $n); do cur $((x0+(x1-x0)*i/n)) $((y0+(y1-y0)*i/n)); done; up; }
key()  { wtype "$@"; }
export -f th shot cur key down up drag
rm -f $E/fifo; mkfifo $E/fifo
IFS=x read W H <<< "${RES:-1440x960}"
$T/vptr/target/release/vptr $W $H < $E/fifo 2>$E/vptr.log &
exec 3> $E/fifo
timeout ${RUN_TIMEOUT:-60} dbus-run-session -- bash -c "source $1"
kill $SWAY 2>/dev/null; wait $SWAY 2>/dev/null
