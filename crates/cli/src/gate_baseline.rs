use smackdebt_project::{GateRow, GateSignal, GateSnapshot};

/// The committed baseline the gate compares against by default.
pub(crate) const BASELINE_FILE_NAME: &str = ".smackdebt-baseline.tsv";

const VERSION_LINE: &str = "# smackdebt gate baseline v1";
const COLUMN_LINE: &str = "path\tsignal\thigh\twatch";

/// Renders one snapshot as committed baseline bytes.
///
/// Rows are already sorted by path then signal; zero-zero rows are omitted so
/// vanished debt also vanishes from the file, and the rendering round-trips
/// byte-identically through [`parse`].
pub(crate) fn render(snapshot: &GateSnapshot) -> String {
    let mut text = format!("{VERSION_LINE}\n{COLUMN_LINE}\n");
    for row in snapshot.rows() {
        if row.high() == 0 && row.watch() == 0 {
            continue;
        }
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            row.path(),
            row.signal().id(),
            row.high(),
            row.watch()
        ));
    }
    text
}

/// Reads baseline bytes strictly, so a malformed baseline never passes.
pub(crate) fn parse(text: &str) -> Result<GateSnapshot, String> {
    if text.contains('\r') {
        return Err("the baseline must use LF line endings".to_owned());
    }
    let mut lines = text.lines();
    if lines.next() != Some(VERSION_LINE) {
        return Err(format!("baseline line 1: expected '{VERSION_LINE}'"));
    }
    if lines.next() != Some(COLUMN_LINE) {
        return Err("baseline line 2: expected the columns path, signal, high, watch".to_owned());
    }
    let mut rows: Vec<GateRow> = Vec::new();
    for (index, line) in lines.enumerate() {
        let number = index + 3;
        let fields: Vec<&str> = line.split('\t').collect();
        let [path, signal, high, watch] = fields[..] else {
            return Err(format!(
                "baseline line {number}: expected 4 tab-separated values"
            ));
        };
        if path.is_empty() {
            return Err(format!("baseline line {number}: empty path"));
        }
        let signal = GateSignal::from_id(signal)
            .ok_or_else(|| format!("baseline line {number}: unknown signal '{signal}'"))?;
        let high = count(high, number)?;
        let watch = count(watch, number)?;
        if let Some(previous) = rows.last() {
            match (previous.path(), previous.signal()).cmp(&(path, signal)) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => {
                    return Err(format!(
                        "baseline line {number}: duplicate row for {path} · {}",
                        signal.id()
                    ));
                }
                std::cmp::Ordering::Greater => {
                    return Err(format!(
                        "baseline line {number}: rows are not sorted by path then signal"
                    ));
                }
            }
        }
        rows.push(GateRow::new(path, signal, high, watch));
    }
    Ok(GateSnapshot::new(rows))
}

fn count(value: &str, number: usize) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("baseline line {number}: '{value}' is not a whole count"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline(rows: &str) -> String {
        format!("{VERSION_LINE}\n{COLUMN_LINE}\n{rows}")
    }

    #[test]
    fn a_written_baseline_round_trips_byte_identically() {
        let snapshot = GateSnapshot::new(vec![
            GateRow::new("crates/a/src/lib.rs", GateSignal::Cognitive, 3, 5),
            GateRow::new("crates/a/src/lib.rs", GateSignal::Nesting, 0, 1),
            GateRow::new("crates/b/src/lib.rs", GateSignal::PackageCycle, 1, 0),
        ]);
        let text = render(&snapshot);
        assert_eq!(
            text,
            baseline(
                "crates/a/src/lib.rs\tcognitive\t3\t5\n\
                 crates/a/src/lib.rs\tnesting\t0\t1\n\
                 crates/b/src/lib.rs\tpackage_cycle\t1\t0\n"
            )
        );
        assert_eq!(parse(&text).unwrap(), snapshot);
        assert_eq!(render(&parse(&text).unwrap()), text);
    }

    #[test]
    fn a_zero_zero_row_is_never_written() {
        let snapshot = GateSnapshot::new(vec![GateRow::new(
            "crates/a/src/lib.rs",
            GateSignal::Cognitive,
            0,
            0,
        )]);
        assert_eq!(render(&snapshot), baseline(""));
    }

    #[test]
    fn an_empty_baseline_is_just_its_headers() {
        assert_eq!(parse(&baseline("")).unwrap(), GateSnapshot::default());
    }

    #[test]
    fn an_unknown_signal_is_rejected() {
        let error = parse(&baseline("src/a.rs\tcoupling\t1\t0\n")).unwrap_err();
        assert_eq!(error, "baseline line 3: unknown signal 'coupling'");
    }

    #[test]
    fn a_duplicate_key_is_rejected() {
        let error = parse(&baseline(
            "src/a.rs\tcognitive\t1\t0\nsrc/a.rs\tcognitive\t2\t0\n",
        ))
        .unwrap_err();
        assert_eq!(
            error,
            "baseline line 4: duplicate row for src/a.rs · cognitive"
        );
    }

    #[test]
    fn out_of_order_rows_are_rejected() {
        let error = parse(&baseline(
            "src/b.rs\tcognitive\t1\t0\nsrc/a.rs\tcognitive\t1\t0\n",
        ))
        .unwrap_err();
        assert_eq!(
            error,
            "baseline line 4: rows are not sorted by path then signal"
        );
        let error = parse(&baseline(
            "src/a.rs\tnesting\t1\t0\nsrc/a.rs\tcognitive\t1\t0\n",
        ))
        .unwrap_err();
        assert_eq!(
            error,
            "baseline line 4: rows are not sorted by path then signal"
        );
    }

    #[test]
    fn wrong_headers_and_malformed_rows_are_rejected() {
        assert_eq!(
            parse("nonsense\n").unwrap_err(),
            "baseline line 1: expected '# smackdebt gate baseline v1'"
        );
        assert_eq!(
            parse(&format!("{VERSION_LINE}\npath,signal\n")).unwrap_err(),
            "baseline line 2: expected the columns path, signal, high, watch"
        );
        assert_eq!(
            parse(&baseline("src/a.rs\tcognitive\t1\n")).unwrap_err(),
            "baseline line 3: expected 4 tab-separated values"
        );
        assert_eq!(
            parse(&baseline("src/a.rs\tcognitive\tmany\t0\n")).unwrap_err(),
            "baseline line 3: 'many' is not a whole count"
        );
        assert_eq!(
            parse(&baseline("src/a.rs\tcognitive\t1\t0 \n")).unwrap_err(),
            "baseline line 3: '0 ' is not a whole count"
        );
        assert!(parse(&baseline("src/a.rs\tcognitive\t1\t0\r\n")).is_err());
    }
}
