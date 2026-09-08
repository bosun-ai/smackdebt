use std::fs;
use std::path::Path;

pub(crate) fn write(root: &Path, path: &str, source: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

pub(crate) fn pairs(report: &smackdebt_analysis::Report) -> Vec<(&str, &str)> {
    report
        .dependency_edges()
        .iter()
        .map(|edge| {
            (
                report.files()[edge.source().index()].path(),
                report.files()[edge.target().index()].path(),
            )
        })
        .collect()
}
