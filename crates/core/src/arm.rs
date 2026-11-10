use crate::asset::HandMetrics;

const GRIP_LEAN: f64 = 0.30;
const GRIP_DEPTH: f64 = 0.12;
const GRIP_FOLLOW: f64 = 0.08;
const PRESS_DECAY: f64 = 0.18;
const PRESS_SHRINK: f64 = 0.06;
const MIN_ROD: f64 = 0.05;
const LIFT: f64 = 0.06;
const UPSTREAM_HZ: f64 = 120.0;
const REST: f64 = 0.002;

fn blend(per_tick: f64, dt: f64) -> f64 {
    1.0 - (1.0 - per_tick).powf(dt * UPSTREAM_HZ)
}

#[derive(Clone, Debug, Default)]
pub struct ArmState {
    pub press: f64,
    pub held: bool,
    grip_x: Option<f64>,
    grip_target: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArmPose {
    pub grip: (f64, f64),
    pub rotation: f64,
    pub rod_len: f64,
    pub hand_h: f64,
    pub width: f64,
    pub rod_x: f64,
    pub tip: (f64, f64),
    pub lift: f64,
    pub shadow_offset: (f64, f64),
    pub shadow_radius: f64,
    pub shadow_alpha: f64,
}

impl ArmState {
    pub fn press(&mut self) {
        self.press = 1.0;
        self.held = true;
    }

    pub fn release(&mut self) {
        self.held = false;
    }

    pub fn reset(&mut self) {
        *self = ArmState::default();
    }

    pub fn step(&mut self, dt: f64, cursor_x: f64, width: f64) {
        let target = (cursor_x + GRIP_LEAN * width).clamp(0.15 * width, 1.05 * width);
        self.grip_x = Some(match self.grip_x {
            Some(x) => x + (target - x) * blend(GRIP_FOLLOW, dt),
            None => target,
        });
        self.grip_target = target;
        if !self.held {
            self.press -= self.press * blend(PRESS_DECAY, dt);
            if self.press < REST {
                self.press = 0.0;
            }
        }
    }

    pub fn is_settling(&self) -> bool {
        (self.press > REST && !self.held) || self.grip_x.is_some_and(|x| (self.grip_target - x).abs() > 0.5)
    }

