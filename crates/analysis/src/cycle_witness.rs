//! Stable directed witnesses for strongly connected dependency components.

use std::collections::VecDeque;

pub fn cycle_witness(component: &[usize], edges: &[(usize, usize)]) -> Option<Vec<(usize, usize)>> {
    component.first()?;
    let mut adjacency = vec![Vec::new(); component.last().copied().unwrap_or(0) + 1];
    for &(source, target) in edges {
        if component.binary_search(&source).is_ok() && component.binary_search(&target).is_ok() {
            adjacency[source].push(target);
        }
    }
    for neighbors in &mut adjacency {
        neighbors.sort_unstable();
        neighbors.dedup();
    }
    let mut best = None;
    for &start in component {
        let candidate = if adjacency[start].binary_search(&start).is_ok() {
            Some(vec![(start, start)])
        } else {
            let mut queue = VecDeque::from([start]);
            let mut previous = vec![None; adjacency.len()];
            previous[start] = Some(start);
            let mut found = None;
            while let Some(node) = queue.pop_front() {
                if node != start && adjacency[node].binary_search(&start).is_ok() {
                    let mut nodes = vec![node];
                    let mut current = node;
                    while current != start {
                        current = previous[current].expect("visited node has a predecessor");
                        nodes.push(current);
                    }
                    nodes.reverse();
                    nodes.push(start);
                    found = Some(nodes.windows(2).map(|pair| (pair[0], pair[1])).collect());
                    break;
                }
                for &target in &adjacency[node] {
                    if target != start && previous[target].is_none() {
                        previous[target] = Some(node);
                        queue.push_back(target);
                    }
                }
            }
            found
        };
        if let Some(candidate) = candidate
            && best.as_ref().is_none_or(|current: &Vec<(usize, usize)>| {
                (candidate.len(), &candidate) < (current.len(), current)
            })
        {
            best = Some(candidate);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chooses_short_stable_cycle() {
        assert_eq!(
            cycle_witness(&[0, 1, 2], &[(0, 2), (2, 0), (0, 1), (1, 2)]),
            Some(vec![(0, 2), (2, 0)])
        );
    }

    #[test]
    fn shortest_cycle_can_start_after_the_first_component_node() {
        assert_eq!(
            cycle_witness(&[0, 1, 2], &[(0, 1), (1, 2), (2, 0), (1, 1)]),
            Some(vec![(1, 1)])
        );
    }
}
