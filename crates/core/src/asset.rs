pub struct Pixels<'a> {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub rgba: &'a [u8],
}

impl<'a> Pixels<'a> {
    pub fn new(width: usize, height: usize, rgba: &'a [u8]) -> Self {
        Pixels { width, height, stride: width * 4, rgba }
    }

    pub fn alpha(&self, x: usize, y: usize) -> u8 {
        self.rgba[y * self.stride + x * 4 + 3]
    }

    pub fn rgb(&self, x: usize, y: usize) -> [u8; 3] {
        let i = y * self.stride + x * 4;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2]]
    }

    fn solid(&self, y: usize) -> impl DoubleEndedIterator<Item = usize> + '_ {
        (0..self.width).filter(move |&x| self.alpha(x, y) > 128)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandMetrics {
    pub aspect: f64,
    pub hand_top: f64,
    pub tip: f64,
    pub wrist: f64,
    pub rod_end: f64,
    pub tip_x: f64,
    pub rod_x: f64,
    pub hand_min_x: f64,
    pub hand_max_x: f64,
}

pub fn measure(p: &Pixels) -> Option<HandMetrics> {
    let (w, h) = (p.width, p.height);
    if w == 0 || h == 0 || p.rgba.len() < (h - 1) * p.stride + w * 4 {
        return None;
    }
    let counts: Vec<usize> = (0..h).map(|y| p.solid(y).count()).collect();
    let center = |y: usize| match (p.solid(y).next(), p.solid(y).next_back()) {
        (Some(lo), Some(hi)) => (lo + hi) as f64 / 2.0 / w as f64,
        _ => 0.5,
    };

    let top = counts.iter().position(|&c| c > 0)?;
    let search_end = h.min(top + (0.45 * h as f64) as usize);
    let max_w = *counts[top..search_end].iter().max()? as f64;
    let fist = (top..h).find(|&y| counts[y] as f64 > 0.6 * max_w)?;
    let wrist = (fist..h).find(|&y| (counts[y] as f64) < 0.35 * max_w)?;
    let rod_w = counts[(h - 1).min(wrist + h / 50)] as f64;
    let last_row = counts.iter().rposition(|&c| c > 0).unwrap_or(h - 1);
    let from = (h - 1).min(wrist + h / 33);
    let rod_end = (from..h).find(|&y| counts[y] as f64 > 1.8 * rod_w).unwrap_or(last_row);

    let (mut min_x, mut max_x) = (w, 0);
    for y in top..wrist {
        if let (Some(lo), Some(hi)) = (p.solid(y).next(), p.solid(y).next_back()) {
            min_x = min_x.min(lo);
            max_x = max_x.max(hi);
        }
    }

    let fh = h as f64;
    Some(HandMetrics {
        aspect: w as f64 / fh,
        hand_top: (top as f64 - 0.004 * fh).max(0.0) / fh,
        tip: top as f64 / fh,
        wrist: wrist as f64 / fh,
        rod_end: rod_end as f64 / fh,
        tip_x: center((h - 1).min(top + h / 200)),
        rod_x: center((wrist + rod_end) / 2),
        hand_min_x: min_x as f64 / w as f64,
        hand_max_x: max_x as f64 / w as f64,
    })
}

pub fn thumbnail_rect(m: &HandMetrics, w: f64, h: f64) -> (f64, f64, f64, f64) {
    let hand_w = (m.hand_max_x - m.hand_min_x) * w;
    let hand_h = (m.wrist - m.hand_top) * h;
    let side = hand_w.max(hand_h) * 1.06;
    let cx = (m.hand_min_x + m.hand_max_x) / 2.0 * w;
    let cy = (m.hand_top + m.wrist) / 2.0 * h;
    let x0 = (cx - side / 2.0).max(0.0);
    let y0 = (cy - side / 2.0).max(0.0);
    let x1 = (cx + side / 2.0).min(w);
    let y1 = (cy + side / 2.0).min(h);
    (x0, y0, x1 - x0, y1 - y0)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn image(w: usize, h: usize, rects: &[(usize, usize, usize, usize)]) -> Vec<u8> {
        let mut rgba = vec![0u8; w * h * 4];
        for &(x0, y0, x1, y1) in rects {
            for y in y0..y1 {
                for x in x0..x1 {
                    rgba[(y * w + x) * 4..][..4].copy_from_slice(&[200, 40, 40, 255]);
                }
            }
        }
        rgba
    }

    pub const HAND_ON_STICK: [(usize, usize, usize, usize); 3] =
        [(30, 20, 36, 60), (20, 60, 80, 140), (45, 140, 55, 280)];

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn measure_finds_hand_on_stick() {
        let rgba = image(100, 300, &HAND_ON_STICK);
        let m = measure(&Pixels::new(100, 300, &rgba)).unwrap();
        assert!(close(m.aspect, 100.0 / 300.0));
        assert!(close(m.tip, 20.0 / 300.0));
        assert!(close(m.hand_top, 18.8 / 300.0));
        assert!(close(m.wrist, 140.0 / 300.0));
        assert!(close(m.rod_end, 279.0 / 300.0));
        assert!(close(m.tip_x, 0.325));
        assert!(close(m.rod_x, 0.495));
        assert!(close(m.hand_min_x, 0.20));
        assert!(close(m.hand_max_x, 0.79));
    }

    #[test]
    fn measure_stops_the_rod_at_a_wider_handle() {
        let mut rects = HAND_ON_STICK.to_vec();
        rects.push((35, 250, 65, 280));
        let rgba = image(100, 300, &rects);
        let m = measure(&Pixels::new(100, 300, &rgba)).unwrap();
        assert!(close(m.rod_end, 250.0 / 300.0));
    }

    #[test]
    fn measure_rejects_empty() {
        let rgba = image(100, 300, &[]);
        assert!(measure(&Pixels::new(100, 300, &rgba)).is_none());
    }

    #[test]
    fn measure_rejects_a_shape_without_a_wrist() {
        let rgba = image(100, 300, &[(20, 20, 80, 300)]);
        assert!(measure(&Pixels::new(100, 300, &rgba)).is_none());
    }

    #[test]
    fn thumbnail_is_a_square_around_the_hand() {
        let rgba = image(100, 300, &HAND_ON_STICK);
        let m = measure(&Pixels::new(100, 300, &rgba)).unwrap();
        let (x, y, w, h) = thumbnail_rect(&m, 100.0, 300.0);
        assert!(x >= 0.0 && y >= 0.0 && x + w <= 100.0 && y + h <= 300.0);
        assert!(x <= 20.0 && x + w >= 80.0);
        assert!(y <= 20.0 && y + h >= 140.0);
    }
}
