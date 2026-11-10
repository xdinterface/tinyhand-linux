PREFIX ?= /usr/local
DESTDIR ?=
CARGO ?= cargo

bindir = $(DESTDIR)$(PREFIX)/bin
datadir = $(DESTDIR)$(PREFIX)/share
extension = tinyhand@xdinterface.github.io

.PHONY: build install install-data uninstall

build:
	$(CARGO) build --release --locked -p tinyhand

install: install-data
	install -Dm755 target/release/tinyhand $(bindir)/tinyhand

install-data:
	install -Dm644 -t $(datadir)/tinyhand/hands assets/hands/*.png
	install -Dm644 -t $(datadir)/tinyhand/sounds assets/sounds/*.wav
	install -Dm644 -t $(datadir)/tinyhand/icons assets/icons/tinyhand-*-symbolic.svg
	install -Dm644 -t $(datadir)/icons/hicolor/symbolic/apps assets/icons/tinyhand-*-symbolic.svg
	install -Dm644 -t $(datadir)/icons/hicolor/scalable/apps assets/icons/io.github.xdinterface.TinyHand.svg
	install -Dm644 -t $(datadir)/applications data/*.desktop
	install -Dm644 -t $(datadir)/gnome-shell/extensions/$(extension) gnome-extension/$(extension)/*
	install -d $(datadir)/dbus-1/services
	sed 's|@bindir@|$(PREFIX)/bin|' data/io.github.xdinterface.TinyHand.service.in \
		> $(datadir)/dbus-1/services/io.github.xdinterface.TinyHand.service

uninstall:
	rm -f $(bindir)/tinyhand
	rm -rf $(datadir)/tinyhand $(datadir)/gnome-shell/extensions/$(extension)
	rm -f $(datadir)/applications/io.github.xdinterface.TinyHand.desktop
	rm -f $(datadir)/applications/io.github.xdinterface.TinyHand.Settings.desktop
	rm -f $(datadir)/icons/hicolor/symbolic/apps/tinyhand-on-symbolic.svg
	rm -f $(datadir)/icons/hicolor/symbolic/apps/tinyhand-off-symbolic.svg
	rm -f $(datadir)/icons/hicolor/scalable/apps/io.github.xdinterface.TinyHand.svg
	rm -f $(datadir)/dbus-1/services/io.github.xdinterface.TinyHand.service
