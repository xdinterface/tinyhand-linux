use crate::backend::{self, Backend};
use crate::hands::{self, HandTex};
use crate::overlay::Overlay;
use crate::sound::Player;
use crate::tray::Tray;
use gtk::glib::ExitCode;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, OnceCell, RefCell};
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;
use tinyhand_core::catalog::{self, DEFAULT_POINTER, InkChoice, LASER};
use tinyhand_core::config::{self, Config};
use tinyhand_core::custom::{self, CustomHand};
use tinyhand_core::paths;

pub const APP_ID: &str = "io.github.xdinterface.TinyHand";
const OFF: u8 = 1;
const RELOAD_DELAY: Duration = Duration::from_millis(100);

#[derive(Default)]
struct Pointer {
    id: String,
    hand: Option<Rc<HandTex>>,
}

pub struct Session {
    pub app: gtk::Application,
    pub config: RefCell<Config>,
    pub customs: RefCell<Vec<CustomHand>>,
    player: Rc<Player>,
    overlays: RefCell<Vec<Rc<Overlay>>>,
    pointer: RefCell<Pointer>,
    backend: OnceCell<Backend>,
    tray: RefCell<Option<Tray>>,
    hold: RefCell<Option<gio::ApplicationHoldGuard>>,
    watch: RefCell<Option<gio::FileMonitor>>,
    reload_queued: Cell<bool>,
    passing: Cell<bool>,
    warned_readonly: Cell<bool>,
}

fn warn(message: &str) {
    eprintln!("tinyhand: {message}");
}

fn load_config() -> Config {
    let (config, warnings) = Config::load(&paths::config_file());
    warnings.iter().for_each(|w| warn(w));
    config
}

pub fn title(id: &str, customs: &[CustomHand]) -> String {
    if id == LASER {
        return catalog::LASER_TITLE.into();
    }
    match (catalog::builtin(id), customs.iter().find(|h| h.id == id)) {
        (Some(hand), _) => hand.title.into(),
        (_, Some(hand)) => hand.name.clone(),
        _ => id.into(),
    }
}

pub fn knows(id: &str, customs: &[CustomHand]) -> bool {
    id == LASER || catalog::builtin(id).is_some() || customs.iter().any(|h| h.id == id)
}

impl Session {
    fn new(app: &gtk::Application) -> Rc<Session> {
        let session = Rc::new(Session {
            app: app.clone(),
            config: RefCell::new(load_config()),
            customs: RefCell::new(custom::load(&paths::custom_hands_dir())),
            player: Rc::default(),
            overlays: RefCell::default(),
            pointer: RefCell::default(),
            backend: OnceCell::new(),
            tray: RefCell::default(),
            hold: RefCell::default(),
            watch: RefCell::default(),
            reload_queued: Cell::new(false),
            passing: Cell::new(false),
            warned_readonly: Cell::new(false),
        });
        session.resolve();
        session
    }

    pub fn backend(&self) -> Backend {
        *self.backend.get_or_init(backend::detect)
    }

    fn resolve(&self) {
        let wanted = self.config.borrow().pointer.clone();
        let mut pointer = Pointer { id: LASER.into(), hand: None };
        if wanted != LASER {
            for id in [wanted.as_str(), DEFAULT_POINTER] {
                if let Some(hand) = hands::load(id) {
                    pointer = Pointer { id: id.into(), hand: Some(hand) };
                    break;
                }
            }
            if pointer.id != wanted {
                warn(&format!("pointer `{wanted}` is not available, using `{}`", pointer.id));
            }
        }
        self.pointer.replace(pointer);
    }

    pub fn pointer_id(&self) -> String {
        self.pointer.borrow().id.clone()
    }

    pub fn hand(&self) -> Option<Rc<HandTex>> {
        self.pointer.borrow().hand.clone()
    }

    pub fn title(&self, id: &str) -> String {
        title(id, &self.customs.borrow())
    }

