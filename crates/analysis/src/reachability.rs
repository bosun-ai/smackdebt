use crate::strongly_connected_components;
use std::collections::VecDeque;

/// What a bounded path probe settled about one ordered pair of nodes.
///
/// `Undecided` is a first-class answer rather than a failure: a probe that
/// spent its whole budget without settling the question proves nothing, and a
/// finding that claims two files have no dependency between them must never
/// rest on a search that ran out of room.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReachAnswer {
    Reaches,
    Separate,
    Undecided,
}

/// How many nodes transitively depend on each node, counting itself.
///
/// An edge `(source, target)` reads "source depends on target", so a change to
/// a node travels to the nodes that depend on it and the count answers how far
/// that change can reach. Cyclic nodes all carry the same count, because every
/// member of a cycle reaches every other member.
///
/// The closure runs over the condensation of the graph in topological order,
/// with one fixed-width bit set per live component, so its transient memory is
/// the number of components times the width of the live cut. **The caller
/// bounds the node count** — the accepted closure node limit is a caller's
/// rule, not this function's — and every walk here uses its own stack, so an
/// adversarially long chain completes rather than overflowing the process
/// stack.
pub fn reach_in_counts(node_count: usize, edges: &[(usize, usize)]) -> Vec<u32> {
    let components = strongly_connected_components(node_count, edges);
    let condensation = Condensation::of(node_count, edges, &components);
    let reached = condensation.reached_nodes();
    (0..node_count)
        .map(|node| reached[condensation.component_of[node]])
        .collect()
}

/// The number of nodes in the largest strongly connected component.
pub fn largest_component_size(components: &[Vec<usize>]) -> u32 {
    components
        .iter()
        .map(|component| component.len() as u32)
        .max()
        .unwrap_or(0)
}

