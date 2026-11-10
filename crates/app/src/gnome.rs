use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use std::cell::Cell;

const UUID: &str = "tinyhand@xdinterface.github.io";
const SHELL_EXTENSIONS: &str = "org.gnome.Shell.Extensions";
const SESSION_MANAGER: &str = "org.gnome.SessionManager";
const STATE_ACTIVE: f64 = 1.0;
const TIMEOUT_MS: i32 = 2000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Helper {
    Ready,
    NotLoaded,
    Failed,
    NoShell,
}

fn shell(bus: &gio::DBusConnection, method: &str) -> Option<glib::Variant> {
    bus.call_sync(
        Some(SHELL_EXTENSIONS),
        "/org/gnome/Shell/Extensions",
        SHELL_EXTENSIONS,
        method,
        Some(&(UUID,).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        TIMEOUT_MS,
        gio::Cancellable::NONE,
    )
    .ok()
}

pub fn prepare(app: &gtk::Application) -> Helper {
    let Some(bus) = app.dbus_connection() else {
        return Helper::NoShell;
    };
    if shell(&bus, "EnableExtension").is_none() {
        return Helper::NoShell;
    }
    let info = shell(&bus, "GetExtensionInfo").and_then(|reply| reply.child_value(0).get::<glib::VariantDict>());
    match info.and_then(|info| info.lookup::<f64>("state").ok().flatten()) {
        Some(state) if state == STATE_ACTIVE => Helper::Ready,
        Some(_) => Helper::Failed,
        None => Helper::NotLoaded,
    }
}

thread_local! {
    static ASKING: Cell<bool> = const { Cell::new(false) };
}

pub fn explain(app: &gtk::Application, problem: Helper) {
    if ASKING.replace(true) {
        return;
    }
    let relogin = problem == Helper::NotLoaded;
    let dialog = gtk::AlertDialog::builder()
        .modal(false)
        .message(if relogin {
            "Log out and back in to finish setting up TinyHand"
        } else {
            "TinyHand's GNOME extension could not start"
        })
        .detail(if relogin {
            "TinyHand needs its helper extension on GNOME, and GNOME only picks up a new extension when you log in. After that, TinyHand works right away."
        } else {
            "This GNOME version may not be supported yet. Run `gnome-extensions info tinyhand@xdinterface.github.io` to see the reason."
        })
        .build();
    if relogin {
        dialog.set_buttons(&["Later", "Log Out…"]);
        dialog.set_cancel_button(0);
        dialog.set_default_button(1);
    }
    let hold = app.hold();
    let bus = app.dbus_connection();
    dialog.choose(gtk::Window::NONE, gio::Cancellable::NONE, move |choice| {
        ASKING.set(false);
        drop(hold);
        if relogin && choice == Ok(1)
            && let Some(bus) = bus
        {
            bus.call(
                Some(SESSION_MANAGER),
                "/org/gnome/SessionManager",
                SESSION_MANAGER,
                "Logout",
                Some(&(0u32,).to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                -1,
                gio::Cancellable::NONE,
                |_| {},
            );
        }
    });
}
