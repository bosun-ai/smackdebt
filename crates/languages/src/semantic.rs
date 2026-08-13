#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CognitiveEvent {
    Structural,
    Alternative,
    BooleanAnd,
    BooleanOr,
    Jump,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Syntax {
    pub(super) cognitive: Option<CognitiveEvent>,
    pub(super) secondary_cognitive: Option<CognitiveEvent>,
    pub(super) decision: bool,
    pub(super) logical_statement: bool,
    pub(super) nests: bool,
}

impl Syntax {
    pub(super) const fn structural(decision: bool) -> Self {
        Self {
            cognitive: Some(CognitiveEvent::Structural),
            secondary_cognitive: None,
            decision,
            logical_statement: false,
            nests: true,
        }
    }

    pub(super) const fn alternative(decision: bool) -> Self {
        Self {
            cognitive: Some(CognitiveEvent::Alternative),
            secondary_cognitive: None,
            decision,
            logical_statement: false,
            nests: false,
        }
    }

    pub(super) const fn boolean(event: CognitiveEvent) -> Self {
        Self {
            cognitive: Some(event),
            secondary_cognitive: None,
            decision: true,
            logical_statement: false,
            nests: false,
        }
    }

    pub(super) const fn statement() -> Self {
        Self {
            cognitive: None,
            secondary_cognitive: None,
            decision: false,
            logical_statement: true,
            nests: false,
        }
    }

    pub(super) const fn with_alternative(mut self) -> Self {
        self.secondary_cognitive = Some(CognitiveEvent::Alternative);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_values_describe_meaning_without_grammar_names() {
        assert_eq!(
            Syntax::structural(true).cognitive,
            Some(CognitiveEvent::Structural)
        );
        assert!(Syntax::statement().logical_statement);
    }
}
