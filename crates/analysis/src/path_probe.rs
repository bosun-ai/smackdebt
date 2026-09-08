//! Reusable dependency traversal with explicit unknown results at search limits.

use std::collections::VecDeque;

/// What a limited path probe settled about one ordered pair of nodes.
///
/// `Undecided` is a first-class answer rather than a failure: a probe that
/// spent its whole budget without settling the question proves nothing, and a
/// finding that claims two files have no dependency between them must never
/// rest on a search that ran out of room. `Separate` is the strongest answer
/// this type carries, because it is the one that creates a finding, so every
/// path that is not a completed search answers `Undecided` instead.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReachAnswer {
    Reaches,
    Separate,
    Undecided,
}

/// A reusable backward walk over one graph.
///
/// The reverse adjacency is built once and answers many probes, because one
/// probe visits at most its node budget while building the adjacency costs the
/// whole graph: rebuilding it per pair would make the cheap answer the
/// expensive one. The visit marks and the queue are reused between probes for
/// the same reason, so a probe allocates nothing.
pub struct PathProbe {
    incoming: Vec<Vec<usize>>,
    visited: Vec<u64>,
    probes: u64,
    pending: VecDeque<usize>,
}

impl PathProbe {
    /// Prepares probes over a graph whose edges read "source depends on
    /// target".
    pub fn over(node_count: usize, edges: &[(usize, usize)]) -> Self {
        Self {
            incoming: incoming_nodes(node_count, edges),
            visited: vec![0; node_count],
            probes: 0,
            pending: VecDeque::new(),
        }
    }

    /// Whether a path leads from `from` to `to`, within a node budget.
    ///
    /// The walk starts at `to` and follows incoming edges breadth first, so it
    /// explores only what can reach the target. It visits at most `budget`
    /// nodes and answers `Separate` only after exhausting the whole set that
    /// reaches `to`, so absence is proved rather than inferred: a walk that
    /// runs out of budget answers `Undecided` instead. Neighbours are visited
    /// in node order, so a tight budget settles the same question the same way
    /// on every run.
    ///
    /// A node index this graph does not hold is a caller mistake, so it trips a
    /// debug assertion; where assertions are off it answers `Undecided` rather
    /// than `Separate`, because `Separate` is the answer that creates a finding
    /// and a stale index must never manufacture one.
    pub fn reaches(&mut self, from: usize, to: usize, budget: usize) -> ReachAnswer {
        let inside = from < self.incoming.len() && to < self.incoming.len();
        debug_assert!(inside, "a path probe reads node indexes of its own graph");
        if !inside {
            return ReachAnswer::Undecided;
        }
        if from == to {
            return ReachAnswer::Reaches;
        }
        if budget == 0 {
            return ReachAnswer::Undecided;
        }
        self.walk_back(from, to, budget)
    }

    /// How many nodes transitively depend on `node`, **excluding itself**.
    ///
    /// This is the other reach convention: the scoped reach sentences count the
    /// changed node, while card evidence states how many *other* files a change
    /// here reaches, so the walk returns what it visited beyond its start. The
    /// walk is exhaustive rather than budgeted, because an exact count has no
    /// undecided answer; the caller bounds the cost by bounding how many nodes
    /// it asks about.
    pub fn dependents(&mut self, node: usize) -> u32 {
        let inside = node < self.incoming.len();
        debug_assert!(inside, "a path probe reads node indexes of its own graph");
        if !inside {
            return 0;
        }
        let Self {
            incoming,
            visited,
            probes,
            pending,
        } = self;
        *probes += 1;
        pending.clear();
        pending.push_back(node);
        visited[node] = *probes;
        let mut dependents = 0;
        while let Some(current) = pending.pop_front() {
            for &source in &incoming[current] {
                if visited[source] == *probes {
                    continue;
                }
                visited[source] = *probes;
                dependents += 1;
                pending.push_back(source);
            }
        }
        dependents
    }

    /// Walks back from `to` until it meets `from`, exhausts what reaches `to`,
    /// or spends its node budget.
    fn walk_back(&mut self, from: usize, to: usize, budget: usize) -> ReachAnswer {
        let Self {
            incoming,
            visited,
            probes,
            pending,
        } = self;
        *probes += 1;
        pending.clear();
        pending.push_back(to);
        visited[to] = *probes;
        let mut seen = 1;
        while let Some(node) = pending.pop_front() {
            for &source in &incoming[node] {
                if visited[source] == *probes {
                    continue;
                }
                if seen == budget {
                    return ReachAnswer::Undecided;
                }
                visited[source] = *probes;
                seen += 1;
                if source == from {
                    return ReachAnswer::Reaches;
                }
                pending.push_back(source);
            }
        }
        ReachAnswer::Separate
    }
}