    pub fn pose(&self, cursor: (f64, f64), size: (f64, f64), m: &HandMetrics, hand_frac: f64) -> ArmPose {
        let (w, h) = size;
        let grip_x = self.grip_x.unwrap_or_else(|| (cursor.0 + GRIP_LEAN * w).clamp(0.15 * w, 1.05 * w));
        let grip = (grip_x, h + GRIP_DEPTH * h);
        let dx = cursor.0 - grip.0;
        let up = grip.1 - cursor.1;
        let reach = dx.hypot(up);
        let aim = up.atan2(dx);

        let hand_h = hand_frac * h * (1.0 - PRESS_SHRINK * self.press);
        let scale = hand_h / (m.wrist - m.hand_top);
        let width = scale * m.aspect;
        let hand_tip = (m.wrist - m.tip) * scale;
        let tip_off = (m.tip_x - m.rod_x) * width;
        let along = (reach * reach - tip_off * tip_off).max(0.0).sqrt();
        let rod_len = (along - hand_tip).max(MIN_ROD * h);
        let tip_angle = (rod_len + hand_tip).atan2(tip_off);

        let lift = LIFT * h * (1.0 - 0.8 * self.press);
        ArmPose {
            grip,
            rotation: tip_angle - aim,
            rod_len,
            hand_h,
            width,
            rod_x: m.rod_x * width,
            tip: (tip_off, -(rod_len + hand_tip)),
            lift,
            shadow_offset: (0.6 * lift, lift),
            shadow_radius: (0.35 * lift).max(1.0) + 5.0 * self.press,
            shadow_alpha: 0.26 + 0.02 * self.press,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Laser {
    pub r: f64,
    pub core: f64,
    pub glow: f64,
}

pub fn laser(press: f64, height: f64, hand_frac: f64) -> Laser {
    let r = (height * 0.0038 * hand_frac / 0.13).max(4.0) * (1.0 + 0.2 * press);
    Laser { r, core: r * 0.45, glow: r * (2.2 + press) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::HandMetrics;

    const METRICS: HandMetrics = HandMetrics {
        aspect: 1024.0 / 1536.0,
        hand_top: 0.094,
        tip: 0.098,
        wrist: 0.46,
        rod_end: 0.95,
        tip_x: 0.40,
        rod_x: 0.50,
        hand_min_x: 0.2,
        hand_max_x: 0.8,
    };
    const SCREEN: (f64, f64) = (1440.0, 960.0);

    fn settled(cursor_x: f64) -> ArmState {
        let mut arm = ArmState::default();
        arm.step(1.0 / 120.0, cursor_x, SCREEN.0);
        arm
    }

    fn tip_on_screen(pose: &ArmPose) -> (f64, f64) {
        let (x, y) = pose.tip;
        let (sin, cos) = pose.rotation.sin_cos();
        (pose.grip.0 + x * cos - y * sin, pose.grip.1 + x * sin + y * cos)
    }

    #[test]
    fn fingertip_lands_on_cursor() {
        for &cursor in &[(100.0, 100.0), (720.0, 480.0), (1400.0, 50.0), (10.0, 600.0), (900.0, 300.0)] {
            let pose = settled(cursor.0).pose(cursor, SCREEN, &METRICS, 0.13);
            assert!(pose.rod_len > 0.05 * SCREEN.1 + 1.0, "rod clamped at {cursor:?}");
            let tip = tip_on_screen(&pose);
            assert!((tip.0 - cursor.0).abs() < 0.5 && (tip.1 - cursor.1).abs() < 0.5, "{tip:?} vs {cursor:?}");
        }
    }

    #[test]
    fn grip_sits_below_the_screen_and_leans_right() {
        let pose = settled(400.0).pose((400.0, 300.0), SCREEN, &METRICS, 0.13);
        assert!((pose.grip.1 - SCREEN.1 * 1.12).abs() < 1e-9);
        assert!((pose.grip.0 - (400.0 + 0.30 * SCREEN.0)).abs() < 1e-9);
    }

    #[test]
    fn rod_never_gets_shorter_than_the_minimum() {
        let pose = settled(700.0).pose((1100.0, 955.0), SCREEN, &METRICS, 0.20);
        assert!(pose.rod_len >= 0.05 * SCREEN.1 - 1e-9);
    }

    #[test]
    fn press_shrinks_the_hand_and_drops_the_lift() {
        let mut arm = settled(400.0);
        let rest = arm.pose((400.0, 300.0), SCREEN, &METRICS, 0.13);
        arm.press();
        let pressed = arm.pose((400.0, 300.0), SCREEN, &METRICS, 0.13);
        assert!((pressed.hand_h - rest.hand_h * 0.94).abs() < 1e-9);
        assert!((pressed.lift - rest.lift * 0.2).abs() < 1e-9);
        assert!((rest.shadow_offset.0 - 0.6 * rest.lift).abs() < 1e-9);
        assert!((rest.shadow_offset.1 - rest.lift).abs() < 1e-9);
    }

    #[test]
    fn press_holds_while_held_then_decays() {
        let mut arm = settled(400.0);
        arm.press();
        for _ in 0..120 {
            arm.step(1.0 / 120.0, 400.0, SCREEN.0);
        }
        assert_eq!(arm.press, 1.0);
        arm.release();
        for _ in 0..120 {
            arm.step(1.0 / 120.0, 400.0, SCREEN.0);
        }
        assert!(!arm.is_settling());
    }

    #[test]
    fn grip_converges() {
        let mut arm = settled(100.0);
        arm.step(1.0 / 120.0, 900.0, SCREEN.0);
        assert!(arm.is_settling());
        for _ in 0..240 {
            arm.step(1.0 / 120.0, 900.0, SCREEN.0);
        }
        assert!(!arm.is_settling());
    }

    #[test]
    fn step_is_frame_rate_independent() {
        let mut fast = settled(100.0);
        let mut slow = settled(100.0);
        for _ in 0..120 {
            fast.step(1.0 / 120.0, 900.0, SCREEN.0);
        }
        for _ in 0..60 {
            slow.step(1.0 / 60.0, 900.0, SCREEN.0);
        }
        let a = fast.pose((900.0, 300.0), SCREEN, &METRICS, 0.13).grip.0;
        let b = slow.pose((900.0, 300.0), SCREEN, &METRICS, 0.13).grip.0;
        assert!((a - b).abs() < 0.01 * a, "{a} vs {b}");
    }

    #[test]
    fn one_upstream_tick_moves_the_grip_by_eight_percent() {
        let mut arm = settled(100.0);
        let before = arm.pose((100.0, 300.0), SCREEN, &METRICS, 0.13).grip.0;
        arm.step(1.0 / 120.0, 600.0, SCREEN.0);
        let after = arm.pose((600.0, 300.0), SCREEN, &METRICS, 0.13).grip.0;
        assert!((after - before - 0.08 * 500.0).abs() < 1e-6);
    }

    #[test]
    fn laser_grows_with_size_and_press() {
        let rest = laser(0.0, 960.0, 0.13);
        assert!((rest.r - 4.0).abs() < 1e-9);
        let big = laser(0.0, 2160.0, 0.20);
        assert!((big.r - 2160.0 * 0.0038 * 0.20 / 0.13).abs() < 1e-9);
        let pressed = laser(1.0, 2160.0, 0.20);
        assert!((pressed.r - big.r * 1.2).abs() < 1e-9);
        assert!((pressed.glow - pressed.r * 3.2).abs() < 1e-9);
        assert!((rest.core - rest.r * 0.45).abs() < 1e-9);
    }
}
