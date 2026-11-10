#!/usr/bin/env bash
# usage: tools/e2e/run-gnome.sh <scenario.sh>
# Private headless GNOME Shell with its own D-Bus, dconf and XDG dirs. Never touches the user's session.
set -u
T=$(dirname "$(readlink -f "$0")")
P=$(dirname "$(dirname "$T")")
export E=$T/out/gnome
rm -rf $E && mkdir -p $E/{config,data,cache,state,runtime} && chmod 700 $E/runtime
mkdir -p $E/data/gnome-shell/extensions
cp -r $T/gnome/tinyhand-test@local $E/data/gnome-shell/extensions/
[ -n "${NO_INSTALL:-}" ] || cp -r $P/gnome-extension/tinyhand@xdinterface.github.io $E/data/gnome-shell/extensions/
cat > $E/inner.sh <<'INNER'
set -u
gsettings set org.gnome.shell disable-user-extensions false
gsettings set org.gnome.shell disable-extension-version-validation true
gsettings set org.gnome.shell enabled-extensions "$( [ -n "${NO_HELPER:-}" ] && echo "['tinyhand-test@local']" || echo "['tinyhand@xdinterface.github.io', 'tinyhand-test@local']")"
gsettings set org.gnome.desktop.interface enable-animations false
gsettings set org.gnome.shell welcome-dialog-last-shown-version '999'
gnome-shell --headless --virtual-monitor ${RES:-1440x960} --wayland-display gnome-test --no-x11 > $E/shell.log 2>&1 &
for i in $(seq 100); do gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell --method org.freedesktop.DBus.Peer.Ping >/dev/null 2>&1 && break; sleep 0.2; done
sleep 1
gnome-extensions enable tinyhand-test@local; [ -n "${NO_HELPER:-}${NO_INSTALL:-}" ] || gnome-extensions enable tinyhand@xdinterface.github.io
for i in $(seq 100); do gdbus call --session --dest org.tinyhand.Test --object-path /org/tinyhand/Test --method org.freedesktop.DBus.Peer.Ping >/dev/null 2>&1 && break; sleep 0.2; done
export WAYLAND_DISPLAY=gnome-test GDK_BACKEND=wayland GTK_A11Y=none GDK_DEBUG=no-portals ADW_DISABLE_PORTAL=1
drv() { gdbus call --session --dest org.tinyhand.Test --object-path /org/tinyhand/Test --method org.tinyhand.Test.$1 "${@:2}" >/dev/null; }
th()   { timeout ${TH_TIMEOUT:-10} $BIN "$@"; }
cur()  { drv Move "$1.0" "$2.0"; sleep 0.05; }
down() { drv Button true; sleep 0.05; }
up()   { drv Button false; sleep 0.05; }
key()  { drv Key "$1" true; sleep 0.05; drv Key "$1" false; sleep 0.05; }
keydown() { drv Key "$1" true; sleep 0.05; }
keyup()   { drv Key "$1" false; sleep 0.05; }
shot() { drv Shot "$E/$1.png"; sleep 0.4; echo "shot $1"; }
windows() { gdbus call --session --dest org.tinyhand.Test --object-path /org/tinyhand/Test --method org.tinyhand.Test.Windows | sed "s/^('//; s/',)$//"; }
gsettings set org.gnome.desktop.interface enable-hot-corners false
drv Move 720.0 480.0; drv Overview false; sleep 0.8
source $SCENARIO
INNER
unset LD_LIBRARY_PATH WAYLAND_DISPLAY DISPLAY
DATA_DIRS=$(tr ':' '\n' <<< "${XDG_DATA_DIRS:-/run/current-system/sw/share}" | while read -r d; do [ -e "$d/gnome-shell/extensions/tinyhand@xdinterface.github.io" ] || echo "$d"; done | paste -sd:)
env -i PATH="$PATH" HOME="$HOME" LANG=C.UTF-8 \
  XDG_CONFIG_HOME=$E/config XDG_DATA_HOME=$E/data XDG_CACHE_HOME=$E/cache XDG_STATE_HOME=$E/state XDG_RUNTIME_DIR=$E/runtime \
  XDG_DATA_DIRS="$DATA_DIRS" \
  TINYHAND_CONFIG=$E/tinyhand.toml TINYHAND_DATA_DIR=$P/assets \
  NO_HELPER=${NO_HELPER:-} NO_INSTALL=${NO_INSTALL:-} BIN=${BIN:-$P/target/debug/tinyhand} E=$E SCENARIO=$(readlink -f "$1") \
  timeout ${RUN_TIMEOUT:-90} dbus-run-session -- bash $E/inner.sh