/// Whether a path leads from `from` to `to`, within a node budget.
///
/// The walk starts at `to` and follows incoming edges breadth first, so it
/// explores only what can reach the target. It visits at most `budget` nodes
/// and answers `Separate` only after exhausting the whole set that reaches
/// `to`, so absence is proved rather than inferred: a walk that runs out of
/// budget answers `Undecided` instead. Neighbours are visited in node order, so
/// a tight budget settles the same question the same way on every run.
pub fn bounded_reaches(
    node_count: usize,
    edges: &[(usize, usize)],
    from: usize,
    to: usize,
    budget: usize,
) -> ReachAnswer {
    if from >= node_count || to >= node_count {
        return ReachAnswer::Separate;
    }
    if from == to {
        return ReachAnswer::Reaches;
    }
    walk_back(&incoming_nodes(node_count, edges), from, to, budget)
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

/// Walks back from `to` until it meets `from`, exhausts what reaches `to`, or
/// spends its node budget.
fn walk_back(incoming: &[Vec<usize>], from: usize, to: usize, budget: usize) -> ReachAnswer {
    if budget == 0 {
        return ReachAnswer::Undecided;
    }
    let mut visited = vec![false; incoming.len()];
    visited[to] = true;
    let mut seen = 1;
    let mut pending = VecDeque::from([to]);
    while let Some(node) = pending.pop_front() {
        for &source in &incoming[node] {
            if visited[source] {
                continue;
            }
            if seen == budget {
                return ReachAnswer::Undecided;
            }
            visited[source] = true;
            seen += 1;
            if source == from {
                return ReachAnswer::Reaches;
            }
            pending.push_back(source);
        }
    }
    ReachAnswer::Separate
}

/// The acyclic graph of a graph's strongly connected components.
///
/// Condensing first is what makes a reach count answerable at all: every member
/// of a cycle reaches every other member, so the question is only ever asked of
/// components.
struct Condensation {
    component_of: Vec<usize>,
    sizes: Vec<u32>,
    dependents: Vec<Vec<usize>>,
    dependencies: Vec<Vec<usize>>,
}

impl Condensation {
    fn of(node_count: usize, edges: &[(usize, usize)], components: &[Vec<usize>]) -> Self {
        let mut component_of = vec![0; node_count];
        for (index, component) in components.iter().enumerate() {
            for &node in component {
                component_of[node] = index;
            }
        }
        let dependents = dependent_components(edges, &component_of, components.len());
        let dependencies = reversed(&dependents);
        Self {
            component_of,
            sizes: components.iter().map(|c| c.len() as u32).collect(),
            dependents,
            dependencies,
        }
    }

    /// How many nodes reach each component, counting the component's own.
    fn reached_nodes(&self) -> Vec<u32> {
        let mut counts = vec![0; self.sizes.len()];
        let mut sets = vec![Reached::default(); self.sizes.len()];
        let mut consumers: Vec<usize> = self.dependencies.iter().map(Vec::len).collect();
        for component in self.order() {
            let mut reached = self.inherited(component, &mut sets, &mut consumers);
            reached.widen(self.width());
            reached.insert(component, self.sizes[component]);
            counts[component] = reached.nodes;
            sets[component] = reached;
        }
        counts
    }

    /// The union of the sets of every component that depends on this one.
    ///
    /// The last reader of a set takes the buffer instead of copying it, so a
    /// long chain moves one buffer along rather than copying it per step.
    fn inherited(
        &self,
        component: usize,
        sets: &mut [Reached],
        consumers: &mut [usize],
    ) -> Reached {
        let mut reached = Reached::default();
        for &dependent in &self.dependents[component] {
            consumers[dependent] -= 1;
            let last = consumers[dependent] == 0;
            if reached.is_vacant() && last {
                reached = std::mem::take(&mut sets[dependent]);
            } else {
                reached.widen(self.width());
                reached.absorb(&sets[dependent], &self.sizes);
            }
            if last {
                sets[dependent] = Reached::default();
            }
        }
        reached
    }

    /// The components ordered so that every dependent precedes the dependency
    /// it reaches, walked with its own queue rather than the process stack.
    fn order(&self) -> Vec<usize> {
        let mut pending: Vec<usize> = self.dependents.iter().map(Vec::len).collect();
        let mut ready: VecDeque<usize> = (0..pending.len())
            .filter(|&component| pending[component] == 0)
            .collect();
        let mut order = Vec::with_capacity(pending.len());
        while let Some(component) = ready.pop_front() {
            order.push(component);
            for &dependency in &self.dependencies[component] {
                pending[dependency] -= 1;
                if pending[dependency] == 0 {
                    ready.push_back(dependency);
                }
            }
        }
        order
    }

    /// The number of words one component set occupies.
    fn width(&self) -> usize {
        self.sizes.len().div_ceil(64)
    }
}

/// The components that depend on each component, in component order.
fn dependent_components(
    edges: &[(usize, usize)],
    component_of: &[usize],
    count: usize,
) -> Vec<Vec<usize>> {
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); count];
    let inside = edges
        .iter()
        .filter(|&&(source, target)| source < component_of.len() && target < component_of.len());
    for &(source, target) in inside {
        let (dependent, dependency) = (component_of[source], component_of[target]);
        if dependent != dependency {
            dependents[dependency].push(dependent);
        }
    }
    for list in dependents.iter_mut() {
        list.sort_unstable();
        list.dedup();
    }
    dependents
}

/// The same relation read the other way around.
fn reversed(dependents: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut dependencies: Vec<Vec<usize>> = vec![Vec::new(); dependents.len()];
    for (dependency, list) in dependents.iter().enumerate() {
        for &dependent in list {
            dependencies[dependent].push(dependency);
        }
    }
    dependencies
}

/// One component's set of reaching components and the nodes they hold.
///
/// The set is a fixed-width bit set over components, and the node count is
/// carried alongside it so a member is weighed by its component's size exactly
/// once, when the bit is first set.
#[derive(Clone, Debug, Default)]
struct Reached {
    members: Vec<u64>,
    nodes: u32,
}

impl Reached {
    fn is_vacant(&self) -> bool {
        self.members.is_empty()
    }

    fn widen(&mut self, width: usize) {
        if self.is_vacant() {
            self.members = vec![0; width];
        }
    }

    fn insert(&mut self, component: usize, size: u32) {
        let (word, bit) = (component / 64, 1 << (component % 64));
        if self.members[word] & bit == 0 {
            self.members[word] |= bit;
            self.nodes += size;
        }
    }

