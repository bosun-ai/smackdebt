use super::*;
fn config(arguments: serde_json::Value) -> CIncludeConfig {
    CIncludeConfig::parse(
        Path::new("/repo"),
        &serde_json::json!([{"directory":"/repo", "file":"src/main.cpp", "arguments": arguments}])
            .to_string(),
    )
    .unwrap()
}
fn lookup(
    configs: &[CIncludeConfig],
    style: IncludeStyle,
    files: &[&str],
) -> Result<Option<PathBuf>, String> {
    resolve(
        configs,
        Path::new("src/main.cpp"),
        "widget.h",
        style,
        |path| files.iter().any(|file| path == Path::new(file)),
    )
}
#[test]
fn quoted_includes_choose_source_directory_before_compiler_paths() {
    let config = config(serde_json::json!(["c++", "-iquote", "quotes", "-Iinclude"]));
    assert_eq!(
        lookup(
            &[config],
            IncludeStyle::Quoted,
            &["src/widget.h", "quotes/widget.h", "include/widget.h"]
        )
        .unwrap(),
        Some(PathBuf::from("src/widget.h"))
    );
}
#[test]
fn angle_includes_skip_source_and_quote_directories() {
    let config = config(serde_json::json!([
        "c++",
        "-iquotequotes",
        "-I",
        "first",
        "-Isecond",
        "-isystem",
        "system"
    ]));
    assert_eq!(
        lookup(
            &[config],
            IncludeStyle::Angle,
            &[
                "src/widget.h",
                "quotes/widget.h",
                "first/widget.h",
                "second/widget.h"
            ]
        )
        .unwrap(),
        Some(PathBuf::from("first/widget.h"))
    );
    assert!(lookup(&[], IncludeStyle::Angle, &["src/widget.h"]).is_err());
}
#[test]
fn differing_compile_entries_do_not_choose_an_arbitrary_configuration() {
    let first = config(serde_json::json!(["c++", "-Ifirst"]));
    let second = config(serde_json::json!(["c++", "-Isecond"]));
    assert!(
        lookup(
            &[first, second],
            IncludeStyle::Angle,
            &["first/widget.h", "second/widget.h"]
        )
        .unwrap_err()
        .contains("conflicting")
    );
}
#[test]
fn command_strings_are_data_and_shell_expansion_is_rejected() {
    assert_eq!(
        command_words("c++ -I 'include space' -c src/main.cpp").unwrap(),
        ["c++", "-I", "include space", "-c", "src/main.cpp"]
    );
    for command in [
        "c++ -I$(touch /tmp/no)",
        "c++ `pwd`",
        "c++; build",
        "c++ -I\"$HOME\"",
    ] {
        assert!(command_words(command).is_err());
    }
}
#[test]
fn repository_paths_normalize_parents_without_escaping() {
    assert_eq!(
        repository_path(Path::new("/repo"), Path::new("build"), "../include"),
        Some(PathBuf::from("include"))
    );
    assert_eq!(
        repository_path(Path::new("/repo"), Path::new(""), "../outside"),
        None
    );
    assert_eq!(
        repository_path(Path::new("/repo"), Path::new(""), "/outside"),
        None
    );
}
#[test]
fn response_files_and_forced_includes_remain_configuration_limits() {
    for argument in ["@flags.txt", "-include", "--sysroot=/sdk", "-I-"] {
        assert!(
            search_paths(
                Path::new("/repo"),
                Path::new(""),
                &["c++".to_owned(), argument.to_owned()]
            )
            .is_err()
        );
    }
}
#[test]
fn standard_headers_remain_external_after_repository_search() {
    assert_eq!(
        resolve(
            &[],
            Path::new("src/main.c"),
            "stdio.h",
            IncludeStyle::Angle,
            |_| false
        )
        .unwrap(),
        None
    );
    let config = config(serde_json::json!(["c++", "-Iinclude"]));
    assert_eq!(
        resolve(
            &[config],
            Path::new("src/main.cpp"),
            "vector",
            IncludeStyle::Angle,
            |path| path == Path::new("include/vector")
        )
        .unwrap(),
        Some(PathBuf::from("include/vector"))
    );
}
#[test]
fn system_classification_overrides_a_duplicate_include_directory() {
    let config = config(serde_json::json!([
        "c++",
        "-Iinc",
        "-Iother",
        "-isysteminc"
    ]));
    assert_eq!(
        lookup(
            &[config],
            IncludeStyle::Angle,
            &["inc/widget.h", "other/widget.h"]
        )
        .unwrap(),
        Some(PathBuf::from("other/widget.h"))
    );
}
#[test]
fn standard_headers_precede_after_directories() {
    let config = config(serde_json::json!(["c++", "-idirafterlate"]));
    assert_eq!(
        resolve(
            &[config],
            Path::new("src/main.cpp"),
            "vector",
            IncludeStyle::Angle,
            |path| path == Path::new("late/vector")
        )
        .unwrap(),
        None
    );
}
#[test]
fn external_search_root_never_guesses_a_later_repository_edge() {
    let config = config(serde_json::json!(["c++", "-I/external", "-Iinclude"]));
    assert!(
        lookup(&[config], IncludeStyle::Angle, &["include/widget.h"])
            .unwrap_err()
            .contains("outside")
    );
}
