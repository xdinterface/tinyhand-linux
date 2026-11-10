use crate::app::Session;
use crate::hands;
use gtk::cairo;
use gtk::graphene;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::f64::consts::TAU;
use std::rc::{Rc, Weak};
use tinyhand_core::catalog::{self, InkChoice, KeyboardMode, LASER, RED};

const BUTTON: i32 = 40;
const SWATCH: i32 = 30;
const RAINBOW: [u32; 6] = [0xFF3B30, 0xFF9500, 0xFFCC00, 0x34C759, 0x007AFF, 0xAF52DE];

fn source(cr: &cairo::Context, color: u32, alpha: f64) {
    let (r, g, b) = catalog::rgb(color);
    cr.set_source_rgba(r.into(), g.into(), b.into(), alpha);
}

fn circle(cr: &cairo::Context, x: f64, y: f64, r: f64) {
    cr.arc(x, y, r, 0.0, TAU);
    let _ = cr.fill();
}

fn icon(size: i32, draw: impl Fn(&cairo::Context) + 'static) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_width(size);
    area.set_content_height(size);
    area.set_draw_func(move |_, cr, w, h| {
        cr.scale(f64::from(w) / 64.0, f64::from(h) / 64.0);
        draw(cr);
    });
    area
}

fn dot(cr: &cairo::Context, color: Option<u32>) {
    let r = 13.0;
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.35);
    circle(cr, 32.0, 33.0, r + 1.5);
    match color {
        Some(color) => {
            source(cr, color, 1.0);
            circle(cr, 32.0, 32.0, r);
        }
        None => {
            for (i, color) in RAINBOW.into_iter().enumerate() {
                let from = TAU * (i as f64 / 6.0 - 0.25);
                source(cr, color, 1.0);
                cr.move_to(32.0, 32.0);
                cr.arc(32.0, 32.0, r, from, from + TAU / 6.0);
                let _ = cr.fill();
            }
        }
    }
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.35);
    cr.save().ok();
    cr.translate(28.5, 26.5);
    cr.scale(4.5, 3.2);
    circle(cr, 0.0, 0.0, 1.0);
    cr.restore().ok();
}

pub fn laser_icon(size: i32) -> gtk::DrawingArea {
    icon(size, |cr| {
        let (r, g, b) = catalog::rgb(RED);
        let glow = cairo::RadialGradient::new(32.0, 32.0, 0.0, 32.0, 32.0, 18.0);
        glow.add_color_stop_rgba(0.0, r.into(), g.into(), b.into(), 0.55);
        glow.add_color_stop_rgba(1.0, r.into(), g.into(), b.into(), 0.0);
        let _ = cr.set_source(&glow);
        circle(cr, 32.0, 32.0, 18.0);
        source(cr, RED, 1.0);
        circle(cr, 32.0, 32.0, 7.0);
        cr.set_source_rgba(1.0, 0.86, 0.82, 0.95);
        circle(cr, 32.0, 32.0, 3.0);
    })
}

pub fn pointer_icon(id: &str, size: i32) -> gtk::Widget {
    if id == LASER {
        return laser_icon(size).upcast();
    }
    let image = gtk::Image::new();
    image.set_pixel_size(size);
    image.set_paintable(hands::thumbnail(id).as_ref());
    image.upcast()
}

fn button(child: &impl IsA<gtk::Widget>, tip: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(child));
    button.set_tooltip_text(Some(tip));
    button.set_focusable(false);
    button.set_focus_on_click(false);
    button
}

pub struct Toolbar {
    session: Weak<Session>,
    pub root: gtk::Box,
    bar: gtk::Box,
    hands: gtk::Box,
    shown: RefCell<Vec<String>>,
    ink: gtk::Button,
    ink_color: Rc<Cell<u32>>,
    swatches: gtk::Box,
    sound: gtk::Button,
    hint: gtk::Box,
}

