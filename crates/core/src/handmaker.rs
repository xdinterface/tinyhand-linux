use crate::asset::Pixels;

pub const CANVAS: (f64, f64) = (1024.0, 1536.0);
pub const ROD_GRADIENT: [(f64, u32); 5] =
    [(0.0, 0x000000), (0.22, 0x121214), (0.45, 0x3A3A3E), (0.75, 0x121214), (1.0, 0x000000)];
pub const NO_ALPHA: &str = "The image needs a transparent background. Save it as a PNG with transparency.";
pub const UNREADABLE: &str = "That file isn't an image TinyHand can read.";

const HAND_TOP: f64 = 150.0;
const HAND_HEIGHT: f64 = 560.0;
const HAND_MAX_WIDTH: f64 = 700.0;
const ROD_BOTTOM: f64 = 1462.0;

const PROMPT: &str = "\
Make a pointer image for TinyHand, an app that shows a toy hand on screen.

Draw one cartoon right hand in this style: [style]

Rules:
- We see the BACK of the hand. The palm faces away from us. The knuckles and the smooth back of the hand face the viewer.
- The index finger points straight up. The other three fingers are folded down, so we only see their knuckles, never their fingertips or fingernails.
- The raised index finger sits at the far left of the knuckle row, right next to the thumb. Three folded knuckles sit to its right. It must never look like a middle finger.
- The thumb is tucked against the left side of the hand. Nothing crosses the front of the fist: no thumb, no fingertips, no fingernails, no palm.
- The fingertip is the highest point of the image. Nothing above or beside it.
- The hand ends at the bottom in a wrist cuff or a clean flat wrist, facing straight down, centred.
- Upright, not tilted. Whole hand visible, nothing cropped.
- Transparent background. No shadow, no floor, no outline glow, no text, no stick, no arm.
- Soft 3D toy look, bright and readable at small sizes.
- Portrait image, hand filling most of the height with a little empty space around it.
";

pub fn prompt(idea: &str) -> String {
    let idea = idea.trim();
    PROMPT.replace("[style]", if idea.is_empty() { "a soft vinyl toy in a bright colour" } else { idea })
}

struct Bounds {
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
}

impl Bounds {
    fn width(&self) -> usize {
        self.max_x - self.min_x + 1
    }

    fn height(&self) -> usize {
        self.max_y - self.min_y + 1
    }
}

fn bounds(p: &Pixels) -> Option<Bounds> {
    let mut b = Bounds { min_x: p.width, min_y: p.height, max_x: 0, max_y: 0 };
    let mut found = false;
    for y in 0..p.height {
        for x in 0..p.width {
            if p.alpha(x, y) > 128 {
                found = true;
                b.min_x = b.min_x.min(x);
                b.max_x = b.max_x.max(x);
                b.min_y = b.min_y.min(y);
                b.max_y = b.max_y.max(y);
            }
        }
    }
    found.then_some(b)
}

fn span(p: &Pixels, b: &Bounds, rows: impl Iterator<Item = usize>) -> Option<(usize, usize)> {
    let mut out: Option<(usize, usize)> = None;
    for y in rows {
        for x in b.min_x..=b.max_x {
            if p.alpha(x, y) > 128 {
                out = Some(out.map_or((x, x), |(lo, hi)| (lo.min(x), hi.max(x))));
            }
        }
    }
    out
}

fn bottom_span(p: &Pixels, b: &Bounds) -> (usize, f64) {
    let from = b.min_y.max(b.max_y.saturating_sub((b.height() * 4 / 100).max(2)));
    match span(p, b, from..=b.max_y) {
        Some((lo, hi)) => (hi - lo + 1, (lo + hi) as f64 / 2.0),
        None => (0, (b.min_x + b.max_x) as f64 / 2.0),
    }
}

fn blobs(p: &Pixels) -> Vec<usize> {
    let step = (p.width.min(p.height) / 256).max(1);
    let (gw, gh) = (p.width / step, p.height / step);
    let mut todo: Vec<bool> =
        (0..gw * gh).map(|i| p.alpha(i % gw * step, i / gw * step) > 128).collect();
    let mut sizes = Vec::new();
    for start in 0..todo.len() {
        if !todo[start] {
            continue;
        }
        todo[start] = false;
        let mut stack = vec![start];
        let mut n = 0;
        while let Some(i) = stack.pop() {
            n += 1;
            let (x, y) = (i % gw, i / gw);
            let mut visit = |j: usize| {
                if todo[j] {
                    todo[j] = false;
                    stack.push(j);
                }
            };
            if x + 1 < gw {
                visit(i + 1);
            }
            if x > 0 {
                visit(i - 1);
            }
            if y + 1 < gh {
                visit(i + gw);
            }
            if y > 0 {
                visit(i - gw);
            }
        }
        sizes.push(n);
    }
    sizes
}

