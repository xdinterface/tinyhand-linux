use crate::hands::{self, Image};
use gtk::cairo::{Context, Format, ImageSurface, LinearGradient};
use gtk::gdk;
use gtk::prelude::*;
use gtk::gdk::prelude::GdkCairoContextExt;
use gtk::glib;
use std::path::Path;
use tinyhand_core::catalog;
use tinyhand_core::custom::{self, CustomHand};
use tinyhand_core::handmaker::{self, CANVAS, NO_ALPHA, ROD_GRADIENT, UNREADABLE};
use tinyhand_core::paths;

const COMPOSE_FAILED: &str = "TinyHand couldn't put that hand on a stick.";
const SAVE_FAILED: &str = "TinyHand couldn't save that hand.";

pub struct Prepared {
    pub texture: gdk::Texture,
    pub ink: u32,
}

fn rounded(cr: &Context, (x, y, w, h): (f64, f64, f64, f64)) {
    let r = w / 2.0;
    cr.new_sub_path();
    cr.arc(x + r, y + r, r, std::f64::consts::PI, 0.0);
    cr.arc(x + r, y + h - r, r, 0.0, std::f64::consts::PI);
    cr.close_path();
}

pub fn prepare(path: &Path) -> Result<Prepared, &'static str> {
    let image = Image::open(path).ok_or(UNREADABLE)?;
    if !image.has_alpha() {
        return Err(NO_ALPHA);
    }
    let pixels = image.pixels();
    handmaker::check(&pixels)?;
    let layout = handmaker::compose_layout(&pixels).ok_or(COMPOSE_FAILED)?;
    let ink = handmaker::ink_color(&pixels);

    let mut surface =
        ImageSurface::create(Format::ARgb32, CANVAS.0 as i32, CANVAS.1 as i32).map_err(|_| COMPOSE_FAILED)?;
    {
        let cr = Context::new(&surface).map_err(|_| COMPOSE_FAILED)?;
        let (x, _, w, _) = layout.rod;
        let gradient = LinearGradient::new(x, 0.0, x + w, 0.0);
        for (offset, color) in ROD_GRADIENT {
            let (r, g, b) = catalog::rgb(color);
            gradient.add_color_stop_rgb(offset, r.into(), g.into(), b.into());
        }
        rounded(&cr, layout.rod);
        cr.set_source(&gradient).map_err(|_| COMPOSE_FAILED)?;
        cr.fill().map_err(|_| COMPOSE_FAILED)?;
        cr.translate(layout.origin.0, layout.origin.1);
        cr.scale(layout.scale, layout.scale);
        cr.set_source_pixbuf(image.pixbuf(), 0.0, 0.0);
        cr.source().set_filter(gtk::cairo::Filter::Best);
        cr.paint().map_err(|_| COMPOSE_FAILED)?;
    }
    surface.flush();
    let stride = surface.stride() as usize;
    let data = surface.data().map_err(|_| COMPOSE_FAILED)?.to_vec();
    let texture = gdk::MemoryTexture::new(
        CANVAS.0 as i32,
        CANVAS.1 as i32,
        gdk::MemoryFormat::B8g8r8a8Premultiplied,
        &glib::Bytes::from_owned(data),
        stride,
    );
    Ok(Prepared { texture: texture.upcast(), ink })
}

pub fn save(prepared: &Prepared, name: &str) -> Result<CustomHand, &'static str> {
    let dir = paths::custom_hands_dir();
    let hand = custom::add(&dir, name, prepared.ink).map_err(|_| SAVE_FAILED)?;
    let file = dir.join(format!("{}.png", hand.id));
    if prepared.texture.save_to_png(&file).is_err() {
        let _ = custom::remove(&dir, &hand.id);
        return Err(SAVE_FAILED);
    }
    hands::forget(&hand.id);
    if hands::load(&hand.id).is_none() {
        let _ = custom::remove(&dir, &hand.id);
        return Err(COMPOSE_FAILED);
    }
    Ok(hand)
}

pub fn delete(id: &str) -> std::io::Result<()> {
    hands::forget(id);
    custom::remove(&paths::custom_hands_dir(), id)
}

pub fn run(args: &[String]) -> glib::ExitCode {
    let Some(input) = args.first() else {
        eprintln!("usage: tinyhand make-hand <image.png> [name]");
        return glib::ExitCode::FAILURE;
    };
    if gtk::init().is_err() {
        eprintln!("tinyhand: no display");
        return glib::ExitCode::FAILURE;
    }
    let path = Path::new(input);
    let count = custom::load(&paths::custom_hands_dir()).len();
    let name = args.get(1).cloned().unwrap_or_else(|| {
        custom::suggested_name(&path.file_stem().unwrap_or_default().to_string_lossy(), count)
    });
    match prepare(path).and_then(|prepared| save(&prepared, &name)) {
        Ok(hand) => {
            println!("added {} ({})", hand.id, hand.name);
            println!("use it with: tinyhand pick {}", hand.id);
            glib::ExitCode::SUCCESS
        }
        Err(problem) => {
            eprintln!("tinyhand: {problem}");
            glib::ExitCode::FAILURE
        }
    }
}