    pub fn toolbar_ids(&self) -> Vec<String> {
        let customs = self.customs.borrow();
        let mut ids: Vec<String> =
            self.config.borrow().toolbar.iter().filter(|id| knows(id, &customs)).cloned().collect();
        let current = self.pointer_id();
        if !ids.contains(&current) {
            ids.push(current);
        }
        ids
    }

    pub fn ink_color(&self) -> u32 {
        if let Some(color) = self.config.borrow().ink_color.color() {
            return color;
        }
        let id = self.pointer_id();
        if id == LASER {
            return catalog::RED;
        }
        let customs = self.customs.borrow();
        catalog::builtin(&id)
            .map(|hand| hand.ink)
            .or_else(|| customs.iter().find(|h| h.id == id).map(|h| h.ink))
            .unwrap_or(catalog::PURPLE)
    }

    pub fn sound_on(&self) -> bool {
        let config = self.config.borrow();
        if self.pointer.borrow().id != LASER { config.sound } else { config.laser_sound }
    }

    pub fn tap(&self) {
        if self.sound_on() {
            let (sound, volume) = (self.config.borrow().tap_sound, self.config.borrow().volume);
            self.player.play(sound, volume);
        }
    }

    fn shortcut_labels(&self) -> (Option<String>, Option<String>) {
        let config = self.config.borrow();
        (config::shortcut_label(&config.toggle_shortcut), config::shortcut_label(&config.clear_shortcut))
    }

    pub fn passing(&self) -> bool {
        self.passing.get()
    }

    pub fn is_active(&self) -> bool {
        !self.overlays.borrow().is_empty()
    }

    fn update(self: &Rc<Self>, key: &str, value: config::Value, change: impl FnOnce(&mut Config)) {
        let mut config = self.config.borrow().clone();
        change(&mut config);
        if config::set_key(&paths::config_file(), key, value).is_err() && !self.warned_readonly.replace(true) {
            warn("the config file cannot be written; changes last until TinyHand quits");
        }
        self.apply(config);
    }

    fn apply(self: &Rc<Self>, config: Config) {
        let old = self.config.replace(config);
        let (pointer_changed, tray_changed) = {
            let new = self.config.borrow();
            if *new == old {
                return;
            }
            (new.pointer != old.pointer, new.tray != old.tray)
        };
        if pointer_changed {
            self.resolve();
        }
        if tray_changed {
            self.sync_tray_presence();
        }
        self.refresh();
    }

    fn refresh(&self) {
        let overlays = self.overlays.borrow().clone();
        overlays.iter().for_each(|overlay| overlay.refresh());
        if let Some(tray) = self.tray.borrow().as_ref() {
            tray.set(self.is_active(), self.sound_on(), self.shortcut_labels());
        }
    }

    fn reload(self: &Rc<Self>) {
        self.customs.replace(custom::load(&paths::custom_hands_dir()));
        let before = self.pointer_id();
        if !knows(&before, &self.customs.borrow()) {
            hands::forget(&before);
            self.resolve();
            self.refresh();
        }
        self.apply(load_config());
        if self.config.borrow().pointer != before && self.pointer_id() == before {
            self.resolve();
            self.refresh();
        }
    }

    pub fn pick(self: &Rc<Self>, id: &str) {
        let id = id.to_string();
        self.update("pointer", id.as_str().into(), |c| c.pointer = id.clone());
    }

    pub fn set_sound(self: &Rc<Self>, on: bool) {
        if self.pointer.borrow().id != LASER {
            self.update("sound", on.into(), |c| c.sound = on);
        } else {
            self.update("laser-sound", on.into(), |c| c.laser_sound = on);
        }
    }

    pub fn set_ink(self: &Rc<Self>, choice: InkChoice) {
        self.update("ink-color", choice.as_str().into(), |c| c.ink_color = choice);
    }

    pub fn set_passing(&self, passing: bool) {
        if self.passing.replace(passing) != passing {
            let overlays = self.overlays.borrow().clone();
            overlays.iter().for_each(|overlay| overlay.set_passing(passing));
        }
    }

