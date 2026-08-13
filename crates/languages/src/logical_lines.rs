#[derive(Default)]
pub(super) struct LogicalLines(u32);

impl LogicalLines {
    pub(super) fn observe(&mut self, statement: bool) {
        self.0 = self.0.saturating_add(u32::from(statement));
    }

    pub(super) const fn finish(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_events_count_instead_of_physical_lines() {
        let mut metric = LogicalLines::default();
        metric.observe(true);
        metric.observe(true);
        metric.observe(false);
        assert_eq!(metric.finish(), 2);
    }
}
