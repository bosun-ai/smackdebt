use std::sync::atomic::{AtomicUsize, Ordering};

static PROCESSES: AtomicUsize = AtomicUsize::new(0);
static OBJECT_READS: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EvidenceSnapshot {
    pub processes: usize,
    pub object_reads: usize,
}

pub fn reset() {
    PROCESSES.store(0, Ordering::Relaxed);
    OBJECT_READS.store(0, Ordering::Relaxed);
}

pub fn snapshot() -> EvidenceSnapshot {
    EvidenceSnapshot {
        processes: PROCESSES.load(Ordering::Relaxed),
        object_reads: OBJECT_READS.load(Ordering::Relaxed),
    }
}

pub(crate) fn record_process() {
    PROCESSES.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn record_object_read() {
    OBJECT_READS.fetch_add(1, Ordering::Relaxed);
}
