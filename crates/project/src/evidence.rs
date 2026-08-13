use std::sync::atomic::{AtomicUsize, Ordering};

static INVENTORY_WALKS: AtomicUsize = AtomicUsize::new(0);
static INVENTORY_VISITS: AtomicUsize = AtomicUsize::new(0);
static SOURCE_READS: AtomicUsize = AtomicUsize::new(0);
static PARSER_VISITS: AtomicUsize = AtomicUsize::new(0);
static ALGORITHM_PASSES: AtomicUsize = AtomicUsize::new(0);
static RENDERER_ENTRIES: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EvidenceSnapshot {
    inventory_walks: usize,
    inventory_visits: usize,
    source_reads: usize,
    object_reads: usize,
    git_processes: usize,
    parser_visits: usize,
    algorithm_passes: usize,
    renderer_entries: usize,
}

impl EvidenceSnapshot {
    pub const fn inventory_walks(self) -> usize {
        self.inventory_walks
    }

    pub const fn inventory_visits(self) -> usize {
        self.inventory_visits
    }

    pub const fn source_reads(self) -> usize {
        self.source_reads
    }

    pub const fn object_reads(self) -> usize {
        self.object_reads
    }

    pub const fn git_processes(self) -> usize {
        self.git_processes
    }

    pub const fn parser_visits(self) -> usize {
        self.parser_visits
    }

    pub const fn algorithm_passes(self) -> usize {
        self.algorithm_passes
    }

    pub const fn renderer_entries(self) -> usize {
        self.renderer_entries
    }

    pub fn since(self, earlier: Self) -> Self {
        Self {
            inventory_walks: self.inventory_walks.saturating_sub(earlier.inventory_walks),
            inventory_visits: self
                .inventory_visits
                .saturating_sub(earlier.inventory_visits),
            source_reads: self.source_reads.saturating_sub(earlier.source_reads),
            object_reads: self.object_reads.saturating_sub(earlier.object_reads),
            git_processes: self.git_processes.saturating_sub(earlier.git_processes),
            parser_visits: self.parser_visits.saturating_sub(earlier.parser_visits),
            algorithm_passes: self
                .algorithm_passes
                .saturating_sub(earlier.algorithm_passes),
            renderer_entries: self
                .renderer_entries
                .saturating_sub(earlier.renderer_entries),
        }
    }
}

pub fn reset() {
    INVENTORY_WALKS.store(0, Ordering::Relaxed);
    INVENTORY_VISITS.store(0, Ordering::Relaxed);
    SOURCE_READS.store(0, Ordering::Relaxed);
    PARSER_VISITS.store(0, Ordering::Relaxed);
    ALGORITHM_PASSES.store(0, Ordering::Relaxed);
    RENDERER_ENTRIES.store(0, Ordering::Relaxed);
    smackdebt_git::reset_evidence();
}

pub fn snapshot() -> EvidenceSnapshot {
    let git = smackdebt_git::evidence_snapshot();
    EvidenceSnapshot {
        inventory_walks: INVENTORY_WALKS.load(Ordering::Relaxed),
        inventory_visits: INVENTORY_VISITS.load(Ordering::Relaxed),
        source_reads: SOURCE_READS.load(Ordering::Relaxed),
        object_reads: git.object_reads,
        git_processes: git.processes,
        parser_visits: PARSER_VISITS.load(Ordering::Relaxed),
        algorithm_passes: ALGORITHM_PASSES.load(Ordering::Relaxed),
        renderer_entries: RENDERER_ENTRIES.load(Ordering::Relaxed),
    }
}

pub fn record_renderer_entry() {
    RENDERER_ENTRIES.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn record_inventory(visits: usize) {
    INVENTORY_WALKS.fetch_add(1, Ordering::Relaxed);
    INVENTORY_VISITS.fetch_add(visits, Ordering::Relaxed);
}

pub(crate) fn record_source_read() {
    SOURCE_READS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn record_parser_visit() {
    PARSER_VISITS.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn record_algorithm_pass() {
    ALGORITHM_PASSES.fetch_add(1, Ordering::Relaxed);
}
