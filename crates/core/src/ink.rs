pub const MIN_STEP: f64 = 1.5;
pub const START_DISTANCE: f64 = 4.0;
pub const FADE_DURATION: f64 = 0.6;

pub type Point = (f64, f64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    Quad(Point, Point),
    Line(Point),
}

pub fn width(height: f64) -> f64 {
    (height * 0.0055).max(3.0)
}

pub fn far_enough(a: Point, b: Point, distance: f64) -> bool {
    (b.0 - a.0).hypot(b.1 - a.1) >= distance
}

pub fn life(fade_seconds: f64) -> Option<f64> {
    (fade_seconds > 0.0).then(|| fade_seconds.max(0.5) + FADE_DURATION)
}

pub fn opacity(since_end: f64, fade_seconds: f64) -> Option<f64> {
    let Some(life) = life(fade_seconds) else {
        return Some(1.0);
    };
    let t = (since_end - (life - FADE_DURATION)) / FADE_DURATION;
    match t {
        t if t <= 0.0 => Some(1.0),
        t if t >= 1.0 => None,
        t => Some(1.0 - t * t),
    }
}

pub fn segments(points: &[Point]) -> Vec<Segment> {
    let mut out: Vec<Segment> = points
        .windows(2)
        .map(|pair| {
            let (prev, p) = (pair[0], pair[1]);
            Segment::Quad(prev, ((prev.0 + p.0) / 2.0, (prev.1 + p.1) / 2.0))
        })
        .collect();
    if let Some(&last) = points.last() {
        out.push(Segment::Line(last));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_scales_with_screen_height() {
        assert_eq!(width(400.0), 3.0);
        assert!((width(2000.0) - 11.0).abs() < 1e-9);
    }

    #[test]
    fn opacity_timeline() {
        assert_eq!(opacity(2.9, 3.0), Some(1.0));
        let mid = opacity(3.3, 3.0).unwrap();
        assert!(mid > 0.0 && mid < 1.0);
        assert!(opacity(3.5, 3.0).unwrap() < mid);
        assert_eq!(opacity(3.7, 3.0), None);
        assert_eq!(opacity(1000.0, 0.0), Some(1.0));
    }

    #[test]
    fn short_fades_still_wait_half_a_second() {
        assert_eq!(opacity(0.4, 0.1), Some(1.0));
        assert_eq!(life(0.1), Some(1.1));
        assert_eq!(life(0.0), None);
    }

    #[test]
    fn points_closer_than_the_step_are_dropped() {
        assert!(!far_enough((0.0, 0.0), (1.0, 1.0), MIN_STEP));
        assert!(far_enough((0.0, 0.0), (1.5, 0.0), MIN_STEP));
        assert!(!far_enough((0.0, 0.0), (3.0, 2.0), START_DISTANCE));
        assert!(far_enough((0.0, 0.0), (4.0, 0.0), START_DISTANCE));
    }

    #[test]
    fn segments_follow_upstream_smoothing() {
        assert_eq!(segments(&[(0.0, 0.0)]), vec![Segment::Line((0.0, 0.0))]);
        assert_eq!(
            segments(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]),
            vec![
                Segment::Quad((0.0, 0.0), (5.0, 0.0)),
                Segment::Quad((10.0, 0.0), (10.0, 5.0)),
                Segment::Line((10.0, 10.0)),
            ]
        );
    }
}
