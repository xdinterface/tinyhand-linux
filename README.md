<img src="./.github/logo.png" width="80" />

# TinyHand for Linux

[![Platform: Linux](https://img.shields.io/badge/platform-Linux%20(Wayland)-lightgrey)](#does-it-work-on-my-desktop)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

A tiny hand on a stick that points at things on your Linux screen.

<img src="./.github/banner.jpg" alt="TinyHand: a tiny hand for your Linux desktop" />

A Linux port of [TinyHand](https://tinyhand.app) by Kamran Ahmed, with the same 36 hands and tap sounds.

## Get started

1. [Install](#1-install) the package for your distro.
2. [Set up your desktop](#2-set-up-your-desktop). On GNOME there is nothing to configure; on other desktops you add two shortcuts.
3. Press **Ctrl+Alt+H** and [point at things](#3-use-it).

## Does it work on my desktop?

TinyHand needs a **Wayland** session. X11 sessions are not supported.

| Desktop | Works | What you set up |
|---|---|---|
| GNOME 47 or later | Yes, through a helper extension that comes with the package | Nothing. Shortcuts work out of the box. |
| niri | Yes | Two shortcut lines in your niri config |
| Hyprland | Yes | Two shortcut lines in your Hyprland config |
| Sway | Yes | Two shortcut lines in your Sway config |
| KDE Plasma 6 | Should work, not tested yet | Two custom shortcuts in System Settings |
| COSMIC and other desktops with layer-shell | Should work, not tested yet | Two shortcuts that run a command |

Tested so far on niri, Sway and GNOME 50.

## 1. Install

### Ubuntu and Debian

For Ubuntu 25.04 or later and Debian 13 or later. Ubuntu 24.04 is not supported, because it has no gtk4-layer-shell package.

```bash
curl -LO https://github.com/xdinterface/tinyhand-linux/releases/latest/download/tinyhand-amd64.deb
sudo apt install ./tinyhand-amd64.deb
```

### Fedora

For Fedora 42 or later:

```bash
sudo dnf install https://github.com/xdinterface/tinyhand-linux/releases/latest/download/tinyhand-x86_64.rpm
```

### Arch Linux

Build and install the package with the PKGBUILD from this repository:

```bash
curl -LO https://raw.githubusercontent.com/xdinterface/tinyhand-linux/main/packaging/arch/PKGBUILD
makepkg -si
```

### NixOS and Nix

Try it without installing:

```bash
nix run github:xdinterface/tinyhand-linux -- toggle
```

Install it into your profile:

```bash
nix profile install github:xdinterface/tinyhand-linux
```

Or with Home Manager, which also starts it with your session:

```nix
# flake.nix
inputs.tinyhand.url = "github:xdinterface/tinyhand-linux";

# home.nix
imports = [inputs.tinyhand.homeModules.default];
programs.tinyhand.enable = true;
```

Two more options: `programs.tinyhand.settings` writes the config file for you (it then becomes read-only, so the toolbar and settings window can't save changes), and `programs.tinyhand.systemd.enable` (on by default) starts TinyHand with your session.

### Any other distro, from source

You need Rust 1.92 or later ([rustup](https://rustup.rs) if your distro's is older), GTK 4.14 or later, libadwaita 1.5 or later, gtk4-layer-shell and the ALSA headers:

```bash
# Debian, Ubuntu
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libgtk4-layer-shell-dev libasound2-dev
# Fedora
sudo dnf install cargo gcc make gtk4-devel libadwaita-devel gtk4-layer-shell-devel alsa-lib-devel
# Arch
sudo pacman -S --needed base-devel cargo gtk4 libadwaita gtk4-layer-shell alsa-lib
```

Then:

```bash
git clone https://github.com/xdinterface/tinyhand-linux
cd tinyhand-linux
make
sudo make install
```

This installs to `/usr/local`. `sudo make uninstall` removes it again.

There is no AppImage or Flatpak. To update, install the newer package the same way.

## 2. Set up your desktop

TinyHand is driven by two commands, and your desktop decides which keys run them:

| Command | Suggested key |
|---|---|
| `tinyhand toggle` | Ctrl+Alt+H |
| `tinyhand clear` | Ctrl+Alt+X |

### GNOME

1. Log out and back in after installing, so GNOME picks up the helper extension.
2. Open **TinyHand** from the app grid once. This switches the extension on.

From then on Ctrl+Alt+H and Ctrl+Alt+X work, also after a restart. To use other keys, set them in the [config file](#settings):

```toml
toggle-shortcut = "ctrl+alt+h"   # or "off"
clear-shortcut = "ctrl+alt+x"
```

GNOME has no tray by default, so the tray icon only shows if you use an AppIndicator extension (Ubuntu has one built in).

### niri

Add to `~/.config/niri/config.kdl`:

```kdl
binds {
    Ctrl+Alt+H { spawn "tinyhand" "toggle"; }
    Ctrl+Alt+X { spawn "tinyhand" "clear"; }
}
```

### Hyprland

Add to `~/.config/hypr/hyprland.conf`:

```ini
bind = CTRL ALT, H, exec, tinyhand toggle
bind = CTRL ALT, X, exec, tinyhand clear
```

### Sway

Add to `~/.config/sway/config`:

```
bindsym Ctrl+Alt+h exec tinyhand toggle
bindsym Ctrl+Alt+x exec tinyhand clear
```

### KDE Plasma and others

Add two custom shortcuts that run the commands above. In KDE: System Settings → Keyboard → Shortcuts → Add New → Command or Script.

### Start at login (optional)

The first `tinyhand toggle` starts TinyHand, so this is only needed if you want the tray icon right after login. Turn on Settings → General → Open TinyHand when you log in. The Home Manager module already does this.

## 3. Use it

| | |
|---|---|
| Ctrl+Alt+H, `tinyhand toggle`, or click the tray icon | Turn the pointer on or off |
| Click | Tap (nothing underneath gets clicked) |
| Hold and drag | Draw on the screen |
| Ctrl+Alt+X, `tinyhand clear`, or C | Clear the drawing |
| Hold Alt | Click for real |
| Esc | Clear everything and turn the pointer off |
| 1–9, M | Pick a pointer from the toolbar, mute |

While the pointer is on, a small toolbar at the bottom of the screen lets you pick one of 36 hands or a laser dot, change the ink colour, mute the tap sound, or exit.

More commands: `tinyhand on`, `off`, `pick <id>`, `list`, `sound [on|off]`, `status`, `settings`, `make-hand <image> [name]`, `quit`. See `tinyhand --help`.

## Settings

Open them with `tinyhand settings` or from the tray menu. Everything is also in a plain config file, `~/.config/tinyhand/config.toml`, which you can keep in your dotfiles. TinyHand reloads it when it changes, and every key is optional:

```toml
pointer = "lavender"      # see `tinyhand list`
size = 0.13               # 0.08 to 0.20 of the screen height
shadow = true
toolbar = ["lavender", "glove", "coral", "plush", "robot", "laser"]   # up to 8
ink-color = "match"       # match, red, orange, yellow, green, blue, purple, white, black
ink-fade = 3              # seconds; 0 keeps lines until cleared
ink-button = true
tap-sound = "thock"       # thock, click, pop, wood, boop, squeak
sound = true
laser-sound = false
interact-key = "alt"      # alt, ctrl, super, shift, off
keyboard = "grab"         # see below
tray = true
toggle-shortcut = "ctrl+alt+h"   # GNOME only
clear-shortcut = "ctrl+alt+x"    # GNOME only
```

`keyboard = "grab"` gives TinyHand the keyboard while the pointer is on, so Esc, C, M and 1–9 work. Your other apps count as unfocused meanwhile, and some draw themselves dimmer. `keyboard = "pass"` leaves the keyboard with your apps: they stay focused and you can type, but those keys no longer reach TinyHand, so use the toolbar and your two shortcuts instead.

## Make your own hand

Settings → Pointers → Make your own… writes a prompt for ChatGPT or another image tool, checks the image you get back and puts it on a stick. From the terminal: `tinyhand make-hand hand.png "My hand"`.

## License

MIT. See [LICENSE](LICENSE). Hands, sounds and the logo are from the original [TinyHand](https://github.com/nilbuild/tinyhand) by Kamran Ahmed, also MIT.
