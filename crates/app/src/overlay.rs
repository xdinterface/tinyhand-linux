use crate::app::Session;
use crate::backend;
use crate::canvas::{self, SnapshotArea, Stroke};
use crate::toolbar::Toolbar;
use gtk::cairo::Region;
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;
use tinyhand_core::arm::{self, ArmState};
use tinyhand_core::catalog::InteractKey;
use tinyhand_core::ink;

const REVEAL_ON_START: f64 = 3.0;
const REVEAL_WITH_HINT: f64 = 10.0;
const FOCUS_SETTLE: Duration = Duration::from_millis(40);
const FIRST_FRAME: f64 = 1.0 / 120.0;
const LONGEST_FRAME: f64 = 0.05;

fn interact_keys(session: &Session) -> &'static [gdk::Key] {
    match session.config.borrow().interact_key {
        InteractKey::Alt => &[gdk::Key::Alt_L, gdk::Key::Alt_R, gdk::Key::Meta_L, gdk::Key::Meta_R],
        InteractKey::Ctrl => &[gdk::Key::Control_L, gdk::Key::Control_R],
        InteractKey::Super => &[gdk::Key::Super_L, gdk::Key::Super_R],
        InteractKey::Shift => &[gdk::Key::Shift_L, gdk::Key::Shift_R],
        InteractKey::Off => &[],
    }
}

fn interact_mask(session: &Session) -> gdk::ModifierType {
    match session.config.borrow().interact_key {
        InteractKey::Alt => gdk::ModifierType::ALT_MASK,
        InteractKey::Ctrl => gdk::ModifierType::CONTROL_MASK,
        InteractKey::Super => gdk::ModifierType::SUPER_MASK,
        InteractKey::Shift => gdk::ModifierType::SHIFT_MASK,
        InteractKey::Off => gdk::ModifierType::empty(),
    }
}

fn interact_key_held(session: &Session) -> bool {
    let mask = interact_mask(session);
    let held = gdk::Display::default()
        .and_then(|display| display.default_seat())
        .and_then(|seat| seat.keyboard())
        .map(|keyboard| keyboard.modifier_state());
    !mask.is_empty() && held.is_some_and(|state| state.contains(mask))
}

fn seconds(micros: i64) -> f64 {
    micros as f64 / 1e6
}

#[derive(Default)]
struct State {
    arm: ArmState,
    cursor: Option<(f64, f64)>,
    down: Option<(f64, f64)>,
    strokes: Vec<Stroke>,
    drawing: bool,
    passing: bool,
    commit_nudge: bool,
    last_frame: Option<i64>,
    reveal_until: i64,
}

pub struct Overlay {
    session: Weak<Session>,
    pub window: gtk::Window,
    pub monitor: gdk::Monitor,
    canvas: SnapshotArea,
    toolbar: Rc<Toolbar>,
    state: RefCell<State>,
    tick: RefCell<Option<gtk::TickCallbackId>>,
}

impl Overlay {
    pub fn new(session: &Rc<Session>, monitor: &gdk::Monitor, hint: bool) -> Rc<Overlay> {
        let window = gtk::Window::builder().application(&session.app).build();
        window.add_css_class("tinyhand-overlay");
        window.remove_css_class("background");
        window.set_cursor_from_name(Some("none"));
        backend::place(&window, monitor, session.backend(), session.config.borrow().keyboard);

        let toolbar = Toolbar::new(session);
        let overlay = Rc::new_cyclic(|weak: &Weak<Overlay>| {
            let draw = weak.clone();
            let canvas = SnapshotArea::new(move |snapshot, w, h| {
                if let Some(overlay) = draw.upgrade() {
                    overlay.draw(snapshot, w, h);
                }
            });
            canvas.set_can_target(false);
            Overlay {
                session: Rc::downgrade(session),
                window,
                monitor: monitor.clone(),
                canvas,
                toolbar,
                state: RefCell::default(),
                tick: RefCell::default(),
            }
        });

        let catcher = gtk::Box::new(gtk::Orientation::Vertical, 0);
        catcher.set_cursor_from_name(Some("none"));
        let layers = gtk::Overlay::new();
        layers.set_cursor_from_name(Some("none"));
        overlay.toolbar.root.set_cursor_from_name(Some("none"));
        layers.set_child(Some(&catcher));
        layers.add_overlay(&overlay.toolbar.root);
        layers.add_overlay(&overlay.canvas);
        overlay.window.set_child(Some(&layers));
        overlay.connect_input(&catcher);

        let reveal = if hint {
            overlay.toolbar.show_hint();
            REVEAL_WITH_HINT
        } else {
            REVEAL_ON_START
        };
        overlay.state.borrow_mut().reveal_until = glib::monotonic_time() + (reveal * 1e6) as i64;
        overlay.toolbar.set_revealed(true);
        let weak = Rc::downgrade(&overlay);
        glib::timeout_add_local_once(Duration::from_secs_f64(reveal + 0.05), move || {
            if let Some(overlay) = weak.upgrade() {
                overlay.toolbar.hide_hint();
                overlay.update_reveal();
            }
        });

        let weak = Rc::downgrade(session);
        overlay.window.connect_close_request(move |_| {
            if let Some(session) = weak.upgrade() {
                session.deactivate();
            }
            glib::Propagation::Stop
        });
        overlay.window.present();
        if session.passing() {
            overlay.set_passing(true);
        }
        overlay
    }

