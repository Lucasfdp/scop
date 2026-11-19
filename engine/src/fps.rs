//! Frames per second (§ 19): counted over a short window, not `1 ÷ dt` per frame,
//! so the number is steady enough to read (Q-19.2).

/// One averaged reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FpsReading {
    /// Frames finished per second, averaged over the window.
    pub fps: f32,
    /// Milliseconds per frame (`1000 ÷ fps`): the number to compare, because it adds up.
    pub frame_ms: f32,
}

/// Counts frames and time; every `window` seconds it gives a reading and starts again.
/// The app owns one and calls `tick(dt)` once per frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FpsCounter {
    window: f32,
    frames: u32,
    elapsed: f32,
}

impl FpsCounter {
    /// A counter that reports every `window` seconds (scop: 0.5).
    pub fn new(window: f32) -> Self {
        FpsCounter {
            window,
            frames: 0,
            elapsed: 0.0,
        }
    }

    /// One frame took `dt` seconds. Returns a reading when the window is full,
    /// `None` the rest of the time.
    pub fn tick(&mut self, dt: f32) -> Option<FpsReading> {
        self.frames += 1;
        self.elapsed += dt;
        if self.elapsed < self.window {
            return None;
        }
        let reading = FpsReading {
            fps: self.frames as f32 / self.elapsed,
            frame_ms: self.elapsed * 1000.0 / self.frames as f32,
        };
        self.frames = 0;
        self.elapsed = 0.0;
        Some(reading)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Frame times below are exact in binary (1/128 s and friends), so the sums hit
    // 0.5 s exactly; with 1/120 the rounding decides which frame tips it over.

    #[test]
    fn averages_over_the_window_then_starts_again() {
        let mut counter = FpsCounter::new(0.5);
        let dt = 1.0 / 128.0;
        // 63 frames: not half a second yet.
        for _ in 0..63 {
            assert_eq!(counter.tick(dt), None);
        }
        // The 64th fills the window: 64 frames in 0.5 s.
        let reading = counter.tick(dt).unwrap();
        assert_eq!(reading.fps, 128.0);
        assert_eq!(reading.frame_ms, 7.8125);
        // Counting restarted: the next frame alone reports nothing.
        assert_eq!(counter.tick(dt), None);
    }

    #[test]
    fn uneven_frames_average_out() {
        // Jitter: alternating short and long frames — `1 ÷ dt` would flip between
        // 256 and 85 fps every frame; the average is a steady 128.
        let mut counter = FpsCounter::new(0.5);
        let mut last = None;
        for i in 0..64 {
            let dt = if i % 2 == 0 { 1.0 / 256.0 } else { 3.0 / 256.0 };
            last = counter.tick(dt).or(last);
        }
        assert_eq!(last.unwrap().fps, 128.0);
    }
}