    pub fn clear(&self) {
        self.overlays.borrow().iter().for_each(|overlay| overlay.clear());
    }

    fn sync_overlays(self: &Rc<Self>, hint: bool) {
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let monitors: Vec<gdk::Monitor> = display.monitors().iter::<gdk::Monitor>().flatten().collect();
        let mut overlays = self.overlays.take();
        overlays.retain(|overlay| {
            let keep = monitors.contains(&overlay.monitor);
            if !keep {
                overlay.close();
            }
            keep
        });
        for monitor in &monitors {
            if !overlays.iter().any(|overlay| overlay.monitor == *monitor) {
                overlays.push(Overlay::new(self, monitor, hint));
            }
        }
        self.overlays.replace(overlays);
    }

    pub fn activate(self: &Rc<Self>) {
        if self.is_active() {
            return;
        }
        let marker = paths::state_dir().join("hint-shown");
        let first_run = !marker.exists();
        if first_run {
            let _ = std::fs::create_dir_all(paths::state_dir()).and_then(|()| std::fs::write(&marker, b""));
        }
        self.passing.set(false);
        if self.pointer.borrow().hand.is_none() && self.pointer.borrow().id != LASER {
            self.resolve();
        }
        self.sync_overlays(first_run);
        let player = self.player.clone();
        glib::idle_add_local_once(move || player.warm());
        self.refresh();
    }

    pub fn deactivate(&self) {
        let overlays = self.overlays.take();
        if overlays.is_empty() {
            return;
        }
        overlays.iter().for_each(|overlay| overlay.close());
        self.passing.set(false);
        self.player.cool();
        self.pointer.borrow_mut().hand = None;
        hands::release();
        self.refresh();
    }

    pub fn toggle(self: &Rc<Self>) {
        if self.is_active() { self.deactivate() } else { self.activate() }
    }

    fn sync_tray_presence(&self) {
        let wanted = self.config.borrow().tray && self.hold.borrow().is_some();
        let mut tray = self.tray.borrow_mut();
        match (wanted, tray.is_some()) {
            (true, false) => *tray = Tray::start(self.is_active(), self.sound_on(), self.shortcut_labels()),
            (false, true) => {
                if let Some(tray) = tray.take() {
                    tray.stop();
                }
            }
            _ => {}
        }
    }

