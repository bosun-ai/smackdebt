//! The observable work counters one analysis accumulates.

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

#[derive(Clone, Debug, Default)]
pub(crate) struct AnalysisWork {
    pub(crate) source_reads: Arc<AtomicUsize>,
}
impl AnalysisWork {
    #[cfg(feature = "evidence-stats")]
    pub(crate) fn record_algorithm_pass(&self) {
        crate::evidence::record_algorithm_pass();
    }

    #[cfg(not(feature = "evidence-stats"))]
    pub(crate) fn record_algorithm_pass(&self) {}

    #[cfg(feature = "evidence-stats")]
    pub(crate) fn record_parser_visit(&self) {
        crate::evidence::record_parser_visit();
    }

    #[cfg(not(feature = "evidence-stats"))]
    pub(crate) fn record_parser_visit(&self) {}
}
