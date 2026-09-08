//! Direct dependency degree: fan-in and fan-out for each graph node.
//!
//! Fan-in counts distinct incoming neighbours; fan-out counts distinct outgoing
//! neighbours. These are the incoming and outgoing coupling operands used by
//! instability. Graph nodes represent files or packages according to the caller.
//! Self edges, duplicate neighbours, and edges outside the node table do not
//! increase degree. Graph eligibility is decided before this algorithm runs.

#![deny(missing_docs)]

/// Returns `(fan_in, fan_out)` for every node in index order, including isolated nodes.
///
/// Edges are `(depending, depended_on)`. Duplicate, self, and out-of-range edges
/// do not increase either count.
pub fn dependency_degree(node_count: usize, edges: &[(usize, usize)]) -> Vec<(u32, u32)> {
    let mut incoming = vec![Vec::new(); node_count];
    let mut outgoing = vec![Vec::new(); node_count];
    for &(source, target) in edges {
        if source < node_count && target < node_count && source != target {
            outgoing[source].push(target);
            incoming[target].push(source);
        }
    }
    (0..node_count)
        .map(|node| {
            outgoing[node].sort_unstable();
            outgoing[node].dedup();
            incoming[node].sort_unstable();
            incoming[node].dedup();
            (incoming[node].len() as u32, outgoing[node].len() as u32)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_unique_neighbors() {
        assert_eq!(
            dependency_degree(3, &[(0, 1), (0, 1), (2, 1)]),
            vec![(0, 1), (2, 0), (0, 1)]
        );
    }
}
