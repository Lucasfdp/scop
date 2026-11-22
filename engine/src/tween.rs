//! A value that eases towards a target (§ 15's fade — moved into the engine in § 24).
//!
//! A dimmer, not a light switch: `toggle` flips where it's heading; every frame
//! `step` moves it a little towards there at a constant speed, never past it; `value`
//! eases that progress so it starts and stops gently.

/// How the progress is shaped on the way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    /// Constant speed.
    Linear,
    /// Slow – fast – slow (smoothstep): no kick at either end (Q-15.5).
    Smooth,
}

/// Between 0 and 1, heading to 0 or 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    /// 0 or 1: where it's heading.
    pub target: f32,
    /// How far along it is, 0..1, before easing.
    pub progress: f32,
    /// Seconds for a whole 0 → 1.
    pub duration: f32,
    pub easing: Easing,
}

impl Tween {
    /// At 0, heading to 0.
    pub fn new(duration: f32, easing: Easing) -> Self {
        Tween {
            target: 0.0,
            progress: 0.0,
            duration,
            easing,
        }
    }

    /// Turn the dimmer the other way. Only the target changes; the progress carries
    /// on from where it is, so pressing mid-way never jumps (Q-15.3).
    pub fn toggle(&mut self) {
        self.target = 1.0 - self.target;
    }

    /// One frame of `dt` seconds: at most `dt / duration` towards the target, never
    /// past it (the clamp), so it arrives and stays.
    pub fn step(&mut self, dt: f32) {
        let max_step = if self.duration > 0.0 {
            dt / self.duration
        } else {
            1.0
        };
        let gap = self.target - self.progress;
        self.progress += gap.clamp(-max_step, max_step);
    }

    /// The eased value, 0..1.
    pub fn value(&self) -> f32 {
        match self.easing {
            Easing::Linear => self.progress,
            Easing::Smooth => smoothstep(self.progress),
        }
    }
}

/// The classic easing curve `t² × (3 − 2t)`: 0 → 0, 0.5 → 0.5, 1 → 1, with zero
/// speed at both ends.
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrives_and_stays() {
        let mut tween = Tween::new(0.6, Easing::Smooth);
        tween.toggle();
        for _ in 0..100 {
            tween.step(0.016); // 1.6 s > 0.6 s
        }
        assert_eq!(tween.progress, 1.0); // arrived, not overshot
        assert_eq!(tween.value(), 1.0);
    }

    #[test]
    fn reverses_mid_way_without_a_jump() {
        let mut tween = Tween::new(0.6, Easing::Smooth);
        tween.toggle();
        tween.step(0.6 * 0.4); // 40 % of the way
        let before = tween.progress;
        tween.toggle();
        assert_eq!(tween.progress, before); // pressing changed only the target
        tween.step(0.016);
        assert!(tween.progress < before && tween.progress > before - 0.05);
    }

    #[test]
    fn smoothstep_keeps_ends_and_middle() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(0.5), 0.5);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!(smoothstep(0.1) < 0.1); // slow start
        assert!(smoothstep(0.9) > 0.9); // slow finish
    }

    #[test]
    fn linear_is_the_progress_itself() {
        let mut tween = Tween::new(1.0, Easing::Linear);
        tween.toggle();
        tween.step(0.25);
        assert_eq!(tween.value(), 0.25);
    }
}