pub fn check(p: &Pixels) -> Result<(), &'static str> {
    let (w, h) = (p.width, p.height);
    if w < 64 || h < 64 {
        return Err("The image is too small. Use one at least 512 pixels tall.");
    }

    let edge_solid = (0..w).map(|x| (x, 0)).chain((0..w).map(|x| (x, h - 1)))
        .chain((0..h).map(|y| (0, y)))
        .chain((0..h).map(|y| (w - 1, y)))
        .filter(|&(x, y)| p.alpha(x, y) > 32)
        .count();
    if edge_solid as f64 / (2 * (w + h)) as f64 > 0.02 {
        return Err("The background isn't transparent, or the hand touches the edge of the image.");
    }

    let sizes = blobs(p);
    let biggest = *sizes.iter().max().ok_or("The image is empty.")?;
    if (biggest as f64) / (sizes.iter().sum::<usize>() as f64) < 0.97 {
        return Err("The image has more than one thing in it. Use a single hand.");
    }

    let b = bounds(p).ok_or("The image is empty.")?;
    let (bw, bh) = (b.width() as f64, b.height() as f64);
    if !(1.05..=2.8).contains(&(bh / bw)) {
        return Err("The hand should stand upright, taller than it is wide.");
    }
    if bh / (h as f64) < 0.5 {
        return Err("The hand is too small in the image. Crop it closer.");
    }

    let top_band = (b.height() * 8 / 100).max(2);
    let finger = span(p, &b, b.min_y..b.min_y + top_band).map_or(0, |(lo, hi)| hi - lo + 1);
    if finger as f64 / bw > 0.4 {
        return Err("The top of the image should be a single finger pointing up.");
    }
    if (bottom_span(p, &b).0 as f64) / bw < 0.2 {
        return Err("The hand should end in a flat wrist or cuff at the bottom.");
    }

    let (mut soft, mut any) = (0usize, 0usize);
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let a = p.alpha(x, y);
            if a > 20 {
                any += 1;
                soft += usize::from(a < 200);
            }
        }
    }
    if soft as f64 / any.max(1) as f64 > 0.08 {
        return Err("The hand has a soft shadow or glow around it. Ask for no shadow.");
    }
    Ok(())
}

pub fn ink_color(p: &Pixels) -> u32 {
    let mut buckets = [(0usize, [0f64; 3]); 216];
    for y in (0..p.height).step_by(3) {
        for x in (0..p.width).step_by(3) {
            if p.alpha(x, y) <= 200 {
                continue;
            }
            let c = p.rgb(x, y).map(|v| f64::from(v) / 255.0);
            let key = (c[0] * 5.0) as usize * 36 + (c[1] * 5.0) as usize * 6 + (c[2] * 5.0) as usize;
            buckets[key].0 += 1;
            for (sum, v) in buckets[key].1.iter_mut().zip(c) {
                *sum += v;
            }
        }
    }

    let mut best = (-1.0, [0.53, 0.33, 0.95]);
    for (n, sum) in buckets.into_iter().filter(|(n, _)| *n > 0) {
        let c = sum.map(|v| v / n as f64);
        let hi = c[0].max(c[1]).max(c[2]);
        let lo = c[0].min(c[1]).min(c[2]);
        let sat = if hi == 0.0 { 0.0 } else { (hi - lo) / hi };
        let lum = 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
        let score = n as f64 * (0.25 + sat) * if lum > 0.12 && lum < 0.85 { 1.0 } else { 0.15 };
        if score > best.0 {
            best = (score, c);
        }
    }

    let mut c = best.1;
    let hi = c[0].max(c[1]).max(c[2]);
    let lo = c[0].min(c[1]).min(c[2]);
    if hi - lo < 0.08 {
        c = [0.42, 0.45, 0.5];
    } else if hi > 0.82 {
        c = c.map(|v| v * 0.82 / hi);
    }
    let [r, g, b] = c.map(|v| (v * 255.0).round().min(255.0) as u32);
    r << 16 | g << 8 | b
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub scale: f64,
    pub origin: (f64, f64),
    pub rod: (f64, f64, f64, f64),
}

