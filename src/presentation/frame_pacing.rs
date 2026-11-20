use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub(super) struct FramePacing {
    previous: Option<Instant>,
}

impl FramePacing {
    pub(super) fn wait(&mut self, limit: u32) {
        if limit > 0 {
            if let Some(previous) = self.previous {
                let period = Duration::from_secs_f64(1.0 / limit as f64);
                if let Some(remaining) = period.checked_sub(previous.elapsed()) {
                    std::thread::sleep(remaining);
                }
            }
        }
        self.previous = Some(Instant::now());
    }
}
