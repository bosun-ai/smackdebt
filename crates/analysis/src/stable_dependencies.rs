//! Stable Dependencies Principle violations between packages.
//!
//! A package should depend toward lower instability. An edge with at least two
//! references produces a Watch finding when its target has strictly greater
//! instability than its source. Equal ratios and isolated packages produce none.
//! The comparison uses exact cross-products of fan-in and fan-out. Findings keep
//! both packages' measurements and witness edges for explanation. The supplied
//! package graph owns source eligibility; this module performs no graph rebuild.

#![deny(missing_docs)]

use crate::architecture::{PackageEdge, PackageGraphMeasurement};
use crate::{DependencyEdgeId, PackageId, Rating};

/// The references one package needs into another before the direction matters.
pub const MINIMUM_STABLE_DEPENDENCY_REFERENCES: u32 = 2;

/// Finds packages that depend on something less stable than themselves.
///
/// Stability is compared by cross-multiplying the integer degree operands, so
/// no instability ratio is ever turned into a floating-point value. A
/// dependency violates the direction when the depended-on package is the more
/// unstable of the two:
/// `out_target * total_source > out_source * total_target`. Equal cross
/// products are not a violation, and a package with no neighbors never
/// participates.
pub fn stable_dependency_findings(
    edges: &[PackageEdge],
    graph: &[PackageGraphMeasurement],
) -> Vec<StableDependencyFinding> {
    let mut findings = Vec::new();
    for edge in edges {
        if edge.references() < MINIMUM_STABLE_DEPENDENCY_REFERENCES {
            continue;
        }
        let (Some(source), Some(target)) = (
            graph.get(edge.source().index()),
            graph.get(edge.target().index()),
        ) else {
            continue;
        };
        let source_total = u64::from(source.fan_in()) + u64::from(source.fan_out());
        let target_total = u64::from(target.fan_in()) + u64::from(target.fan_out());
        if source_total == 0 || target_total == 0 {
            continue;
        }
        if u64::from(target.fan_out()) * source_total <= u64::from(source.fan_out()) * target_total
        {
            continue;
        }
        findings.push(StableDependencyFinding::new(
            StableDependencyFindingId::from_index(findings.len()),
            edge.source(),
            edge.target(),
            StableDependencyEvidence::new(*source, *target, edge.references()),
            edge.file_edges().to_vec(),
        ));
    }
    findings
}

/// One package that depends on a less stable package.
///
/// Stable-dependency violations own their table and their own index type, so a
/// row can never be mistaken for a position in the cycle finding table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StableDependencyFinding {
    id: StableDependencyFindingId,
    rating: Rating,
    source: PackageId,
    target: PackageId,
    evidence: StableDependencyEvidence,
    witness_edges: Vec<DependencyEdgeId>,
}

impl StableDependencyFinding {
    /// Records a selected stable-dependency violation as Watch with its witnesses.
    pub fn new(
        id: StableDependencyFindingId,
        source: PackageId,
        target: PackageId,
        evidence: StableDependencyEvidence,
        witness_edges: Vec<DependencyEdgeId>,
    ) -> Self {
        Self {
            id,
            rating: Rating::Watch,
            source,
            target,
            evidence,
            witness_edges,
        }
    }
    /// The row's typed position in its owning report table.
    pub const fn id(&self) -> StableDependencyFindingId {
        self.id
    }
    /// The health rating carried by this observation.
    pub const fn rating(&self) -> Rating {
        self.rating
    }
    /// The depending package.
    pub const fn source(&self) -> PackageId {
        self.source
    }
    /// The depended-on package.
    pub const fn target(&self) -> PackageId {
        self.target
    }
    /// Both packages' degree operands and the references supporting the violation.
    pub const fn evidence(&self) -> StableDependencyEvidence {
        self.evidence
    }
    /// The file-edge identities that explain this finding.
    pub fn witness_edges(&self) -> &[DependencyEdgeId] {
        &self.witness_edges
    }
}

/// The exact degree operands behind one stable-dependency violation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StableDependencyEvidence {
    source: PackageGraphMeasurement,
    target: PackageGraphMeasurement,
    references: u32,
}

impl StableDependencyEvidence {
    /// Retains both packages' degree measurements and the reference count between them.
    pub const fn new(
        source: PackageGraphMeasurement,
        target: PackageGraphMeasurement,
        references: u32,
    ) -> Self {
        Self {
            source,
            target,
            references,
        }
    }
    /// The depending package's degree facts.
    pub const fn source(self) -> PackageGraphMeasurement {
        self.source
    }
    /// The depended-on package's degree facts.
    pub const fn target(self) -> PackageGraphMeasurement {
        self.target
    }
    /// References the depending package has into the depended-on package.
    pub const fn references(self) -> u32 {
        self.references
    }
}

crate::table_index::table_index!(
    /// The position of one stable dependency finding in its report table.
    StableDependencyFindingId
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DependencyEdgeId, PackageEdgeId, PackageId, Rating};

    fn measurement(index: usize, fan_in: u32, fan_out: u32) -> PackageGraphMeasurement {
        PackageGraphMeasurement::new(PackageId::from_index(index), fan_in, fan_out)
    }

    fn edge(source: usize, target: usize, references: u32) -> PackageEdge {
        PackageEdge::new(
            PackageEdgeId::from_index(0),
            PackageId::from_index(source),
            PackageId::from_index(target),
            1,
            references,
            vec![DependencyEdgeId::from_index(0)],
        )
    }

    #[test]
    fn a_stable_package_depending_on_an_unstable_one_is_a_watch_finding() {
        // Source instability 1/4, target instability 2/3.
        let graph = [measurement(0, 3, 1), measurement(1, 1, 2)];
        let findings = stable_dependency_findings(&[edge(0, 1, 3)], &graph);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id(), StableDependencyFindingId::from_index(0));
        assert_eq!(findings[0].rating(), Rating::Watch);
        assert_eq!(
            (findings[0].source(), findings[0].target()),
            (PackageId::from_index(0), PackageId::from_index(1))
        );
        let evidence = findings[0].evidence();
        assert_eq!(
            (evidence.source().fan_in(), evidence.source().fan_out()),
            (3, 1)
        );
        assert_eq!(
            (evidence.target().fan_in(), evidence.target().fan_out()),
            (1, 2)
        );
        assert_eq!(evidence.references(), 3);
    }

    #[test]
    fn one_incidental_reference_is_never_a_finding() {
        let graph = [measurement(0, 3, 1), measurement(1, 1, 2)];
        assert_eq!(MINIMUM_STABLE_DEPENDENCY_REFERENCES, 2);
        assert!(stable_dependency_findings(&[edge(0, 1, 1)], &graph).is_empty());
        assert_eq!(
            stable_dependency_findings(&[edge(0, 1, 2)], &graph).len(),
            1
        );
    }

    #[test]
    fn equal_integer_cross_products_are_not_a_violation() {
        // Both packages have instability 1/2.
        let graph = [measurement(0, 1, 1), measurement(1, 2, 2)];
        assert!(stable_dependency_findings(&[edge(0, 1, 9)], &graph).is_empty());
    }

    #[test]
    fn depending_on_a_more_stable_package_is_never_a_violation() {
        let graph = [measurement(0, 1, 2), measurement(1, 3, 1)];
        assert!(stable_dependency_findings(&[edge(0, 1, 9)], &graph).is_empty());
    }

    #[test]
    fn a_package_without_neighbors_does_not_participate() {
        let graph = [measurement(0, 0, 1), measurement(1, 0, 0)];
        assert!(stable_dependency_findings(&[edge(0, 1, 4)], &graph).is_empty());
    }
}