/// The nodes with an edge into each node, in node order.
fn incoming_nodes(node_count: usize, edges: &[(usize, usize)]) -> Vec<Vec<usize>> {
    let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); node_count];
    let inside = edges
        .iter()
        .filter(|&&(source, target)| source < node_count && target < node_count);
    for &(source, target) in inside {
        incoming[target].push(source);
    }
    for sources in incoming.iter_mut() {
        sources.sort_unstable();
        sources.dedup();
    }
    incoming
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodes_with_no_path_between_them_are_separate() {
        let mut probe = PathProbe::over(4, &[(0, 1), (2, 3)]);
        assert_eq!(probe.reaches(0, 3, 4_096), ReachAnswer::Separate);
    }

    #[test]
    fn a_three_hop_path_reaches() {
        let mut probe = PathProbe::over(4, &[(0, 1), (1, 2), (2, 3)]);
        assert_eq!(probe.reaches(0, 3, 4_096), ReachAnswer::Reaches);
        // The walk follows the edge direction, so the far end reaches nothing.
        assert_eq!(probe.reaches(3, 0, 4_096), ReachAnswer::Separate);
        assert_eq!(probe.reaches(2, 2, 0), ReachAnswer::Reaches);
    }

    #[test]
    fn one_probe_answers_many_pairs_from_one_reverse_graph() {
        let mut probe = PathProbe::over(4, &[(0, 1), (1, 2), (2, 3)]);
        // Repeating a pair and interleaving pairs must not leak visit marks
        // from an earlier walk into a later one.
        assert_eq!(probe.reaches(0, 3, 4_096), ReachAnswer::Reaches);
        assert_eq!(probe.reaches(3, 0, 4_096), ReachAnswer::Separate);
        assert_eq!(probe.reaches(0, 3, 4_096), ReachAnswer::Reaches);
        assert_eq!(probe.reaches(1, 2, 4_096), ReachAnswer::Reaches);
        assert_eq!(probe.reaches(0, 3, 3), ReachAnswer::Undecided);
        assert_eq!(probe.reaches(0, 3, 4), ReachAnswer::Reaches);
    }

    #[test]
    fn a_budget_one_node_short_of_the_answer_is_undecided() {
        let mut path = PathProbe::over(4, &[(0, 1), (1, 2), (2, 3)]);
        // Reaching 3 from 0 walks four nodes, so a budget of four settles it
        // and a budget of three runs out with the question open.
        assert_eq!(path.reaches(0, 3, 4), ReachAnswer::Reaches);
        assert_eq!(path.reaches(0, 3, 3), ReachAnswer::Undecided);
        // Separate is only ever answered by exhausting the reachable set, so
        // the same one-node-short budget leaves absence unproved.
        let mut split = PathProbe::over(4, &[(0, 1), (2, 3)]);
        assert_eq!(split.reaches(2, 1, 2), ReachAnswer::Separate);
        assert_eq!(split.reaches(2, 1, 1), ReachAnswer::Undecided);
    }

    #[test]
    fn an_exhaustive_walk_counts_every_dependent_except_the_node_itself() {
        // 1 and 2 depend on 0, 3 depends on 1, and 4 depends on nothing.
        let mut probe = PathProbe::over(5, &[(1, 0), (2, 0), (3, 1)]);
        assert_eq!(probe.dependents(0), 3);
        assert_eq!(probe.dependents(1), 1);
        assert_eq!(probe.dependents(3), 0, "a leaf is depended on by nothing");
        assert_eq!(probe.dependents(4), 0);
        // The scratch is shared with the limited probe, so an interleaved run
        // must not read a visit mark the other walk left behind.
        assert_eq!(probe.reaches(3, 0, 4_096), ReachAnswer::Reaches);
        assert_eq!(probe.dependents(0), 3);
    }

    #[test]
    fn every_member_of_a_cycle_depends_on_every_other() {
        let mut probe = PathProbe::over(4, &[(0, 1), (1, 2), (2, 0), (3, 0)]);
        assert_eq!(probe.dependents(0), 3);
        assert_eq!(probe.dependents(3), 0);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "a path probe reads node indexes of its own graph")]
    fn a_node_index_outside_the_graph_is_caught_where_assertions_run() {
        // Where they are off the same call answers Undecided rather than
        // Separate, so a stale index can never manufacture a finding.
        PathProbe::over(2, &[(0, 1)]).reaches(0, 7, 4_096);
    }
}
