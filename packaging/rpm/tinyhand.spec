%global debug_package %{nil}

Name:           tinyhand
Version:        %{?tinyhand_version}%{!?tinyhand_version:0}
Release:        1
Summary:        A tiny hand on a stick that points at things on your screen
License:        MIT
URL:            https://github.com/xdinterface/tinyhand-linux

BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  pkgconfig(gtk4)
BuildRequires:  pkgconfig(libadwaita-1)
BuildRequires:  pkgconfig(gtk4-layer-shell-0)
BuildRequires:  pkgconfig(alsa)

%description
A pointer overlay for presentations and screen sharing: tap on click,
drag to draw. Linux port of TinyHand.

%build
make build

%install
make install DESTDIR=%{buildroot} PREFIX=%{_prefix}

%files
%license LICENSE
%{_bindir}/tinyhand
%{_datadir}/tinyhand/
%{_datadir}/applications/io.github.xdinterface.TinyHand.desktop
%{_datadir}/applications/io.github.xdinterface.TinyHand.Settings.desktop
%{_datadir}/icons/hicolor/scalable/apps/io.github.xdinterface.TinyHand.svg
%{_datadir}/icons/hicolor/symbolic/apps/tinyhand-on-symbolic.svg
%{_datadir}/icons/hicolor/symbolic/apps/tinyhand-off-symbolic.svg
%{_datadir}/gnome-shell/extensions/tinyhand@xdinterface.github.io/
%{_datadir}/dbus-1/services/io.github.xdinterface.TinyHand.service
