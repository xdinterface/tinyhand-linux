use super::Ctx;
use crate::canvas::{self, SnapshotArea};
use crate::hands;
use adw::prelude::*;
use gtk::glib;
use std::path::PathBuf;
use std::rc::Rc;
use tinyhand_core::arm::{self, ArmState};
use tinyhand_core::catalog::{InkChoice, InteractKey, KeyboardMode, TapSound, LASER};
use tinyhand_core::config::{SIZE_MAX, SIZE_MIN};
use tinyhand_core::paths;

const FADES: [(&str, f64); 4] =
    [("After 3 seconds", 3.0), ("After 6 seconds", 6.0), ("After 10 seconds", 10.0), ("Keep until cleared", 0.0)];
const PREVIEW_HEIGHT: f64 = 720.0;
const AUTOSTART: &str = "io.github.xdinterface.TinyHand.desktop";

fn page() -> adw::PreferencesPage {
    adw::PreferencesPage::new()
}

fn group(title: &str, description: Option<&str>) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title(title);
    if let Some(text) = description {
        group.set_description(Some(text));
    }
    group
}

fn switch(ctx: &Rc<Ctx>, title: &str, subtitle: Option<&str>, get: fn(&Ctx) -> bool, set: fn(&Rc<Ctx>, bool)) -> adw::SwitchRow {
    let row = adw::SwitchRow::new();
    row.set_title(title);
    if let Some(text) = subtitle {
        row.set_subtitle(text);
    }
    row.set_active(get(ctx));
    let weak = Rc::downgrade(ctx);
    row.connect_active_notify(move |row| {
        if let Some(ctx) = weak.upgrade()
            && get(&ctx) != row.is_active()
        {
            set(&ctx, row.is_active());
        }
    });
    let (weak, sync) = (Rc::downgrade(ctx), row.downgrade());
    ctx.on_rebuild(move || {
        if let (Some(ctx), Some(row)) = (weak.upgrade(), sync.upgrade()) {
            row.set_active(get(&ctx));
        }
    });
    row
}

fn combo(ctx: &Rc<Ctx>, title: &str, items: &[&str], get: fn(&Ctx) -> usize, set: fn(&Rc<Ctx>, usize)) -> adw::ComboRow {
    let row = adw::ComboRow::new();
    row.set_title(title);
    row.set_model(Some(&gtk::StringList::new(items)));
    row.set_selected(get(ctx) as u32);
    let weak = Rc::downgrade(ctx);
    row.connect_selected_notify(move |row| {
        if let Some(ctx) = weak.upgrade()
            && get(&ctx) != row.selected() as usize
        {
            set(&ctx, row.selected() as usize);
        }
    });
    let (weak, sync) = (Rc::downgrade(ctx), row.downgrade());
    ctx.on_rebuild(move || {
        if let (Some(ctx), Some(row)) = (weak.upgrade(), sync.upgrade()) {
            row.set_selected(get(&ctx) as u32);
        }
    });
    row
}

fn command_row(title: &str, command: &'static str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(title);
    row.set_subtitle(command);
    row.add_css_class("property");
    let copy = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy.set_valign(gtk::Align::Center);
    copy.add_css_class("flat");
    copy.set_tooltip_text(Some("Copy"));
    copy.connect_clicked(move |button| button.clipboard().set_text(command));
    row.add_suffix(&copy);
    row
}

fn autostart_file() -> PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
    config.join("autostart").join(AUTOSTART)
}

fn set_autostart(ctx: &Rc<Ctx>, on: bool) {
    let file = autostart_file();
    let result = if on {
        let text = "[Desktop Entry]\nType=Application\nName=TinyHand\nExec=tinyhand\nIcon=io.github.xdinterface.TinyHand\nX-GNOME-Autostart-enabled=true\nNoDisplay=true\n";
        std::fs::create_dir_all(file.parent().unwrap_or(&file)).and_then(|()| std::fs::write(&file, text))
    } else {
        std::fs::remove_file(&file).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) })
    };
    if let Err(err) = result {
        ctx.toast(&format!("Couldn't change the login item: {err}"));
    }
}