    fn absorb(&mut self, other: &Self, sizes: &[u32]) {
        let mut added = 0;
        for (word, (current, extra)) in self.members.iter_mut().zip(&other.members).enumerate() {
            let mut fresh = *extra & !*current;
            *current |= fresh;
            while fresh != 0 {
                added += sizes[word * 64 + fresh.trailing_zeros() as usize];
                fresh &= fresh - 1;
            }
        }
        self.nodes += added;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strongly_connected_components;

    #[test]
    fn a_diamond_counts_every_transitive_dependent_once() {
        // 3 depends on 1 and 2, which both depend on 0.
        let counts = reach_in_counts(4, &[(1, 0), (2, 0), (3, 1), (3, 2)]);
        assert_eq!(counts, vec![4, 2, 2, 1]);
    }

    #[test]
    fn every_member_of_a_cycle_reaches_every_member() {
        // 0, 1, and 2 form a cycle that 3 depends on.
        let counts = reach_in_counts(4, &[(0, 1), (1, 2), (2, 0), (3, 0)]);
        assert_eq!(counts, vec![4, 4, 4, 1]);
    }

    #[test]
    fn an_edge_outside_the_graph_and_a_self_edge_change_nothing() {
        let counts = reach_in_counts(2, &[(0, 1), (0, 0), (2, 1), (1, 9)]);
        assert_eq!(counts, vec![1, 2]);
        assert_eq!(reach_in_counts(0, &[]), Vec::<u32>::new());
    }

    #[test]
    fn a_two_hundred_thousand_node_chain_is_reduced_and_closed() {
        let nodes = 200_000;
        let edges: Vec<_> = (0..nodes - 1).map(|node| (node, node + 1)).collect();
        let counts = reach_in_counts(nodes, &edges);
        assert_eq!(counts[0], 1);
        assert_eq!(counts[nodes / 2], (nodes / 2) as u32 + 1);
        assert_eq!(counts[nodes - 1], nodes as u32);
    }

    #[test]
    fn the_largest_component_is_the_core() {
        let components = strongly_connected_components(5, &[(0, 1), (1, 0), (2, 3), (3, 2)]);
        assert_eq!(largest_component_size(&components), 2);
        assert_eq!(largest_component_size(&[]), 0);
    }

    #[test]
    fn nodes_with_no_path_between_them_are_separate() {
        assert_eq!(
            bounded_reaches(4, &[(0, 1), (2, 3)], 0, 3, 4_096),
            ReachAnswer::Separate
        );
    }

    #[test]
    fn a_three_hop_path_reaches() {
        let edges = [(0, 1), (1, 2), (2, 3)];
        assert_eq!(
            bounded_reaches(4, &edges, 0, 3, 4_096),
            ReachAnswer::Reaches
        );
        // The walk follows the edge direction, so the far end reaches nothing.
        assert_eq!(
            bounded_reaches(4, &edges, 3, 0, 4_096),
            ReachAnswer::Separate
        );
        assert_eq!(bounded_reaches(4, &edges, 2, 2, 0), ReachAnswer::Reaches);
    }

    #[test]
    fn a_budget_one_node_short_of_the_answer_is_undecided() {
        let edges = [(0, 1), (1, 2), (2, 3)];
        // Reaching 3 from 0 walks four nodes, so a budget of four settles it
        // and a budget of three runs out with the question open.
        assert_eq!(bounded_reaches(4, &edges, 0, 3, 4), ReachAnswer::Reaches);
        assert_eq!(bounded_reaches(4, &edges, 0, 3, 3), ReachAnswer::Undecided);
        // Separate is only ever answered by exhausting the reachable set, so
        // the same one-node-short budget leaves absence unproved.
        assert_eq!(
            bounded_reaches(4, &[(0, 1), (2, 3)], 2, 1, 2),
            ReachAnswer::Separate
        );
        assert_eq!(
            bounded_reaches(4, &[(0, 1), (2, 3)], 2, 1, 1),
            ReachAnswer::Undecided
        );
    }
}
