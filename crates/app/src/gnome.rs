use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use std::fs;
use tinyhand_core::paths;

const UUID: &str = "tinyhand@xdinterface.github.io";
const SHELL_EXTENSIONS: &str = "org.gnome.Shell.Extensions";

pub fn enable_extension(app: &gtk::Application) {
    let marker = paths::state_dir().join("gnome-extension-enabled");
    if marker.exists() {
        return;
    }
    let Some(bus) = app.dbus_connection() else {
        return;
    };
    let app = app.downgrade();
    bus.call(
        Some(SHELL_EXTENSIONS),
        "/org/gnome/Shell/Extensions",
        SHELL_EXTENSIONS,
        "EnableExtension",
        Some(&(UUID,).to_variant()),
        Some(glib::VariantTy::new("(b)").expect("valid type")),
        gio::DBusCallFlags::NONE,
        5000,
        gio::Cancellable::NONE,
        move |reply| match reply.map(|value| value.get::<(bool,)>()) {
            Ok(Some((true,))) => {
                if let Some(dir) = marker.parent() {
                    let _ = fs::create_dir_all(dir);
                }
                let _ = fs::write(&marker, b"");
            }
            Ok(_) => {
                let note = gio::Notification::new("Log out and back in to finish setting up TinyHand");
                note.set_body(Some("GNOME only picks up the TinyHand helper extension when you log in."));
                if let Some(app) = app.upgrade() {
                    app.send_notification(Some("gnome-extension"), &note);
                }
            }
            Err(_) => {}
        },
    );
}
