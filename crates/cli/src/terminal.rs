use crate::arguments::ColorChoice;

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
