pub const HELP: &str = "\
TinyHand: a tiny hand on a stick that points at things on your screen.

Usage: tinyhand [COMMAND]

With no command, TinyHand starts and waits in the background.

Commands:
  toggle                     Turn the pointer on or off
  on, off                    Turn the pointer on or off
  clear                      Clear the drawing
  pick <id>                  Choose a pointer (see `tinyhand list`)
  sound [on|off]             Turn the tap sound on or off; no argument toggles
  status                     Print `on` or `off`
  list                       List pointer ids
  settings                   Open the settings window
  make-hand <image> [name]   Add your own hand from an image
  quit                       Stop TinyHand

While the pointer is on:
  Click         Tap (nothing underneath is clicked)
  Drag          Draw on the screen
  Hold Alt      Click for real
  Esc           Clear everything and turn the pointer off
  C             Clear the drawing
  M             Mute or unmute the tap sound
  1-9           Pick a pointer from the toolbar

Config: ~/.config/tinyhand/config.toml
";

pub const REMOTE: [&str; 8] = ["toggle", "on", "off", "clear", "pick", "sound", "status", "quit"];