impl Toolbar {
    pub fn new(session: &Rc<Session>) -> Rc<Toolbar> {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.add_css_class("th-toolbar");
        root.set_halign(gtk::Align::Center);
        root.set_valign(gtk::Align::End);

        let hint = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        hint.add_css_class("th-hint");
        hint.set_halign(gtk::Align::Center);
        hint.set_visible(false);
        hint.set_can_target(false);

        let swatches = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        swatches.add_css_class("th-swatches");
        swatches.set_halign(gtk::Align::Center);
        swatches.set_visible(false);

        let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        bar.add_css_class("th-bar");
        bar.set_halign(gtk::Align::Center);
        let hands = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let divider = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        divider.add_css_class("th-divider");
        divider.set_valign(gtk::Align::Center);

        let ink_color = Rc::new(Cell::new(catalog::PURPLE));
        let ink_dot = icon(BUTTON, {
            let color = ink_color.clone();
            move |cr| dot(cr, Some(color.get()))
        });
        let ink = button(&ink_dot, "Ink colour");
        let sound = button(&gtk::Image::from_icon_name("audio-volume-high-symbolic"), "Tap sound");
        let settings = button(&gtk::Image::from_icon_name("emblem-system-symbolic"), "Settings");
        let exit = button(&gtk::Image::from_icon_name("window-close-symbolic"), "Exit (Esc)");

        bar.append(&hands);
        bar.append(&divider);
        bar.append(&ink);
        bar.append(&sound);
        bar.append(&settings);
        bar.append(&exit);
        root.append(&hint);
        root.append(&swatches);
        root.append(&bar);

        let toolbar = Rc::new(Toolbar {
            session: Rc::downgrade(session),
            root,
            bar,
            hands,
            shown: RefCell::default(),
            ink,
            ink_color,
            swatches,
            sound,
            hint,
        });

        for &choice in InkChoice::ALL {
            let swatch = button(&icon(SWATCH, move |cr| dot(cr, choice.color())), choice.title());
            let weak = Rc::downgrade(&toolbar);
            swatch.connect_clicked(move |_| {
                if let Some(toolbar) = weak.upgrade() {
                    toolbar.swatches.set_visible(false);
                    if let Some(session) = toolbar.session.upgrade() {
                        session.set_ink(choice);
                    }
                }
            });
            toolbar.swatches.append(&swatch);
        }

        let weak = Rc::downgrade(&toolbar);
        toolbar.ink.connect_clicked(move |_| {
            if let Some(toolbar) = weak.upgrade() {
                toolbar.swatches.set_visible(!toolbar.swatches.is_visible());
                toolbar.refresh();
            }
        });
        let weak = Rc::downgrade(session);
        toolbar.sound.connect_clicked(move |_| {
            if let Some(session) = weak.upgrade() {
                session.set_sound(!session.sound_on());
            }
        });
        let weak = Rc::downgrade(session);
        settings.connect_clicked(move |_| {
            if let Some(session) = weak.upgrade() {
                session.open_settings();
            }
        });
        let weak = Rc::downgrade(session);
        exit.connect_clicked(move |_| {
            if let Some(session) = weak.upgrade() {
                session.deactivate();
            }
        });

        toolbar.refresh();
        toolbar
    }

    pub fn refresh(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let ids = session.toolbar_ids();
        let current = session.pointer_id();
        if *self.shown.borrow() != ids {
            while let Some(child) = self.hands.first_child() {
                self.hands.remove(&child);
            }
            for id in &ids {
                let pick = button(&pointer_icon(id, BUTTON - 6), &session.title(id));
                let (weak, id) = (self.session.clone(), id.clone());
                pick.connect_clicked(move |_| {
                    if let Some(session) = weak.upgrade() {
                        session.pick(&id);
                    }
                });
                self.hands.append(&pick);
            }
            self.shown.replace(ids.clone());
        }
        let mut child = self.hands.first_child();
        for id in &ids {
            let Some(pick) = child else { break };
            pick.set_css_classes(if *id == current { &["selected"] } else { &[] });
            child = pick.next_sibling();
        }

        let config = session.config.borrow();
        self.ink_color.set(session.ink_color());
        self.ink.set_visible(config.ink_button);
        if !config.ink_button {
            self.swatches.set_visible(false);
        }
        self.ink.set_css_classes(if self.swatches.is_visible() { &["selected"] } else { &[] });
        if let Some(dot) = self.ink.child() {
            dot.queue_draw();
        }
        let mut swatch = self.swatches.first_child();
        for &choice in InkChoice::ALL {
            let Some(widget) = swatch else { break };
            widget.set_css_classes(if choice == config.ink_color { &["selected"] } else { &[] });
            swatch = widget.next_sibling();
        }

        let on = session.sound_on();
        let (name, tip) = if on {
            ("audio-volume-high-symbolic", "Tap sound on. Click to mute (M)")
        } else {
            ("audio-volume-muted-symbolic", "Tap sound off. Click to unmute (M)")
        };
        self.sound.set_child(Some(&gtk::Image::from_icon_name(name)));
        self.sound.set_tooltip_text(Some(tip));
        self.sound.set_css_classes(if on { &[] } else { &["muted"] });
    }

    pub fn set_revealed(&self, revealed: bool) {
        if self.root.has_css_class("revealed") == revealed {
            return;
        }
        if revealed {
            self.root.add_css_class("revealed");
        } else {
            self.root.remove_css_class("revealed");
            if self.swatches.is_visible() {
                self.swatches.set_visible(false);
                self.refresh();
            }
        }
    }

    pub fn show_hint(&self) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let config = session.config.borrow();
        let mut parts = vec![("Click".to_string(), "to tap"), ("Drag".to_string(), "to draw")];
        if config.keyboard == KeyboardMode::Grab {
            if config.interact_key != catalog::InteractKey::Off {
                parts.push((format!("Hold {}", config.interact_key.title()), "to click for real"));
            }
            parts.push(("Esc".to_string(), "to exit"));
        }
        for (key, text) in parts {
            let part = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            let key = gtk::Label::new(Some(&key));
            key.add_css_class("th-key");
            let text = gtk::Label::new(Some(text));
            text.add_css_class("th-text");
            part.append(&key);
            part.append(&text);
            self.hint.append(&part);
        }
        self.hint.set_visible(true);
    }

    pub fn hide_hint(&self) {
        self.hint.set_visible(false);
    }

    pub fn is_near(&self, point: (f64, f64), within: &impl IsA<gtk::Widget>) -> bool {
        let target = if self.swatches.is_visible() { self.root.upcast_ref::<gtk::Widget>() } else { self.bar.upcast_ref() };
        let Some(bounds) = target.compute_bounds(within) else {
            return false;
        };
        let zone = graphene::Rect::new(
            bounds.x() - 80.0,
            bounds.y() - 100.0,
            bounds.width() + 160.0,
            bounds.height() + 110.0,
        );
        zone.contains_point(&graphene::Point::new(point.0 as f32, point.1 as f32))
    }
}