    fn connect_input(self: &Rc<Self>, catcher: &gtk::Box) {
        let motion = gtk::EventControllerMotion::new();
        motion.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = Rc::downgrade(self);
        motion.connect_enter(move |_, x, y| {
            if let Some(overlay) = weak.upgrade() {
                overlay.moved(x, y);
            }
        });
        let weak = Rc::downgrade(self);
        motion.connect_motion(move |_, x, y| {
            if let Some(overlay) = weak.upgrade() {
                overlay.moved(x, y);
            }
        });
        let weak = Rc::downgrade(self);
        motion.connect_leave(move |_| {
            if let Some(overlay) = weak.upgrade() {
                overlay.state.borrow_mut().cursor = None;
                overlay.update_reveal();
                overlay.canvas.queue_draw();
            }
        });
        self.window.add_controller(motion);

        let buttons = gtk::EventControllerLegacy::new();
        let weak = Rc::downgrade(self);
        buttons.connect_event(move |_, event| {
            let Some(overlay) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            match event.event_type() {
                gdk::EventType::ButtonPress => {
                    let button = event.downcast_ref::<gdk::ButtonEvent>().map_or(0, gdk::ButtonEvent::button);
                    overlay.pressed(button == gdk::BUTTON_PRIMARY);
                    glib::Propagation::Stop
                }
                gdk::EventType::ButtonRelease => {
                    overlay.released();
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        catcher.add_controller(buttons);

        let keys = gtk::EventControllerKey::new();
        let weak = self.session.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(session) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if interact_keys(&session).contains(&key) {
                session.set_passing(true);
                return glib::Propagation::Stop;
            }
            match key {
                gdk::Key::Escape => session.deactivate(),
                gdk::Key::c | gdk::Key::C | gdk::Key::Delete | gdk::Key::BackSpace => session.clear(),
                gdk::Key::m | gdk::Key::M => session.set_sound(!session.sound_on()),
                key => match key.to_unicode().and_then(|c| c.to_digit(10)) {
                    Some(n @ 1..=9) => {
                        if let Some(id) = session.toolbar_ids().get(n as usize - 1) {
                            session.pick(id);
                        }
                    }
                    _ => return glib::Propagation::Proceed,
                },
            }
            glib::Propagation::Stop
        });
        let weak = self.session.clone();
        keys.connect_key_released(move |_, key, _, _| {
            if let Some(session) = weak.upgrade()
                && interact_keys(&session).contains(&key)
            {
                session.set_passing(false);
            }
        });
        let weak = self.session.clone();
        keys.connect_modifiers(move |_, state| {
            if let Some(session) = weak.upgrade() {
                let mask = interact_mask(&session);
                session.set_passing(!mask.is_empty() && state.contains(mask));
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(keys);

        let focus = gtk::EventControllerLegacy::new();
        let weak = self.session.clone();
        focus.connect_event(move |_, event| {
            if let Some(focus) = event.downcast_ref::<gdk::FocusEvent>()
                && let Some(session) = weak.upgrade()
            {
                if focus.is_in() {
                    let weak = Rc::downgrade(&session);
                    glib::timeout_add_local_once(FOCUS_SETTLE, move || {
                        if let Some(session) = weak.upgrade() {
                            session.set_passing(interact_key_held(&session));
                        }
                    });
                } else {
                    session.set_passing(false);
                }
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(focus);

        let weak = self.session.clone();
        self.window.connect_is_active_notify(move |window| {
            if !window.is_active()
                && let Some(session) = weak.upgrade()
            {
                session.set_passing(false);
            }
        });
    }

    fn size(&self) -> (f64, f64) {
        (f64::from(self.canvas.width()), f64::from(self.canvas.height()))
    }

    fn moved(self: &Rc<Self>, x: f64, y: f64) {
        {
            let mut state = self.state.borrow_mut();
            state.cursor = Some((x, y));
            if let Some(start) = state.down {
                self.drag(&mut state, start, (x, y));
            }
        }
        self.update_reveal();
        self.ensure_tick();
    }

    fn drag(&self, state: &mut State, start: ink::Point, to: ink::Point) {
        if !state.drawing {
            if !ink::far_enough(start, to, ink::START_DISTANCE) {
                return;
            }
            let Some(session) = self.session.upgrade() else {
                return;
            };
            state.strokes.push(Stroke::new(start, session.ink_color(), ink::width(self.size().1)));
            state.drawing = true;
        }
        if let Some(stroke) = state.strokes.last_mut() {
            stroke.extend(to);
        }
    }

    fn pressed(self: &Rc<Self>, primary: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.arm.press();
            state.down = state.cursor.filter(|_| primary);
        }
        self.toolbar.hide_hint();
        if let Some(session) = self.session.upgrade() {
            session.tap();
        }
        self.ensure_tick();
    }

    fn released(self: &Rc<Self>) {
        let ended = {
            let mut state = self.state.borrow_mut();
            state.arm.release();
            state.down = None;
            let ended = std::mem::take(&mut state.drawing);
            if ended && let Some(stroke) = state.strokes.last_mut() {
                stroke.ended = Some(glib::monotonic_time());
            }
            ended
        };
        if ended {
            self.wake_for_fade();
        }
        self.update_reveal();
        self.ensure_tick();
    }

    fn wake_for_fade(self: &Rc<Self>) {
        let fade = self.session.upgrade().map_or(0.0, |s| s.config.borrow().ink_fade);
        let Some(life) = ink::life(fade) else {
            return;
        };
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_secs_f64(life - ink::FADE_DURATION), move || {
            if let Some(overlay) = weak.upgrade() {
                overlay.ensure_tick();
            }
        });
    }

    fn update_reveal(&self) {
        let state = self.state.borrow();
        let near = state.cursor.is_some_and(|at| self.toolbar.is_near(at, &self.window));
        let fresh = glib::monotonic_time() < state.reveal_until;
        self.toolbar.set_revealed(!state.drawing && !state.passing && (near || fresh));
    }

    fn ensure_tick(self: &Rc<Self>) {
        if self.tick.borrow().is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        let id = self.canvas.add_tick_callback(move |_, clock| {
            let Some(overlay) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            if overlay.frame(clock.frame_time()) {
                return glib::ControlFlow::Continue;
            }
            overlay.tick.take();
            glib::ControlFlow::Break
        });
        self.tick.replace(Some(id));
    }

    fn frame(&self, now: i64) -> bool {
        let fade = self.session.upgrade().map_or(0.0, |s| s.config.borrow().ink_fade);
        let width = self.size().0;
        let mut state = self.state.borrow_mut();
        let dt = state.last_frame.map_or(FIRST_FRAME, |last| seconds(now - last).clamp(0.0, LONGEST_FRAME));
        if let Some((x, _)) = state.cursor {
            state.arm.step(dt, x, width);
        }
        let opacity = |stroke: &Stroke| stroke.ended.map_or(Some(1.0), |at| ink::opacity(seconds(now - at), fade));
        state.strokes.retain(|stroke| opacity(stroke).is_some());
        let fading = state.strokes.iter().any(|stroke| opacity(stroke).is_some_and(|o| o < 1.0));
        let busy = fading || (state.cursor.is_some() && state.arm.is_settling());
        state.last_frame = busy.then_some(now);
        self.canvas.queue_draw();
        busy
    }

    fn draw(&self, snapshot: &gtk::Snapshot, w: f64, h: f64) {
        let Some(session) = self.session.upgrade() else {
            return;
        };
        let config = session.config.borrow();
        let state = self.state.borrow();
        canvas::draw_commit_nudge(snapshot, state.commit_nudge);
        let now = glib::monotonic_time();
        for stroke in &state.strokes {
            let opacity = stroke.ended.map_or(Some(1.0), |at| ink::opacity(seconds(now - at), config.ink_fade));
            if let Some(opacity) = opacity {
                canvas::draw_stroke(snapshot, stroke, opacity);
            }
        }
        let Some(cursor) = state.cursor.filter(|_| !state.passing) else {
            return;
        };
        match session.hand() {
            Some(hand) => {
                let pose = state.arm.pose(cursor, (w, h), &hand.metrics, config.size);
                canvas::draw_arm(snapshot, &hand, &pose, config.shadow);
            }
            None => canvas::draw_laser(snapshot, cursor, &arm::laser(state.arm.press, h, config.size)),
        }
    }

    pub fn refresh(&self) {
        if let Some(session) = self.session.upgrade() {
            backend::set_keyboard(&self.window, session.backend(), session.config.borrow().keyboard);
        }
        self.toolbar.refresh();
        self.canvas.queue_draw();
    }

    pub fn clear(&self) {
        let mut state = self.state.borrow_mut();
        state.strokes.clear();
        state.drawing = false;
        state.down = None;
        self.canvas.queue_draw();
    }

    pub fn set_passing(&self, passing: bool) {
        {
            let mut state = self.state.borrow_mut();
            state.passing = passing;
            state.commit_nudge = !state.commit_nudge;
            if passing {
                state.cursor = None;
                state.down = None;
                state.drawing = false;
                state.arm.release();
            }
        }
        if let Some(session) = self.session.upgrade() {
            backend::set_passing(&self.window, &self.monitor, session.backend(), passing);
        }
        if let Some(surface) = self.window.surface() {
            let nothing = Region::create();
            surface.set_input_region(passing.then_some(&nothing));
        }
        self.update_reveal();
        self.canvas.queue_draw();
    }

    pub fn close(&self) {
        if let Some(id) = self.tick.take() {
            id.remove();
        }
        self.window.destroy();
    }
}