    fn stay(self: &Rc<Self>) {
        if self.hold.borrow().is_some() {
            return;
        }
        self.hold.replace(Some(self.app.hold()));
        self.sync_tray_presence();
        if self.backend() == Backend::Window {
            crate::gnome::enable_extension(&self.app);
        }

        let monitor = gio::File::for_path(paths::config_file())
            .monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
            .ok();
        if let Some(monitor) = &monitor {
            let weak = Rc::downgrade(self);
            monitor.connect_changed(move |_, _, _, _| {
                let Some(session) = weak.upgrade() else {
                    return;
                };
                if session.reload_queued.replace(true) {
                    return;
                }
                let weak = Rc::downgrade(&session);
                glib::timeout_add_local_once(RELOAD_DELAY, move || {
                    if let Some(session) = weak.upgrade() {
                        session.reload_queued.set(false);
                        session.reload();
                    }
                });
            });
        }
        self.watch.replace(monitor);

        if let Some(display) = gdk::Display::default() {
            let weak = Rc::downgrade(self);
            display.monitors().connect_items_changed(move |_, _, _, _| {
                if let Some(session) = weak.upgrade()
                    && session.is_active()
                {
                    session.sync_overlays(false);
                    if !session.is_active() {
                        session.player.cool();
                    }
                    session.refresh();
                }
            });
        }

        let mut ids = self.toolbar_ids().into_iter();
        glib::idle_add_local(move || match ids.next() {
            Some(id) => {
                hands::thumbnail(&id);
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
    }

    fn quit(&self) {
        self.deactivate();
        if let Some(tray) = self.tray.take() {
            tray.stop();
        }
        self.hold.take();
        self.app.quit();
    }

    fn open_settings(&self) {
        self.deactivate();
        if let Ok(exe) = std::env::current_exe() {
            let spawned = gio::Subprocess::newv(
                &[exe.as_os_str(), std::ffi::OsStr::new("settings")],
                gio::SubprocessFlags::NONE,
            );
            if let Err(err) = spawned {
                warn(&format!("cannot open settings: {err}"));
            }
        }
    }

    fn command(self: &Rc<Self>, args: &[String]) -> ExitCode {
        let arg = |i: usize| args.get(i).map(String::as_str);
        match (arg(0), arg(1)) {
            (None, _) => self.stay(),
            (Some("toggle"), _) => {
                self.stay();
                self.toggle();
            }
            (Some("on"), _) => {
                self.stay();
                self.activate();
            }
            (Some("off"), _) => self.deactivate(),
            (Some("clear"), _) => self.clear(),
            (Some("quit"), _) => self.quit(),
            (Some("pick"), Some(id)) => self.pick(id),
            (Some("sound"), None | Some("toggle")) => self.set_sound(!self.sound_on()),
            (Some("sound"), Some("on")) => self.set_sound(true),
            (Some("sound"), Some("off")) => self.set_sound(false),
            (Some("status"), _) => return if self.is_active() { ExitCode::SUCCESS } else { ExitCode::from(OFF) },
            _ => return ExitCode::from(2),
        }
        ExitCode::SUCCESS
    }
}

fn pointer_ids(customs: &[CustomHand]) -> Vec<String> {
    catalog::BUILTIN
        .iter()
        .map(|h| h.id.to_string())
        .chain(customs.iter().map(|h| h.id.clone()))
        .chain([LASER.to_string()])
        .collect()
}

pub fn list() -> ExitCode {
    let customs = custom::load(&paths::custom_hands_dir());
    let mut out = std::io::stdout().lock();
    for id in pointer_ids(&customs) {
        if writeln!(out, "{id:<18}{}", title(&id, &customs)).is_err() {
            break;
        }
    }
    ExitCode::SUCCESS
}

fn usage_error(args: &[String]) -> Option<String> {
    let arg = |i: usize| args.get(i).map(String::as_str);
    match (arg(1), arg(2)) {
        (Some("pick"), None) => Some("usage: tinyhand pick <id>".into()),
        (Some("pick"), Some(id)) if !knows(id, &custom::load(&paths::custom_hands_dir())) => {
            Some(format!("no pointer called `{id}`. See `tinyhand list`."))
        }
        (Some("sound"), Some(value)) if !["on", "off", "toggle"].contains(&value) => {
            Some("usage: tinyhand sound [on|off]".into())
        }
        _ => None,
    }
}

pub fn run(args: &[String]) -> ExitCode {
    if let Some(problem) = usage_error(args) {
        eprintln!("tinyhand: {problem}");
        return ExitCode::FAILURE;
    }

    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let session: Rc<OnceCell<Rc<Session>>> = Rc::default();

    let slot = session.clone();
    app.connect_startup(move |app| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_USER + 1);
        }
        let session = Session::new(app);
        let action = |name: &str, run: fn(&Rc<Session>)| {
            let weak = Rc::downgrade(&session);
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(move |_, _| {
                if let Some(session) = weak.upgrade() {
                    run(&session);
                }
            });
            app.add_action(&action);
        };
        action("toggle", |s| s.toggle());
        action("off", |s| s.deactivate());
        action("clear", |s| s.clear());
        action("sound", |s| s.set_sound(!s.sound_on()));
        action("settings", |s| s.open_settings());
        action("quit", |s| s.quit());
        let _ = slot.set(session);
    });

    let slot = session.clone();
    app.connect_command_line(move |_, command_line| {
        let args: Vec<String> =
            command_line.arguments().iter().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
        slot.get().map_or(ExitCode::FAILURE, |session| session.command(&args))
    });

    let code = app.run_with_args(args);
    if args.get(1).is_some_and(|a| a == "status") {
        println!("{}", if code == ExitCode::SUCCESS { "on" } else { "off" });
    }
    code
}
