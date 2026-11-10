mod maker_dialog;
mod pages;
mod pointers;

use adw::prelude::*;
use gtk::glib::ExitCode;
use gtk::glib;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tinyhand_core::config::{self, Config, Value};
use tinyhand_core::custom::{self, CustomHand};
use tinyhand_core::paths;

pub const APP_ID: &str = "io.github.xdinterface.TinyHand.Settings";

pub struct Ctx {
    pub config: RefCell<Config>,
    pub customs: RefCell<Vec<CustomHand>>,
    path: PathBuf,
    pub writable: bool,
    toasts: adw::ToastOverlay,
    pub window: adw::ApplicationWindow,
    pub player: Rc<crate::sound::Player>,
    rebuild: RefCell<Vec<Box<dyn Fn()>>>,
}

impl Ctx {
    pub fn fresh(&self) {
        self.config.replace(Config::load(&self.path).0);
    }

    pub fn set(&self, key: &str, value: impl Into<Value>, change: impl FnOnce(&mut Config)) {
        self.fresh();
        change(&mut self.config.borrow_mut());
        if let Err(err) = config::set_key(&self.path, key, value.into()) {
            self.toast(&format!("Couldn't save the setting: {err}"));
        }
    }

    pub fn toast(&self, text: &str) {
        self.toasts.add_toast(adw::Toast::new(text));
    }

    pub fn reload_customs(&self) {
        self.customs.replace(custom::load(&paths::custom_hands_dir()));
    }

    pub fn on_rebuild(&self, f: impl Fn() + 'static) {
        self.rebuild.borrow_mut().push(Box::new(f));
    }

    pub fn rebuild(&self) {
        for f in self.rebuild.borrow().iter() {
            f();
        }
    }

    pub fn reset(&self) {
        if let Err(err) = config::reset(&self.path) {
            self.toast(&format!("Couldn't reset: {err}"));
            return;
        }
        self.config.replace(Config::load(&self.path).0);
        self.rebuild();
        self.toast("Settings reset");
    }
}

fn sidebar_row(title: &str, icon: &str) -> gtk::ListBoxRow {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.append(&gtk::Image::from_icon_name(icon));
    row.append(&gtk::Label::new(Some(title)));
    let item = gtk::ListBoxRow::new();
    item.set_child(Some(&row));
    item
}

fn build(app: &adw::Application) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }
    let path = paths::config_file();
    let (loaded, warnings) = Config::load(&path);
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("TinyHand Settings")
        .default_width(860)
        .default_height(640)
        .build();
    let toasts = adw::ToastOverlay::new();
    let ctx = Rc::new(Ctx {
        config: RefCell::new(loaded),
        customs: RefCell::new(custom::load(&paths::custom_hands_dir())),
        writable: config::is_writable(&path),
        path,
        toasts: toasts.clone(),
        window: window.clone(),
        player: Rc::default(),
        rebuild: RefCell::default(),
    });

    let sections: Vec<(&str, &str, gtk::Widget)> = vec![
        ("General", "preferences-system-symbolic", pages::general(&ctx).upcast()),
        ("Pointers", "input-mouse-symbolic", pointers::page(&ctx).upcast()),
        ("Appearance", "applications-graphics-symbolic", pages::appearance(&ctx).upcast()),
        ("Drawing", "document-edit-symbolic", pages::drawing(&ctx).upcast()),
        ("Sound", "audio-speakers-symbolic", pages::sound(&ctx).upcast()),
        ("About", "help-about-symbolic", pages::about(&ctx).upcast()),
    ];

    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    let list = gtk::ListBox::new();
    list.add_css_class("navigation-sidebar");
    for (title, icon, page) in &sections {
        stack.add_named(page, Some(title));
        page.set_sensitive(ctx.writable || *title == "About");
        list.append(&sidebar_row(title, icon));
    }

    let content_header = adw::HeaderBar::new();
    let content_title = adw::WindowTitle::new("General", "");
    content_header.set_title_widget(Some(&content_title));
    let content = adw::ToolbarView::new();
    content.add_top_bar(&content_header);
    let banner = adw::Banner::new("Settings are managed by a read-only config file");
    banner.set_revealed(!ctx.writable);
    content.add_top_bar(&banner);
    toasts.set_child(Some(&stack));
    content.set_content(Some(&toasts));
    let content_page = adw::NavigationPage::new(&content, "General");

    let split = adw::NavigationSplitView::new();
    let titles: Vec<&str> = sections.iter().map(|s| s.0).collect();
    list.connect_row_selected(glib::clone!(
        #[weak]
        stack,
        #[weak]
        split,
        #[weak]
        content_page,
        move |_, row| {
            if let Some(title) = row.and_then(|r| titles.get(r.index() as usize)) {
                stack.set_visible_child_name(title);
                content_title.set_title(title);
                content_page.set_title(title);
                split.set_show_content(true);
            }
        }
    ));
    list.select_row(list.row_at_index(0).as_ref());

    let side = adw::ToolbarView::new();
    side.add_top_bar(&adw::HeaderBar::new());
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_child(Some(&list));
    side.set_content(Some(&scroller));
    split.set_sidebar(Some(&adw::NavigationPage::new(&side, "TinyHand")));
    split.set_content(Some(&content_page));

    let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        560.0,
        adw::LengthUnit::Sp,
    ));
    breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
    window.add_breakpoint(breakpoint);
    window.set_content(Some(&split));
    let keep = ctx.clone();
    window.connect_close_request(move |_| {
        let _ = &keep;
        glib::Propagation::Proceed
    });
    window.present();
    for warning in warnings {
        ctx.toast(&warning);
    }
}

pub fn run() -> ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build);
    app.run_with_args(&["tinyhand"])
}
