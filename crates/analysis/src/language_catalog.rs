//! Source identities and names, independent of parsers and filesystem access.

macro_rules! languages {
    ($( $variant:ident => ($name:literal, $key:literal, [$($extension:literal),*], [$($filename:literal),*]) ),* $(,)?) => {
        /// Language classification retained in report facts.
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum Language { $($variant,)* }

        impl Language {
            /// The stable language identifier written in reports.
            pub const fn key(self) -> &'static str {
                match self { $(Self::$variant => $key,)* }
            }

            /// The human-readable language name.
            pub const fn name(self) -> &'static str {
                match self { $(Self::$variant => $name,)* }
            }

            /// Classifies a filename without reading its contents.
            pub fn from_filename(filename: &str) -> Option<Self> {
                if filename.ends_with(".blade.php") { return Some(Self::Unknown); }
                $(if [$($filename),*].contains(&filename) { return Some(Self::$variant); })*
                let extension = filename.rsplit_once('.')?.1;
                match extension {
                    $($($extension => Some(Self::$variant),)*)*
                    extension if unsupported_name(extension).is_some() => Some(Self::Unknown),
                    _ => None,
                }
            }
        }
    };
}

languages! {
    C => ("C", "c", ["c"], []),
    Cpp => ("C++", "cpp", ["h", "cc", "hh", "cpp", "hpp", "cxx", "hxx"], []),
    Java => ("Java", "java", ["java"], []),
    JavaScript => ("JavaScript", "javascript", ["js", "mjs", "cjs"], []),
    Jsx => ("JavaScript", "jsx", ["jsx"], []),
    Python => ("Python", "python", ["py"], []),
    Rust => ("Rust", "rust", ["rs"], []),
    TypeScript => ("TypeScript", "typescript", ["ts", "mts", "cts"], []),
    Tsx => ("TypeScript", "tsx", ["tsx"], []),
    Ruby => ("Ruby", "ruby", ["rb", "rake", "gemspec"], ["Rakefile", "Gemfile"]),
    Vue => ("Vue", "vue", ["vue"], []),
    Astro => ("Astro", "astro", ["astro"], []),
    Kotlin => ("Kotlin", "kotlin", ["kt", "kts"], []),
    Go => ("Go", "go", ["go"], []),
    Php => ("PHP", "php", ["php", "phtml"], []),
    Unknown => ("Unknown", "unknown", [], []),
}

fn unsupported_name(extension: &str) -> Option<&'static str> {
    Some(match extension {
        "swift" => "Swift",
        "cs" => "C#",
        "razor" | "cshtml" => "Razor",
        "scala" | "sc" => "Scala",
        "ex" | "exs" => "Elixir",
        "dart" => "Dart",
        _ => return None,
    })
}

impl Language {
    /// The declared source name, including recognized unsupported languages.
    pub fn source_name(filename: &str) -> Option<&'static str> {
        if filename.ends_with(".blade.php") {
            return Some("Blade");
        }
        match Self::from_filename(filename)? {
            Self::Unknown => unsupported_name(filename.rsplit_once('.')?.1),
            language => Some(language.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_identity_and_names_share_one_catalog() {
        for (filename, language, name) in [
            ("x.tsx", Language::Tsx, "TypeScript"),
            ("Rakefile", Language::Ruby, "Ruby"),
            ("x.rake", Language::Ruby, "Ruby"),
            ("x.go", Language::Go, "Go"),
            ("x.php", Language::Php, "PHP"),
        ] {
            assert_eq!(Language::from_filename(filename), Some(language));
            assert_eq!(Language::source_name(filename), Some(name));
        }
        assert_eq!(Language::from_filename("image.png"), None);
    }
}
