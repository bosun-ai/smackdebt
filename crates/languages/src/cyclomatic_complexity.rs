pub(super) struct CyclomaticComplexity(u32);

impl Default for CyclomaticComplexity {
    fn default() -> Self {
        Self(1)
    }
}

impl CyclomaticComplexity {
    pub(super) fn observe(&mut self, decision: bool) {
        self.0 = self.0.saturating_add(u32::from(decision));
    }

    pub(super) const fn finish(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_decisions_make_complexity_four() {
        let mut metric = CyclomaticComplexity::default();
        for _ in 0..3 {
            metric.observe(true);
        }
        assert_eq!(metric.finish(), 4);
    }
}
