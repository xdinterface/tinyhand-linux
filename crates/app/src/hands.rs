use gtk::gdk;
use gtk::gdk_pixbuf::{InterpType, Pixbuf};
use gtk::glib;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tinyhand_core::asset::{self, HandMetrics, Pixels};
use tinyhand_core::{catalog, paths};

const THUMBNAIL: i32 = 128;
const CACHED_HANDS: usize = 2;
const ROD_OVERLAP_ROWS: usize = 2;

pub struct Image {
    pixbuf: Pixbuf,
    bytes: glib::Bytes,
}

impl Image {
    pub fn open(path: &Path) -> Option<Image> {
        let pixbuf = Pixbuf::from_file(path).ok()?;
        (pixbuf.bits_per_sample() == 8).then_some(())?;
        let bytes = pixbuf.read_pixel_bytes();
        Some(Image { pixbuf, bytes })
    }

    pub fn has_alpha(&self) -> bool {
        self.pixbuf.has_alpha() && self.pixbuf.n_channels() == 4
    }

    pub fn pixels(&self) -> Pixels<'_> {
        Pixels {
            width: self.pixbuf.width() as usize,
            height: self.pixbuf.height() as usize,
            stride: self.pixbuf.rowstride() as usize,
            rgba: &self.bytes,
        }
    }

    pub fn pixbuf(&self) -> &Pixbuf {
        &self.pixbuf
    }

    fn rows(&self, from: usize, to: usize) -> gdk::Texture {
        let p = self.pixels();
        let to = to.clamp(from + 1, p.height);
        let end = ((to - 1) * p.stride + p.width * 4).min(self.bytes.len());
        let band = glib::Bytes::from_bytes(&self.bytes, from * p.stride..end);
        gdk::MemoryTexture::new(
            p.width as i32,
            (to - from) as i32,
            gdk::MemoryFormat::R8g8b8a8,
            &band,
            p.stride,
        )
        .into()
    }
}

pub fn texture(pixbuf: &Pixbuf) -> gdk::Texture {
    let format = if pixbuf.has_alpha() { gdk::MemoryFormat::R8g8b8a8 } else { gdk::MemoryFormat::R8g8b8 };
    gdk::MemoryTexture::new(
        pixbuf.width(),
        pixbuf.height(),
        format,
        &pixbuf.read_pixel_bytes(),
        pixbuf.rowstride() as usize,
    )
    .into()
}

pub struct HandTex {
    pub metrics: HandMetrics,
    pub hand: gdk::Texture,
    pub rod: gdk::Texture,
    pub rod_overlap: f64,
}

pub fn path(id: &str) -> Option<PathBuf> {
    if id.starts_with("custom-") {
        let file = paths::custom_hands_dir().join(format!("{id}.png"));
        return file.exists().then_some(file);
    }
    catalog::builtin(id)?;
    paths::find_data(&format!("hands/{id}.png"))
}

fn open(id: &str) -> Option<(Image, HandMetrics)> {
    let image = Image::open(&path(id)?).filter(Image::has_alpha)?;
    let metrics = asset::measure(&image.pixels())?;
    Some((image, metrics))
}

thread_local! {
    static HANDS: RefCell<Vec<(String, Rc<HandTex>)>> = const { RefCell::new(Vec::new()) };
    static THUMBNAILS: RefCell<HashMap<String, gdk::Texture>> = RefCell::new(HashMap::new());
}

pub fn load(id: &str) -> Option<Rc<HandTex>> {
    HANDS.with_borrow_mut(|cache| {
        if let Some(at) = cache.iter().position(|(key, _)| key == id) {
            let hit = cache.remove(at);
            cache.push(hit);
            return cache.last().map(|(_, tex)| tex.clone());
        }
        let (image, metrics) = open(id)?;
        let h = image.pixels().height;
        let row = |frac: f64| (frac * h as f64).round() as usize;
        let (wrist, rod_end) = (row(metrics.wrist), row(metrics.rod_end));
        let overlap = ROD_OVERLAP_ROWS.min(wrist);
        let tex = Rc::new(HandTex {
            metrics,
            hand: image.rows((metrics.hand_top * h as f64) as usize, wrist),
            rod: image.rows(wrist - overlap, rod_end),
            rod_overlap: overlap as f64 / (rod_end.max(wrist + 1) - wrist) as f64,
        });
        cache.push((id.to_string(), tex.clone()));
        if cache.len() > CACHED_HANDS {
            cache.remove(0);
        }
        Some(tex)
    })
}

pub fn release() {
    HANDS.with_borrow_mut(Vec::clear);
}

pub fn forget(id: &str) {
    HANDS.with_borrow_mut(|cache| cache.retain(|(key, _)| key != id));
    THUMBNAILS.with_borrow_mut(|cache| cache.remove(id));
}

pub fn thumbnail(id: &str) -> Option<gdk::Texture> {
    if let Some(hit) = THUMBNAILS.with_borrow(|cache| cache.get(id).cloned()) {
        return Some(hit);
    }
    let (image, metrics) = open(id)?;
    let p = image.pixels();
    let (x, y, w, h) = asset::thumbnail_rect(&metrics, p.width as f64, p.height as f64);
    let crop = image.pixbuf().new_subpixbuf(x as i32, y as i32, (w as i32).max(1), (h as i32).max(1));
    let fit = f64::from(THUMBNAIL) / w.max(h);
    let small = crop.scale_simple(
        ((w * fit) as i32).max(1),
        ((h * fit) as i32).max(1),
        InterpType::Bilinear,
    )?;
    let tex = texture(&small);
    THUMBNAILS.with_borrow_mut(|cache| cache.insert(id.to_string(), tex.clone()));
    Some(tex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_hand_measures() {
        for hand in catalog::BUILTIN {
            let (image, m) = open(hand.id).unwrap_or_else(|| panic!("{} did not load", hand.id));
            assert_eq!((image.pixels().width, image.pixels().height), (1024, 1536), "{}", hand.id);
            assert!(m.hand_top <= m.tip && m.tip < m.wrist && m.wrist < m.rod_end, "{}: {m:?}", hand.id);
            assert!(m.wrist - m.tip > 0.2 && m.rod_end - m.wrist > 0.2, "{}: {m:?}", hand.id);
            assert!((m.rod_x - 0.5).abs() < 0.1 && m.tip_x < m.rod_x, "{}: {m:?}", hand.id);
        }
    }
}
