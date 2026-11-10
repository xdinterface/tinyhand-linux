use crate::hands::HandTex;
use gtk::glib;
use gtk::graphene::{Point, Rect};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gsk};
use std::cell::RefCell;
use tinyhand_core::arm::{ArmPose, Laser};
use tinyhand_core::catalog::{self, RED};
use tinyhand_core::ink::{self, Segment};

type Draw = Box<dyn Fn(&gtk::Snapshot, f64, f64)>;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct SnapshotArea {
        pub draw: RefCell<Option<Draw>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SnapshotArea {
        const NAME: &'static str = "TinyHandSnapshotArea";
        type Type = super::SnapshotArea;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for SnapshotArea {}

    impl WidgetImpl for SnapshotArea {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            if let Some(draw) = self.draw.borrow().as_ref() {
                draw(snapshot, f64::from(widget.width()), f64::from(widget.height()));
            }
        }
    }
}

glib::wrapper! {
    pub struct SnapshotArea(ObjectSubclass<imp::SnapshotArea>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SnapshotArea {
    pub fn new(draw: impl Fn(&gtk::Snapshot, f64, f64) + 'static) -> Self {
        let area: Self = glib::Object::new();
        area.imp().draw.replace(Some(Box::new(draw)));
        area
    }
}

pub fn rgba(color: u32, alpha: f32) -> gdk::RGBA {
    let (r, g, b) = catalog::rgb(color);
    gdk::RGBA::new(r, g, b, alpha)
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect::new(x as f32, y as f32, w as f32, h as f32)
}

pub fn draw_arm(s: &gtk::Snapshot, tex: &HandTex, pose: &ArmPose, shadow: bool) {
    if shadow {
        s.push_shadow(&[gsk::Shadow::new(
            gdk::RGBA::new(0.0, 0.0, 0.0, pose.shadow_alpha as f32),
            pose.shadow_offset.0 as f32,
            pose.shadow_offset.1 as f32,
            (pose.shadow_radius * 2.0) as f32,
        )]);
    }
    s.save();
    s.translate(&Point::new(pose.grip.0 as f32, pose.grip.1 as f32));
    s.rotate(pose.rotation.to_degrees() as f32);
    let x = -pose.rod_x;
    let overlap = pose.rod_len * tex.rod_overlap;
    s.append_scaled_texture(
        &tex.rod,
        gsk::ScalingFilter::Trilinear,
        &rect(x, -pose.rod_len - overlap, pose.width, pose.rod_len + overlap),
    );
    s.append_scaled_texture(
        &tex.hand,
        gsk::ScalingFilter::Trilinear,
        &rect(x, -pose.rod_len - pose.hand_h, pose.width, pose.hand_h),
    );
    s.restore();
    if shadow {
        s.pop();
    }
}

fn disc(s: &gtk::Snapshot, at: (f64, f64), r: f64, color: &gdk::RGBA) {
    let bounds = rect(at.0 - r, at.1 - r, 2.0 * r, 2.0 * r);
    s.push_rounded_clip(&gsk::RoundedRect::from_rect(bounds, r as f32));
    s.append_color(color, &bounds);
    s.pop();
}

pub fn draw_laser(s: &gtk::Snapshot, at: (f64, f64), laser: &Laser) {
    let reach = laser.r + laser.glow * 1.6;
    let stop = |offset: f64, alpha: f32| gsk::ColorStop::new(offset as f32, rgba(RED, alpha));
    s.append_radial_gradient(
        &rect(at.0 - reach, at.1 - reach, 2.0 * reach, 2.0 * reach),
        &Point::new(at.0 as f32, at.1 as f32),
        reach as f32,
        reach as f32,
        0.0,
        1.0,
        &[stop(0.0, 0.9), stop(laser.r / reach, 0.75), stop(0.55, 0.22), stop(1.0, 0.0)],
    );
    disc(s, at, laser.r, &rgba(RED, 1.0));
    disc(s, at, laser.core, &gdk::RGBA::new(1.0, 0.86, 0.82, 0.95));
}

pub struct Stroke {
    pub points: Vec<ink::Point>,
    pub color: u32,
    pub width: f64,
    pub ended: Option<i64>,
    path: RefCell<Option<gsk::Path>>,
}

impl Stroke {
    pub fn new(start: ink::Point, color: u32, width: f64) -> Stroke {
        Stroke { points: vec![start], color, width, ended: None, path: RefCell::new(None) }
    }

    pub fn extend(&mut self, p: ink::Point) {
        if self.points.last().is_some_and(|&last| ink::far_enough(last, p, ink::MIN_STEP)) {
            self.points.push(p);
            self.path.replace(None);
        }
    }

    fn path(&self) -> gsk::Path {
        self.path
            .borrow_mut()
            .get_or_insert_with(|| {
                let builder = gsk::PathBuilder::new();
                if let Some(&(x, y)) = self.points.first() {
                    builder.move_to(x as f32, y as f32);
                }
                for segment in ink::segments(&self.points) {
                    match segment {
                        Segment::Quad(c, to) => builder.quad_to(c.0 as f32, c.1 as f32, to.0 as f32, to.1 as f32),
                        Segment::Line(to) => builder.line_to(to.0 as f32, to.1 as f32),
                    }
                }
                builder.to_path()
            })
            .clone()
    }
}

pub fn draw_stroke(s: &gtk::Snapshot, stroke: &Stroke, opacity: f64) {
    let pen = gsk::Stroke::new(stroke.width as f32);
    pen.set_line_cap(gsk::LineCap::Round);
    pen.set_line_join(gsk::LineJoin::Round);
    if opacity < 1.0 {
        s.push_opacity(opacity);
    }
    s.push_shadow(&[gsk::Shadow::new(gdk::RGBA::new(0.0, 0.0, 0.0, 0.18), 1.0, 1.5, 3.0)]);
    s.append_stroke(&stroke.path(), &pen, &rgba(stroke.color, 1.0));
    s.pop();
    if opacity < 1.0 {
        s.pop();
    }
}

pub fn draw_commit_nudge(s: &gtk::Snapshot, flip: bool) {
    let alpha = if flip { 2.0 } else { 1.0 } / 255.0;
    s.append_color(&gdk::RGBA::new(0.0, 0.0, 0.0, alpha), &rect(0.0, 0.0, 1.0, 1.0));
}