pub fn compose_layout(p: &Pixels) -> Option<Layout> {
    let b = bounds(p)?;
    let (bw, bh) = (b.width() as f64, b.height() as f64);
    let scale = (HAND_HEIGHT / bh).min(HAND_MAX_WIDTH / bw);
    let wrist_x = (bottom_span(p, &b).1 + 0.5) * scale;
    let hand_bottom = HAND_TOP + bh * scale;
    let rod_w = (bw * scale * 0.18).clamp(54.0, 76.0);
    let rod_top = hand_bottom - 30.0;
    Some(Layout {
        scale,
        origin: (CANVAS.0 / 2.0 - wrist_x, HAND_TOP - b.min_y as f64 * scale),
        rod: (CANVAS.0 / 2.0 - rod_w / 2.0, rod_top, rod_w, ROD_BOTTOM - rod_top),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::Pixels;
    use crate::asset::tests::image;

    const W: usize = 200;
    const H: usize = 300;
    const GOOD: [(usize, usize, usize, usize); 2] = [(60, 20, 80, 80), (40, 80, 160, 280)];

    fn verdict(rgba: &[u8], w: usize, h: usize) -> Result<(), &'static str> {
        check(&Pixels::new(w, h, rgba))
    }

    #[test]
    fn accepts_an_upright_hand_with_a_finger_and_a_wrist() {
        assert_eq!(verdict(&image(W, H, &GOOD), W, H), Ok(()));
    }

    #[test]
    fn rejects_tiny_images() {
        assert_eq!(
            verdict(&image(40, 60, &[(10, 10, 30, 50)]), 40, 60),
            Err("The image is too small. Use one at least 512 pixels tall.")
        );
    }

    #[test]
    fn rejects_opaque_backgrounds_and_edge_contact() {
        assert_eq!(
            verdict(&image(W, H, &[(0, 0, W, H)]), W, H),
            Err("The background isn't transparent, or the hand touches the edge of the image.")
        );
    }

    #[test]
    fn rejects_empty_images() {
        assert_eq!(verdict(&image(W, H, &[]), W, H), Err("The image is empty."));
    }

    #[test]
    fn rejects_more_than_one_object() {
        let mut rects = GOOD.to_vec();
        rects.push((170, 20, 195, 120));
        assert_eq!(
            verdict(&image(W, H, &rects), W, H),
            Err("The image has more than one thing in it. Use a single hand.")
        );
    }

    #[test]
    fn rejects_wide_shapes() {
        assert_eq!(
            verdict(&image(300, 200, &[(20, 60, 280, 160)]), 300, 200),
            Err("The hand should stand upright, taller than it is wide.")
        );
    }

    #[test]
    fn rejects_small_hands() {
        assert_eq!(
            verdict(&image(W, H, &[(90, 100, 110, 140), (80, 140, 140, 200)]), W, H),
            Err("The hand is too small in the image. Crop it closer.")
        );
    }

    #[test]
    fn rejects_a_flat_top() {
        assert_eq!(
            verdict(&image(W, H, &[(40, 20, 160, 280)]), W, H),
            Err("The top of the image should be a single finger pointing up.")
        );
    }

    #[test]
    fn rejects_a_pointed_bottom() {
        assert_eq!(
            verdict(&image(W, H, &[(60, 20, 80, 80), (40, 80, 160, 260), (95, 260, 105, 285)]), W, H),
            Err("The hand should end in a flat wrist or cuff at the bottom.")
        );
    }

    #[test]
    fn rejects_soft_shadows() {
        let mut rgba = image(W, H, &GOOD);
        for y in 80..280 {
            for x in 160..190 {
                rgba[(y * W + x) * 4 + 3] = 150;
            }
        }
        assert_eq!(
            verdict(&rgba, W, H),
            Err("The hand has a soft shadow or glow around it. Ask for no shadow.")
        );
    }

    #[test]
    fn ink_follows_the_dominant_colour_and_caps_brightness() {
        let rgba = image(W, H, &GOOD);
        let ink = ink_color(&Pixels::new(W, H, &rgba));
        assert_eq!(ink, 0xC82828);
        let mut bright = rgba.clone();
        for px in bright.chunks_exact_mut(4) {
            px[..3].copy_from_slice(&[255, 60, 60]);
        }
        let (r, g, b) = crate::catalog::rgb(ink_color(&Pixels::new(W, H, &bright)));
        assert!((r - 0.82).abs() < 0.01 && g < 0.25 && b < 0.25);
    }

    #[test]
    fn grey_hands_get_the_neutral_ink() {
        let mut rgba = image(W, H, &GOOD);
        for px in rgba.chunks_exact_mut(4) {
            px[..3].copy_from_slice(&[128, 128, 128]);
        }
        assert_eq!(ink_color(&Pixels::new(W, H, &rgba)), 0x6B7380);
    }

    #[test]
    fn layout_puts_the_hand_at_the_top_of_the_canvas_on_a_centred_rod() {
        let rgba = image(W, H, &GOOD);
        let l = compose_layout(&Pixels::new(W, H, &rgba)).unwrap();
        let scale = 560.0 / 260.0;
        assert!((l.scale - scale).abs() < 1e-9);
        assert!((l.origin.1 + 20.0 * scale - 150.0).abs() < 1e-9);
        assert!((l.origin.0 + 100.0 * scale - 512.0).abs() < 1e-9);
        let (x, y, w, h) = l.rod;
        assert_eq!(w, 54.0);
        assert!((x + w / 2.0 - 512.0).abs() < 1e-9);
        assert!((y - (150.0 + 560.0 - 30.0)).abs() < 1e-9);
        assert!((y + h - 1462.0).abs() < 1e-9);
    }

    #[test]
    fn prompt_carries_the_idea_or_a_default() {
        assert!(prompt("  knitted wool ").contains("in this style: knitted wool\n"));
        assert!(prompt("").contains("a soft vinyl toy in a bright colour"));
        assert!(!prompt("x").contains("[describe"));
        assert!(!prompt("x").contains("Mac"));
    }
}
