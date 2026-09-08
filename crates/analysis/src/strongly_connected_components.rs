//! Strongly connected components of an indexed directed dependency graph.

pub fn strongly_connected_components(
    node_count: usize,
    edges: &[(usize, usize)],
) -> Vec<Vec<usize>> {
    let mut outgoing = vec![Vec::new(); node_count];
    let mut incoming = vec![Vec::new(); node_count];
    for &(source, target) in edges {
        if source < node_count && target < node_count {
            outgoing[source].push(target);
            incoming[target].push(source);
        }
    }
    for neighbors in outgoing.iter_mut().chain(&mut incoming) {
        neighbors.sort_unstable();
        neighbors.dedup();
    }

    // Kosaraju with explicit stacks keeps both passes linear without using the
    // process stack for long dependency chains.
    let mut seen = vec![false; node_count];
    let mut order = Vec::with_capacity(node_count);
    for start in 0..node_count {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![(start, 0usize)];
        while let Some((node, next)) = stack.last_mut() {
            if *next < outgoing[*node].len() {
                let target = outgoing[*node][*next];
                *next += 1;
                if !seen[target] {
                    seen[target] = true;
                    stack.push((target, 0));
                }
            } else {
                order.push(*node);
                stack.pop();
            }
        }
    }

    seen.fill(false);
    let mut components = Vec::new();
    for &start in order.iter().rev() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            component.push(node);
            for &target in incoming[node].iter().rev() {
                if !seen[target] {
                    seen[target] = true;
                    stack.push(target);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components.sort();
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_components_in_stable_order() {
        assert_eq!(
            strongly_connected_components(5, &[(1, 2), (2, 1), (3, 4)]),
            vec![vec![0], vec![1, 2], vec![3], vec![4]]
        );
    }

    #[test]
    fn a_very_deep_chain_does_not_use_the_process_stack() {
        let nodes = 200_000;
        let edges: Vec<_> = (0..nodes - 1).map(|node| (node, node + 1)).collect();
        let components = strongly_connected_components(nodes, &edges);
        assert_eq!(components.len(), nodes);
        assert_eq!(components[0], vec![0]);
        assert_eq!(components[nodes - 1], vec![nodes - 1]);
    }
}