pub fn general(ctx: &Rc<Ctx>) -> adw::PreferencesPage {
    let page = page();
    let shortcuts = group(
        "Shortcuts",
        Some("Bind these commands to keys in your desktop's keyboard settings. Ctrl+Alt+H and Ctrl+Alt+X match the Mac app."),
    );
    shortcuts.add(&command_row("Turn the pointer on or off", "tinyhand toggle"));
    shortcuts.add(&command_row("Clear the drawing", "tinyhand clear"));
    page.add(&shortcuts);

    let clicking = group("Clicking for real", None);
    let keys: Vec<&str> = InteractKey::ALL.iter().map(|k| k.title()).collect();
    let hold = combo(
        ctx,
        "Hold to use the app",
        &keys,
        |ctx| InteractKey::ALL.iter().position(|k| *k == ctx.config.borrow().interact_key).unwrap_or(0),
        |ctx, i| {
            let key = InteractKey::ALL[i];
            ctx.set("interact-key", key.as_str(), |c| c.interact_key = key);
        },
    );
    hold.set_subtitle("Clicks and drags reach the app while you hold it");
    clicking.add(&hold);
    clicking.add(&switch(
        ctx,
        "Type in apps while the pointer is on",
        Some("Esc and the hold key then stop working; turn the pointer off with your shortcut"),
        |ctx| ctx.config.borrow().keyboard == KeyboardMode::Pass,
        |ctx, on| {
            let mode = if on { KeyboardMode::Pass } else { KeyboardMode::Grab };
            ctx.set("keyboard", mode.as_str(), |c| c.keyboard = mode);
        },
    ));
    page.add(&clicking);

    let startup = group("Startup", None);
    startup.add(&switch(ctx, "Open TinyHand when you log in", None, |_| autostart_file().exists(), set_autostart));
    startup.add(&switch(
        ctx,
        "Tray icon",
        Some("Click it to turn the pointer on or off"),
        |ctx| ctx.config.borrow().tray,
        |ctx, on| ctx.set("tray", on, |c| c.tray = on),
    ));
    page.add(&startup);
    page
}

fn preview(ctx: &Rc<Ctx>) -> SnapshotArea {
    let weak = Rc::downgrade(ctx);
    let area = SnapshotArea::new(move |s, w, h| {
        let Some(ctx) = weak.upgrade() else { return };
        let config = ctx.config.borrow();
        let target = (w * 0.42, h * 0.38);
        let frac = config.size * PREVIEW_HEIGHT / h;
        let mut state = ArmState::default();
        state.step(1.0 / 120.0, target.0, w);
        match (config.pointer != LASER).then(|| hands::load(&config.pointer)).flatten() {
            Some(tex) => canvas::draw_arm(s, &tex, &state.pose(target, (w, h), &tex.metrics, frac), config.shadow),
            None => canvas::draw_laser(s, target, &arm::laser(0.0, PREVIEW_HEIGHT, config.size)),
        }
    });
    area.set_size_request(-1, 260);
    area.set_overflow(gtk::Overflow::Hidden);
    area.add_css_class("card");
    area
}

pub fn appearance(ctx: &Rc<Ctx>) -> adw::PreferencesPage {
    let page = page();
    let pointer = group("Pointer", None);
    let area = preview(ctx);
    pointer.add(&area);

    let size = adw::ActionRow::new();
    size.set_title("Size");
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, SIZE_MIN, SIZE_MAX, 0.01);
    scale.set_value(ctx.config.borrow().size);
    scale.set_hexpand(true);
    scale.set_width_request(260);
    scale.set_valign(gtk::Align::Center);
    let weak = Rc::downgrade(ctx);
    scale.connect_value_changed(glib::clone!(
        #[weak]
        area,
        move |scale| {
            if let Some(ctx) = weak.upgrade() {
                let value = (scale.value() * 100.0).round() / 100.0;
                if (ctx.config.borrow().size - value).abs() > f64::EPSILON {
                    ctx.set("size", value, |c| c.size = value);
                }
                area.queue_draw();
            }
        }
    ));
    size.add_suffix(&scale);
    pointer.add(&size);

    let shadow = switch(
        ctx,
        "Shadow under the pointer",
        None,
        |ctx| ctx.config.borrow().shadow,
        |ctx, on| ctx.set("shadow", on, |c| c.shadow = on),
    );
    shadow.connect_active_notify(glib::clone!(
        #[weak]
        area,
        move |_| area.queue_draw()
    ));
    pointer.add(&shadow);
    let (weak, scale_ref, area_ref) = (Rc::downgrade(ctx), scale.downgrade(), area.downgrade());
    ctx.on_rebuild(move || {
        if let (Some(ctx), Some(scale), Some(area)) = (weak.upgrade(), scale_ref.upgrade(), area_ref.upgrade()) {
            scale.set_value(ctx.config.borrow().size);
            area.queue_draw();
        }
    });
    page.add(&pointer);
    page
}

pub fn drawing(ctx: &Rc<Ctx>) -> adw::PreferencesPage {
    let page = page();
    let ink = group("Ink", None);
    let colours: Vec<&str> = InkChoice::ALL.iter().map(|c| c.title()).collect();
    ink.add(&combo(
        ctx,
        "Ink colour",
        &colours,
        |ctx| InkChoice::ALL.iter().position(|c| *c == ctx.config.borrow().ink_color).unwrap_or(0),
        |ctx, i| {
            let choice = InkChoice::ALL[i];
            ctx.set("ink-color", choice.as_str(), |c| c.ink_color = choice);
        },
    ));
    let fades: Vec<&str> = FADES.iter().map(|f| f.0).collect();
    let fade = combo(
        ctx,
        "Lines fade",
        &fades,
        |ctx| {
            let current = ctx.config.borrow().ink_fade;
            FADES.iter().position(|f| (f.1 - current).abs() < 0.01).unwrap_or(0)
        },
        |ctx, i| {
            let seconds = FADES[i].1;
            ctx.set("ink-fade", seconds, |c| c.ink_fade = seconds);
        },
    );
    fade.set_subtitle("Clear them any time with tinyhand clear or the C key");
    ink.add(&fade);
    ink.add(&switch(
        ctx,
        "Ink button in the toolbar",
        Some("Change the colour while you present"),
        |ctx| ctx.config.borrow().ink_button,
        |ctx, on| ctx.set("ink-button", on, |c| c.ink_button = on),
    ));
    page.add(&ink);
    page
}

