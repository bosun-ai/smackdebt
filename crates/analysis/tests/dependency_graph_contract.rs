use smackdebt_analysis::{cycle_witness, strongly_connected_components};

#[test]
fn every_four_node_graph_agrees_with_independent_reachability_and_closed_witnesses() {
    for bits in 0_u32..(1 << 16) {
        let edges = graph_edges(bits);
        let components = strongly_connected_components(4, &edges);
        assert_components(&components, &reachability(&edges), bits);
        for component in components.iter().filter(|part| part.len() > 1) {
            assert_closed_witness(component, &edges);
        }
    }
}

fn graph_edges(bits: u32) -> Vec<(usize, usize)> {
    (0..4)
        .flat_map(|source| {
            (0..4).filter_map(move |target| {
                (bits & (1 << (source * 4 + target)) != 0).then_some((source, target))
            })
        })
        .collect()
}

fn reachability(edges: &[(usize, usize)]) -> [[bool; 4]; 4] {
    let mut reachable = [[false; 4]; 4];
    for (node, row) in reachable.iter_mut().enumerate() {
        row[node] = true;
    }
    for &(source, target) in edges {
        reachable[source][target] = true;
    }
    for via in 0..4 {
        let destinations = reachable[via];
        for row in &mut reachable {
            let reaches_via = row[via];
            for (target, reaches) in row.iter_mut().enumerate() {
                *reaches |= reaches_via && destinations[target];
            }
        }
    }
    reachable
}

fn assert_components(components: &[Vec<usize>], reachable: &[[bool; 4]; 4], bits: u32) {
    for (source, row) in reachable.iter().enumerate() {
        for (target, reaches) in row.iter().enumerate() {
            assert_eq!(
                components
                    .iter()
                    .any(|part| part.contains(&source) && part.contains(&target)),
                *reaches && reachable[target][source],
                "graph {bits}"
            );
        }
    }
}

fn assert_closed_witness(component: &[usize], edges: &[(usize, usize)]) {
    let witness = cycle_witness(component, edges).expect("multi-file component has a witness");
    assert_eq!(witness.first().unwrap().0, witness.last().unwrap().1);
    for &(source, target) in &witness {
        assert!(edges.contains(&(source, target)));
        assert!(component.contains(&source) && component.contains(&target));
    }
    for consecutive in witness.windows(2) {
        assert_eq!(consecutive[0].1, consecutive[1].0);
    }
}
