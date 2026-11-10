#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
arch=$(dpkg --print-architecture)
work=target/deb
stage=$work/tinyhand
rm -rf "$work"
mkdir -p "$stage/DEBIAN" "$work/debian"
make install DESTDIR="$stage" PREFIX=/usr
install -Dm644 LICENSE "$stage/usr/share/doc/tinyhand/copyright"
printf 'Source: tinyhand\n\nPackage: tinyhand\nArchitecture: any\n' > "$work/debian/control"
depends=$(cd "$work" && dpkg-shlibdeps -O tinyhand/usr/bin/tinyhand | sed 's/^shlibs:Depends=//')
cat > "$stage/DEBIAN/control" <<CONTROL
Package: tinyhand
Version: $version
Architecture: $arch
Maintainer: xdinterface <xdinterface@users.noreply.github.com>
Depends: $depends
Section: utils
Priority: optional
Homepage: https://github.com/xdinterface/tinyhand-linux
Description: A tiny hand on a stick that points at things on your screen
 A pointer overlay for presentations and screen sharing: tap on click,
 drag to draw. Linux port of TinyHand.
CONTROL
dpkg-deb --root-owner-group --build "$stage" "$work/tinyhand-$arch.deb"
