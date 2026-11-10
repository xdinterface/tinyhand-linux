use crate::canvas::{self, Stroke};
use crate::hands;
use gtk::glib::ExitCode;
use gtk::graphene::Rect;
use gtk::prelude::*;
use gtk::{gdk, gsk};
use tinyhand_core::arm::{self, ArmState};
use tinyhand_core::catalog::{self, LASER};
use tinyhand_core::ink;

const PANEL: (f64, f64) = (420.0, 560.0);
const TARGET: (f64, f64) = (190.0, 180.0);

pub fn run(args: &[String]) -> ExitCode {
    let Some(out) = args.first() else {
        eprintln!("usage: tinyhand render <out.png> [pointer ids...]");
        return ExitCode::FAILURE;
    };
    if gtk::init().is_err() {
        eprintln!("tinyhand: no display");
        return ExitCode::FAILURE;
    }
    let ids: Vec<&str> = match &args[1..] {
        [] => vec!["lavender", "glove", "robot", "skeleton", LASER],
        ids => ids.iter().map(String::as_str).collect(),
    };

    let s = gtk::Snapshot::new();
    let bounds = Rect::new(0.0, 0.0, (PANEL.0 * ids.len() as f64) as f32, PANEL.1 as f32);
    s.append_color(&gdk::RGBA::new(0.93, 0.94, 0.92, 1.0), &bounds);
    for (i, id) in ids.iter().enumerate() {
        s.save();
        s.translate(&gtk::graphene::Point::new((PANEL.0 * i as f64) as f32, 0.0));
        s.push_clip(&Rect::new(0.0, 0.0, PANEL.0 as f32, PANEL.1 as f32));

        let ink = catalog::builtin(id).map_or(catalog::RED, |h| h.ink);
        let centre = (TARGET.0 + 40.0, TARGET.1 + 8.0);
        let mut stroke: Option<Stroke> = None;
        for k in 0..=60 {
            let a = f64::from(k) / 60.0 * std::f64::consts::TAU + std::f64::consts::PI;
            let p = (centre.0 + 40.0 * a.cos(), centre.1 + 32.0 * a.sin());
            match &mut stroke {
                Some(stroke) => stroke.extend(p),
                None => stroke = Some(Stroke::new(p, ink, ink::width(PANEL.1))),
            }
        }
        if let Some(stroke) = &stroke {
            canvas::draw_stroke(&s, stroke, 1.0);
        }

        let mut state = ArmState::default();
        state.step(1.0 / 120.0, TARGET.0, PANEL.0);
        if i % 2 == 1 {
            state.press();
        }
        match hands::load(id) {
            Some(tex) => canvas::draw_arm(&s, &tex, &state.pose(TARGET, PANEL, &tex.metrics, 0.13), true),
            None => canvas::draw_laser(&s, TARGET, &arm::laser(state.press, PANEL.1, 0.13)),
        }
        if *id != LASER && std::env::var_os("TINYHAND_NO_MARKER").is_none() {
            let marker = Rect::new(TARGET.0 as f32 - 3.0, TARGET.1 as f32 - 3.0, 6.0, 6.0);
            s.append_color(&gdk::RGBA::new(0.95, 0.8, 0.1, 1.0), &marker);
        }
        s.pop();
        s.restore();
    }

    let Some(node) = s.to_node() else {
        return ExitCode::FAILURE;
    };
    let renderer = gsk::CairoRenderer::new();
    let Some(display) = gdk::Display::default() else {
        return ExitCode::FAILURE;
    };
    if renderer.realize_for_display(&display).is_err() {
        return ExitCode::FAILURE;
    }
    let texture = renderer.render_texture(&node, Some(&bounds));
    renderer.unrealize();
    match texture.save_to_png(out) {
        Ok(()) => {
            println!("rendered {out}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("tinyhand: {err}");
            ExitCode::FAILURE
        }
    }
}
