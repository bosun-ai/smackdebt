//! Validated history-window durations and timestamp inclusion rules.

/// The selected history window that every history signal describes.
///
/// The window is a pure policy value: it owns the cutoff timestamp and answers
/// whether one streamed commit belongs to the analyzed window. Applying it once,
/// where streamed records become facts, keeps touches, churn, coupling, and
/// concentration describing the same commits.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HistoryWindow {
    days: Option<u32>,
    cutoff: i64,
}

impl HistoryWindow {
    /// A window that includes every locally available commit.
    pub const fn unbounded() -> Self {
        Self {
            days: None,
            cutoff: i64::MIN,
        }
    }

    /// A window of `days` ending at `now`, both in whole seconds since the epoch.
    pub const fn of_days(days: u32, now: i64) -> Self {
        Self {
            days: Some(days),
            cutoff: now.saturating_sub(days as i64 * 86_400),
        }
    }

    /// The selected window length in days, when a window is selected.
    pub const fn days(self) -> Option<u32> {
        self.days
    }

    /// The oldest commit timestamp the window includes.
    pub const fn cutoff(self) -> i64 {
        self.cutoff
    }

    /// Whether a commit with this timestamp belongs to the analyzed window.
    pub const fn includes(self, timestamp: i64) -> bool {
        timestamp >= self.cutoff
    }
}

impl Default for HistoryWindow {
    fn default() -> Self {
        Self::unbounded()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_includes_its_own_cutoff_second_and_excludes_older_commits() {
        let now = 1_000_000_000;
        let window = HistoryWindow::of_days(90, now);
        assert_eq!(window.days(), Some(90));
        assert_eq!(window.cutoff(), now - 90 * 86_400);
        assert!(window.includes(now));
        assert!(window.includes(window.cutoff()));
        assert!(!window.includes(window.cutoff() - 1));
    }

    #[test]
    fn an_unbounded_window_states_no_length_and_includes_every_commit() {
        let window = HistoryWindow::unbounded();
        assert_eq!(window.days(), None);
        assert!(window.includes(i64::MIN));
        assert!(window.includes(i64::MAX));
    }
}
