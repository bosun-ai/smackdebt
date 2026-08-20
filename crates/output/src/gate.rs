use std::io::{self, Write};

use smackdebt_analysis::{GateComparison, GateDelta};

use crate::output::Counted;

/// Writes one gate report in the house vocabulary.
///
/// Regressions come first as `worse` rows, improvements follow as `better`
/// rows, and each row names the path, the signal, and every counter that
/// moved as `<before> → <after>`. The proposal line appears only when a
/// regression exists, because a clean or improving gate needs no update.
pub fn write_gate(
    writer: &mut impl Write,
    baseline: &str,
    comparison: &GateComparison,
) -> io::Result<()> {
    writeln!(writer, "GATE  {baseline}")?;
    writeln!(writer)?;
    if !comparison.regressions().is_empty() || !comparison.improvements().is_empty() {
        for delta in comparison.regressions() {
            writeln!(writer, "  {:<6} {}", "worse", delta_facts(delta))?;
        }
        for delta in comparison.improvements() {
            writeln!(writer, "  {:<6} {}", "better", delta_facts(delta))?;
        }
        writeln!(writer)?;
    }
    writeln!(
        writer,
        "{} · {}",
        Counted::new(comparison.regressions().len(), "regression", "regressions"),
        Counted::new(
            comparison.improvements().len(),
            "improvement",
            "improvements"
        ),
    )?;
    if comparison.regressed() {
        writeln!(writer, "next: smackdebt gate --update")?;
    }
    Ok(())
}

fn delta_facts(delta: &GateDelta) -> String {
    let mut facts = vec![delta.path().to_owned(), delta.signal().id().to_owned()];
    if delta.high() != delta.baseline_high() {
        facts.push(format!("high {} → {}", delta.baseline_high(), delta.high()));
    }
    if delta.watch() != delta.baseline_watch() {
        facts.push(format!(
            "watch {} → {}",
            delta.baseline_watch(),
            delta.watch()
        ));
    }
    facts.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use smackdebt_analysis::{GateRow, GateSignal, GateSnapshot};

    fn rendered(baseline: &[GateRow], observed: &[GateRow]) -> String {
        let comparison = GateComparison::between(
            &GateSnapshot::new(baseline.to_vec()),
            &GateSnapshot::new(observed.to_vec()),
        );
        let mut bytes = Vec::new();
        write_gate(&mut bytes, ".smackdebt-baseline.tsv", &comparison).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn a_clean_gate_states_its_zero_totals_without_a_proposal() {
        assert_eq!(
            rendered(&[], &[]),
            "GATE  .smackdebt-baseline.tsv\n\n0 regressions · 0 improvements\n"
        );
    }

    #[test]
    fn a_regression_and_an_improvement_share_one_report_with_the_proposal() {
        let baseline = [
            GateRow::new(
                "crates/analysis/src/verdict.rs",
                GateSignal::Cognitive,
                2,
                0,
            ),
            GateRow::new("crates/output/src/output.rs", GateSignal::Cognitive, 3, 5),
        ];
        let observed = [
            GateRow::new(
                "crates/analysis/src/verdict.rs",
                GateSignal::Cognitive,
                1,
                0,
            ),
            GateRow::new("crates/output/src/output.rs", GateSignal::Cognitive, 4, 5),
        ];
        assert_eq!(
            rendered(&baseline, &observed),
            concat!(
                "GATE  .smackdebt-baseline.tsv\n",
                "\n",
                "  worse  crates/output/src/output.rs · cognitive · high 3 → 4\n",
                "  better crates/analysis/src/verdict.rs · cognitive · high 2 → 1\n",
                "\n",
                "1 regression · 1 improvement\n",
                "next: smackdebt gate --update\n",
            )
        );
    }

    #[test]
    fn a_row_names_every_counter_that_moved_and_only_those() {
        let baseline = [GateRow::new("src/a.rs", GateSignal::Nesting, 1, 2)];
        let observed = [GateRow::new("src/a.rs", GateSignal::Nesting, 2, 4)];
        let report = rendered(&baseline, &observed);
        assert!(
            report.contains("  worse  src/a.rs · nesting · high 1 → 2 · watch 2 → 4\n"),
            "{report}"
        );
    }
}
