use std::time::{Duration, Instant};

const FRAME_DURATION: Duration = Duration::from_millis(133);

/// Braille-based spinner frames evoking a rotating three-point pattern.
/// 12 frames cycling at ~133ms each = 1.6s full rotation.
const FRAMES: &[&str] = &[
    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "⠛", "⠿",
];

pub struct SpinnerState {
    frame: usize,
    last_tick: Instant,
}

impl SpinnerState {
    pub fn new() -> Self {
        Self {
            frame: 0,
            last_tick: Instant::now(),
        }
    }

    pub fn tick(&mut self) {
        if self.last_tick.elapsed() >= FRAME_DURATION {
            self.frame = (self.frame + 1) % FRAMES.len();
            self.last_tick = Instant::now();
        }
    }

    pub fn current_frame(&self) -> &'static str {
        FRAMES[self.frame]
    }
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self::new()
    }
}
