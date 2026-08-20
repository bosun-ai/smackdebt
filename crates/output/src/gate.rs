use std::io::{self, Write};

use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
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

/// Streams gate JSON schema version 1, described by `schemas/gate-v1.schema.json`.
///
/// Every value is an integer or a string, like the report schema: each delta
/// row carries both counters' baseline and observed values, so a consumer
/// never has to re-derive which counter moved.
pub fn write_gate_json(
    writer: &mut impl Write,
    baseline: &str,
    comparison: &GateComparison,
) -> io::Result<()> {
    let mut serializer = serde_json::Serializer::new(writer);
    GateView {
        baseline,
        comparison,
    }
    .serialize(&mut serializer)
    .map_err(io::Error::other)
}

struct GateView<'a> {
    baseline: &'a str,
    comparison: &'a GateComparison,
}

impl Serialize for GateView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("schema_version", &1)?;
        map.serialize_entry(
            "status",
            if self.comparison.regressed() {
                "regressed"
            } else {
                "clean"
            },
        )?;
        map.serialize_entry("baseline", self.baseline)?;
        map.serialize_entry("regressions", &Deltas(self.comparison.regressions()))?;
        map.serialize_entry("improvements", &Deltas(self.comparison.improvements()))?;
        map.serialize_entry("totals", &Totals(self.comparison))?;
        map.end()
    }
}

struct Deltas<'a>(&'a [GateDelta]);

impl Serialize for Deltas<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for delta in self.0 {
            seq.serialize_element(&DeltaView(delta))?;
        }
        seq.end()
    }
}

struct DeltaView<'a>(&'a GateDelta);

impl Serialize for DeltaView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("path", self.0.path())?;
        map.serialize_entry("signal", self.0.signal().id())?;
        map.serialize_entry("baseline_high", &self.0.baseline_high())?;
        map.serialize_entry("high", &self.0.high())?;
        map.serialize_entry("baseline_watch", &self.0.baseline_watch())?;
        map.serialize_entry("watch", &self.0.watch())?;
        map.end()
    }
}

struct Totals<'a>(&'a GateComparison);

impl Serialize for Totals<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("regressions", &self.0.regressions().len())?;
        map.serialize_entry("improvements", &self.0.improvements().len())?;
        map.end()
    }
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

    fn rendered_json(baseline: &[GateRow], observed: &[GateRow]) -> String {
        let comparison = GateComparison::between(
            &GateSnapshot::new(baseline.to_vec()),
            &GateSnapshot::new(observed.to_vec()),
        );
        let mut bytes = Vec::new();
        write_gate_json(&mut bytes, ".smackdebt-baseline.tsv", &comparison).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn a_clean_gate_serializes_its_status_and_empty_tables() {
        assert_eq!(
            rendered_json(&[], &[]),
            concat!(
                "{\"schema_version\":1,\"status\":\"clean\",",
                "\"baseline\":\".smackdebt-baseline.tsv\",",
                "\"regressions\":[],\"improvements\":[],",
                "\"totals\":{\"regressions\":0,\"improvements\":0}}",
            )
        );
    }

    #[test]
    fn a_delta_row_serializes_both_counters_baseline_and_observed_values() {
        let baseline = [GateRow::new("src/a.rs", GateSignal::Nesting, 1, 2)];
        let observed = [GateRow::new("src/a.rs", GateSignal::Nesting, 2, 4)];
        assert_eq!(
            rendered_json(&baseline, &observed),
            concat!(
                "{\"schema_version\":1,\"status\":\"regressed\",",
                "\"baseline\":\".smackdebt-baseline.tsv\",",
                "\"regressions\":[{\"path\":\"src/a.rs\",\"signal\":\"nesting\",",
                "\"baseline_high\":1,\"high\":2,\"baseline_watch\":2,\"watch\":4}],",
                "\"improvements\":[],",
                "\"totals\":{\"regressions\":1,\"improvements\":0}}",
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
