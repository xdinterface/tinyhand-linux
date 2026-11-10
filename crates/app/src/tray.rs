use crate::app::APP_ID;
use gtk::gio::prelude::*;
use gtk::{gio, glib};
use ksni::blocking::TrayMethods;
use ksni::menu::{CheckmarkItem, MenuItem, StandardItem};
use tinyhand_core::paths;

fn send(action: &'static str) {
    glib::MainContext::default().invoke(move || {
        if let Some(app) = gio::Application::default() {
            app.activate_action(action, None);
        }
    });
}

struct Icon {
    active: bool,
    sound: bool,
    icons: String,
}

impl ksni::Tray for Icon {
    fn id(&self) -> String {
        APP_ID.into()
    }

    fn title(&self) -> String {
        "TinyHand".into()
    }

    fn icon_name(&self) -> String {
        if self.active { "tinyhand-on-symbolic" } else { "tinyhand-off-symbolic" }.into()
    }

    fn icon_theme_path(&self) -> String {
        self.icons.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: if self.active { "TinyHand is on" } else { "TinyHand" }.into(),
            description: "Click to turn the pointer on or off".into(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        send("toggle");
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let item = |label: &str, action: &'static str| {
            MenuItem::Standard(StandardItem {
                label: label.into(),
                activate: Box::new(move |_| send(action)),
                ..Default::default()
            })
        };
        vec![
            item(if self.active { "Turn Pointer Off" } else { "Turn Pointer On" }, "toggle"),
            item("Clear Drawing", "clear"),
            MenuItem::Checkmark(CheckmarkItem {
                label: "Tap Sound".into(),
                checked: self.sound,
                activate: Box::new(|_| send("sound")),
                ..Default::default()
            }),
            MenuItem::Separator,
            item("Settings…", "settings"),
            MenuItem::Separator,
            item("Quit TinyHand", "quit"),
        ]
    }
}

pub struct Tray(ksni::blocking::Handle<Icon>);

impl Tray {
    pub fn start(active: bool, sound: bool) -> Option<Tray> {
        let icons = paths::find_data("icons")?.to_string_lossy().into_owned();
        Icon { active, sound, icons }.assume_sni_available(true).spawn().ok().map(Tray)
    }

    pub fn set(&self, active: bool, sound: bool) {
        self.0.update(|icon| {
            icon.active = active;
            icon.sound = sound;
        });
    }

    pub fn stop(self) {
        self.0.shutdown();
    }
}
