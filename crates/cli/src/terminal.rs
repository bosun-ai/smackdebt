use clap::ValueEnum;
use std::io::{self, IsTerminal};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// Help and argument errors are emitted before a complete Cli can be parsed.
pub(crate) fn help_color(arguments: &[std::ffi::OsString]) -> clap::ColorChoice {
    let mut args = arguments.iter().take_while(|arg| *arg != "--");
    let mut choice = ColorChoice::Auto;
    while let Some(arg) = args.next() {
        let value = if arg == "--color" {
            args.next().and_then(|value| value.to_str())
        } else {
            arg.to_str()
                .and_then(|value| value.strip_prefix("--color="))
        };
        if let Some(parsed) = value.and_then(|value| ColorChoice::from_str(value, false).ok()) {
            choice = parsed;
        }
    }
    match choice {
        ColorChoice::Always => clap::ColorChoice::Always,
        ColorChoice::Never => clap::ColorChoice::Never,
        ColorChoice::Auto
            if std::env::var_os("NO_COLOR").is_some()
                || std::env::var("TERM").as_deref() == Ok("dumb") =>
        {
            clap::ColorChoice::Never
        }
        ColorChoice::Auto => clap::ColorChoice::Auto,
    }
}

pub(crate) fn interactive() -> bool {
    io::stdin().is_terminal()
        && io::stdout().is_terminal()
        && io::stderr().is_terminal()
        && std::env::var_os("CI").is_none()
        && std::env::var("TERM").as_deref() != Ok("dumb")
}

pub(crate) fn configure(choice: ColorChoice) {
    let capable = std::env::var("TERM").as_deref() != Ok("dumb");
    let no_color = std::env::var_os("NO_COLOR").is_some();
    console::set_colors_enabled(color(
        choice,
        io::stdout().is_terminal() && capable,
        no_color,
    ));
    console::set_colors_enabled_stderr(color(
        choice,
        io::stderr().is_terminal() && capable,
        no_color,
    ));
}

pub(crate) fn options(
    choice: ColorChoice,
    is_terminal: bool,
    all: bool,
) -> smackdebt_output::TerminalOptions {
    let capable = is_terminal && std::env::var("TERM").as_deref() != Ok("dumb");
    smackdebt_output::TerminalOptions::new(
        width(is_terminal, std::env::var("COLUMNS").ok().as_deref()),
        all,
        color(choice, capable, std::env::var_os("NO_COLOR").is_some()),
    )
    .with_decorations(decorations(choice, capable))
}

pub(crate) fn width(is_terminal: bool, columns: Option<&str>) -> usize {
    if let Some(width) = columns
        .and_then(|value| value.parse().ok())
        .filter(|width| *width >= 40)
    {
        return width;
    }
    if !is_terminal {
        return 100;
    }
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(width), _)| usize::from(width))
        .filter(|width| *width >= 40)
        .unwrap_or(100)
}

pub(crate) const fn color(choice: ColorChoice, is_terminal: bool, no_color: bool) -> bool {
    match choice {
        ColorChoice::Auto => is_terminal && !no_color,
        ColorChoice::Always => true,
        ColorChoice::Never => false,
    }
}

/// Whether glyphs and the tier bar decorate the words.
///
/// Decoration is resolved beside color and needs no option of its own.
/// `NO_COLOR` removes styling only, so a terminal keeps its glyphs while a
/// pipe receives the same report in words alone.
pub(crate) const fn decorations(choice: ColorChoice, is_terminal: bool) -> bool {
    match choice {
        ColorChoice::Auto => is_terminal,
        ColorChoice::Always => true,
        ColorChoice::Never => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_prefers_columns_and_has_a_redirect_default() {
        assert_eq!(width(false, Some("80")), 80);
        assert_eq!(width(false, None), 100);
        assert_eq!(width(false, Some("20")), 100);
    }

    #[test]
    fn decorations_follow_the_terminal_without_a_new_option() {
        assert!(decorations(ColorChoice::Auto, true));
        assert!(!decorations(ColorChoice::Auto, false));
        assert!(decorations(ColorChoice::Always, false));
        assert!(!decorations(ColorChoice::Never, true));
    }

    #[test]
    fn color_honors_mode_terminal_and_no_color() {
        assert!(color(ColorChoice::Always, false, true));
        assert!(!color(ColorChoice::Never, true, false));
        assert!(color(ColorChoice::Auto, true, false));
        assert!(!color(ColorChoice::Auto, true, true));
        assert!(!color(ColorChoice::Auto, false, false));
    }
}