pub fn sound(ctx: &Rc<Ctx>) -> adw::PreferencesPage {
    let page = page();
    let taps = group("Tap sound", Some("Click a sound to hear it. It plays each time the hand taps."));
    let off = gtk::CheckButton::new();
    let row = |title: &str, check: &gtk::CheckButton| {
        let row = adw::ActionRow::new();
        row.set_title(title);
        row.add_prefix(check);
        row.set_activatable_widget(Some(check));
        row
    };
    taps.add(&row("Off", &off));
    let mut checks = vec![(None, off.clone())];
    for &sound in TapSound::ALL {
        let check = gtk::CheckButton::new();
        check.set_group(Some(&off));
        taps.add(&row(sound.title(), &check));
        checks.push((Some(sound), check));
    }
    let sync = {
        let checks = checks.clone();
        move |ctx: &Ctx| {
            let config = ctx.config.borrow();
            let current = config.sound.then_some(config.tap_sound);
            for (sound, check) in &checks {
                check.set_active(*sound == current);
            }
        }
    };
    sync(ctx);
    for (sound, check) in checks {
        let weak = Rc::downgrade(ctx);
        check.connect_toggled(move |check| {
            let Some(ctx) = weak.upgrade() else { return };
            if !check.is_active() {
                return;
            }
            ctx.fresh();
            match sound {
                None if ctx.config.borrow().sound => ctx.set("sound", false, |c| c.sound = false),
                Some(sound) => {
                    let config = ctx.config.borrow().clone();
                    if !config.sound {
                        ctx.set("sound", true, |c| c.sound = true);
                    }
                    if config.tap_sound != sound {
                        ctx.set("tap-sound", sound.as_str(), |c| c.tap_sound = sound);
                    }
                    ctx.player.play(sound);
                }
                None => {}
            }
        });
    }
    let weak = Rc::downgrade(ctx);
    ctx.on_rebuild(move || {
        if let Some(ctx) = weak.upgrade() {
            sync(&ctx);
        }
    });
    page.add(&taps);

    let laser = group("Laser dot", None);
    laser.add(&switch(
        ctx,
        "Laser dot taps too",
        Some("Off by default, for quiet pointing"),
        |ctx| ctx.config.borrow().laser_sound,
        |ctx, on| ctx.set("laser-sound", on, |c| c.laser_sound = on),
    ));
    page.add(&laser);
    page
}

fn link_row(title: &str, detail: &str, url: &'static str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    row.set_title(title);
    row.set_subtitle(detail);
    row.add_suffix(&gtk::Image::from_icon_name("adw-external-link-symbolic"));
    row.set_activatable(true);
    row.connect_activated(move |row| {
        let window = row.root().and_downcast::<gtk::Window>();
        gtk::UriLauncher::new(url).launch(window.as_ref(), gtk::gio::Cancellable::NONE, |_| {});
    });
    row
}

pub fn about(ctx: &Rc<Ctx>) -> adw::PreferencesPage {
    let page = page();
    let info = group("TinyHand", Some("A tiny hand on a stick that points at things on your screen. Free and open source."));
    let version = adw::ActionRow::new();
    version.set_title("Version");
    version.set_subtitle(env!("CARGO_PKG_VERSION"));
    version.add_css_class("property");
    info.add(&version);
    let config = adw::ActionRow::new();
    config.set_title("Config file");
    config.set_subtitle(&paths::config_file().display().to_string());
    config.add_css_class("property");
    config.set_subtitle_selectable(true);
    info.add(&config);
    info.add(&link_row("Original Mac app", "tinyhand.app by Kamran Ahmed", "https://tinyhand.app"));
    info.add(&link_row("Original source code", "github.com/nilbuild/tinyhand", "https://github.com/nilbuild/tinyhand"));
    info.add(&link_row("License", "MIT", "https://opensource.org/license/mit"));
    page.add(&info);

    let reset = group("", None);
    let button = adw::ButtonRow::new();
    button.set_title("Reset all settings…");
    button.add_css_class("destructive-action");
    button.set_sensitive(ctx.writable);
    let weak = Rc::downgrade(ctx);
    button.connect_activated(move |_| {
        let Some(ctx) = weak.upgrade() else { return };
        let dialog = adw::AlertDialog::new(
            Some("Reset all settings?"),
            Some("Toolbar pointers, size, shadow, drawing and sound go back to how they were at first. Your own hands stay."),
        );
        dialog.add_responses(&[("cancel", "Cancel"), ("reset", "Reset")]);
        dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        let weak = Rc::downgrade(&ctx);
        dialog.connect_response(Some("reset"), move |_, _| {
            if let Some(ctx) = weak.upgrade() {
                ctx.reset();
            }
        });
        dialog.present(Some(&ctx.window));
    });
    reset.add(&button);
    page.add(&reset);
    page
}
