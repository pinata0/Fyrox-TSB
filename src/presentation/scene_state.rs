//! Fyrox 1.0.1 notifies the graph even when a tracked property is assigned its old value.
use fyrox::scene::base::Base;

/// Executor subtracts this tick from lag after Plugin::post_update. A throttled
/// tick has lag=0, and also needs a presentation update.
pub(super) fn presentation_due(lag: f32, dt: f32) -> bool {
    lag < 2.0 * dt - dt * 0.0001
}

#[cfg(test)]
mod tests {
    use super::presentation_due;

    #[test]
    fn catch_up_presents_last_tick_and_throttled_tick() {
        let dt = 1.0 / 60.0;
        assert!(!presentation_due(3.0 * dt, dt));
        assert!(!presentation_due(2.0 * dt, dt));
        assert!(presentation_due(1.5 * dt, dt));
        assert!(presentation_due(dt, dt));
        assert!(presentation_due(0.0, 0.3));
    }
}

pub(super) trait SceneState {
    /// Returns the previous visibility, matching Base::set_visibility.
    fn set_visible(&mut self, visible: bool) -> bool;
    fn set_active(&mut self, enabled: bool);
}

impl SceneState for Base {
    #[inline]
    fn set_visible(&mut self, visible: bool) -> bool {
        let previous = self.visibility();
        if previous != visible {
            self.set_visibility(visible);
        }
        previous
    }

    #[inline]
    fn set_active(&mut self, enabled: bool) {
        if self.is_enabled() != enabled {
            self.set_enabled(enabled);
        }
    }
}
