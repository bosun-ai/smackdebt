use crate::semantic::CognitiveEvent;

#[derive(Default)]
pub(super) struct CognitiveComplexity {
    value: u32,
    boolean_run: Option<CognitiveEvent>,
}

impl CognitiveComplexity {
    pub(super) fn begin_statement(&mut self) {
        self.boolean_run = None;
    }

    pub(super) fn observe(&mut self, event: Option<CognitiveEvent>, nesting: u32) {
        match event {
            Some(CognitiveEvent::Structural) => {
                self.value = self.value.saturating_add(1 + nesting);
                self.boolean_run = None;
            }
            Some(CognitiveEvent::Alternative | CognitiveEvent::Jump) => {
                self.value = self.value.saturating_add(1);
                self.boolean_run = None;
            }
            Some(event @ (CognitiveEvent::BooleanAnd | CognitiveEvent::BooleanOr))
                if self.boolean_run != Some(event) =>
            {
                self.value = self.value.saturating_add(1);
                self.boolean_run = Some(event);
            }
            Some(CognitiveEvent::BooleanAnd | CognitiveEvent::BooleanOr) => {}
            None => {}
        }
    }

    pub(super) const fn finish(self) -> u32 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_flow_costs_one_plus_nesting() {
        let mut metric = CognitiveComplexity::default();
        metric.observe(Some(CognitiveEvent::Structural), 0);
        metric.observe(Some(CognitiveEvent::Structural), 1);
        assert_eq!(metric.finish(), 3);
    }

    #[test]
    fn boolean_runs_cost_only_when_the_operator_changes() {
        let mut metric = CognitiveComplexity::default();
        metric.observe(Some(CognitiveEvent::BooleanAnd), 0);
        metric.observe(Some(CognitiveEvent::BooleanAnd), 0);
        metric.observe(Some(CognitiveEvent::BooleanOr), 0);
        assert_eq!(metric.finish(), 2);
    }

    #[test]
    fn separate_boolean_expressions_start_separate_runs() {
        let mut metric = CognitiveComplexity::default();
        metric.observe(Some(CognitiveEvent::BooleanAnd), 0);
        metric.begin_statement();
        metric.observe(Some(CognitiveEvent::BooleanAnd), 0);
        assert_eq!(metric.finish(), 2);
    }
}
