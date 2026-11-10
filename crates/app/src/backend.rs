use gtk::gdk;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode as LayerKeyboard, Layer, LayerShell};
use tinyhand_core::catalog::KeyboardMode;

pub const WINDOW_TITLE: &str = "TinyHand Overlay";
const PASS_SUFFIX: &str = " [pass]";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    Layer,
    Window,
}

pub fn detect() -> Backend {
    match std::env::var("TINYHAND_BACKEND").as_deref() {
        Ok("layer") => Backend::Layer,
        Ok("window") => Backend::Window,
        _ if gtk4_layer_shell::is_supported() => Backend::Layer,
        _ => Backend::Window,
    }
}

pub fn place(window: &gtk::Window, monitor: &gdk::Monitor, backend: Backend, keyboard: KeyboardMode) {
    window.set_decorated(false);
    match backend {
        Backend::Layer => {
            window.init_layer_shell();
            window.set_namespace(Some("tinyhand"));
            window.set_layer(Layer::Overlay);
            window.set_monitor(Some(monitor));
            window.set_exclusive_zone(0);
            for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                window.set_anchor(edge, true);
            }
            set_keyboard(window, backend, keyboard);
        }
        Backend::Window => {
            let connector = monitor.connector().unwrap_or_default();
            window.set_title(Some(&format!("{WINDOW_TITLE} {connector}")));
            window.maximize();
        }
    }
}

pub fn set_passing(window: &gtk::Window, monitor: &gdk::Monitor, backend: Backend, passing: bool) {
    if backend == Backend::Window {
        let connector = monitor.connector().unwrap_or_default();
        let suffix = if passing { PASS_SUFFIX } else { "" };
        window.set_title(Some(&format!("{WINDOW_TITLE} {connector}{suffix}")));
    }
}

pub fn set_keyboard(window: &gtk::Window, backend: Backend, keyboard: KeyboardMode) {
    if backend == Backend::Layer {
        window.set_keyboard_mode(match keyboard {
            KeyboardMode::Grab => LayerKeyboard::Exclusive,
            KeyboardMode::Pass => LayerKeyboard::None,
        });
    }
}
