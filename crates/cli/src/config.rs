use std::fs;
use std::path::Path;

use serde::Deserialize;
use smackdebt_project::{DEFAULT_MINIMUM_TOUCHES, SourceRoleRule};

use crate::arguments::parse_days;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectConfig {
    pub(crate) history: Option<String>,
    #[serde(default)]
    pub(crate) exclude: Vec<String>,
    thresholds: Option<ThresholdConfig>,
    #[serde(default)]
    source_roles: RoleConfig,
    hotspots: Option<HotspotConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_configuration_builds_declarative_project_rules() {
        let config: ProjectConfig = toml::from_str(
            "[source_roles]\ntest = ['spec/**']\nfixture = ['samples/**']\ngenerated = ['src/api.rs']\n",
        )
        .unwrap();
        config.validate().unwrap();
        let rules = config.role_rules();
        let values: Vec<_> = rules
            .iter()
            .map(|rule| (rule.role_name(), rule.pattern()))
            .collect();
        assert_eq!(
            values,
            [
                ("test", "spec/**"),
                ("fixture", "samples/**"),
                ("generated", "src/api.rs")
            ]
        );
    }

    #[test]
    fn nesting_and_parameter_thresholds_have_documented_defaults_and_overrides() {
        let default = ProjectConfig::default();
        let (_, _, _, nesting, parameters) = default.thresholds();
        assert_eq!(nesting, (4, 7));
        assert_eq!(parameters, (6, 9));
        let config: ProjectConfig = toml::from_str(
            "[thresholds]\nnesting = { watch = 2, high = 3 }\nparameters = { watch = 3, high = 4 }\n",
        )
        .unwrap();
        config.validate().unwrap();
        let (_, _, _, nesting, parameters) = config.thresholds();
        assert_eq!(nesting, (2, 3));
        assert_eq!(parameters, (3, 4));
        let weak: ProjectConfig =
            toml::from_str("[thresholds]\nnesting = { watch = 3, high = 3 }\n").unwrap();
        assert!(weak.validate().is_err());
    }

    #[test]
    fn size_and_hotspot_settings_have_documented_defaults_and_overrides() {
        let default = ProjectConfig::default();
        assert_eq!(default.size_thresholds(), ((400, 800), (300, 600)));
        assert_eq!(default.minimum_hotspot_touches(), 5);
        let config: ProjectConfig = toml::from_str(
            "[thresholds]\nfile_lines = { watch = 200, high = 500 }\ncontainer_lines = { watch = 100, high = 250 }\n[hotspots]\nminimum_touches = 12\n",
        )
        .unwrap();
        config.validate().unwrap();
        assert_eq!(config.size_thresholds(), ((200, 500), (100, 250)));
        assert_eq!(config.minimum_hotspot_touches(), 12);
    }

    #[test]
    fn weak_size_and_hotspot_settings_are_rejected() {
        let sizes: ProjectConfig =
            toml::from_str("[thresholds]\nfile_lines = { watch = 0, high = 5 }\n").unwrap();
        assert!(sizes.validate().is_err());
        let touches: ProjectConfig = toml::from_str("[hotspots]\nminimum_touches = 0\n").unwrap();
        assert_eq!(
            touches.validate(),
            Err("hotspots.minimum_touches requires at least one touch".to_owned())
        );
    }

    #[test]
    fn one_pattern_cannot_be_assigned_two_configured_roles() {
        let config: ProjectConfig =
            toml::from_str("[source_roles]\ntest = ['shared/**']\nfixture = ['shared/**']\n")
                .unwrap();
        assert_eq!(
            config.validate(),
            Err("source_roles assigns pattern \"shared/**\" to both test and fixture".to_owned())
        );
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleConfig {
    #[serde(default)]
    primary: Vec<String>,
    #[serde(default)]
    test: Vec<String>,
    #[serde(default)]
    example: Vec<String>,
    #[serde(default)]
    benchmark: Vec<String>,
    #[serde(default)]
    fixture: Vec<String>,
    #[serde(default)]
    generated: Vec<String>,
    #[serde(default)]
    vendored: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThresholdConfig {
    cognitive: Option<LimitConfig>,
    cyclomatic: Option<LimitConfig>,
    function_lines: Option<LimitConfig>,
    file_lines: Option<LimitConfig>,
    container_lines: Option<LimitConfig>,
    nesting: Option<LimitConfig>,
    parameters: Option<LimitConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HotspotConfig {
    minimum_touches: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitConfig {
    watch: u32,
    high: u32,
}

pub(crate) fn load(selected: &Path) -> Result<ProjectConfig, String> {
    let start = if selected.is_file() {
        selected.parent().unwrap_or(selected)
    } else {
        selected
    };
    let mut directory = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("cannot resolve current directory: {error}"))?
            .join(start)
    };
    loop {
        let candidate = directory.join(".smackdebt.toml");
        if candidate.is_file() {
            let source = fs::read_to_string(&candidate)
                .map_err(|error| format!("cannot read {}: {error}", candidate.display()))?;
            let config: ProjectConfig = toml::from_str(&source)
                .map_err(|error| format!("invalid {}: {error}", candidate.display()))?;
            config.validate()?;
            return Ok(config);
        }
        if !directory.pop() {
            return Ok(ProjectConfig::default());
        }
    }
}

impl ProjectConfig {
    pub(crate) fn role_rules(&self) -> Vec<SourceRoleRule> {
        let mut rules = Vec::new();
        for (make_rule, patterns) in [
            (
                SourceRoleRule::primary as fn(String) -> SourceRoleRule,
                &self.source_roles.primary,
            ),
            (SourceRoleRule::test, &self.source_roles.test),
            (SourceRoleRule::example, &self.source_roles.example),
            (SourceRoleRule::benchmark, &self.source_roles.benchmark),
            (SourceRoleRule::fixture, &self.source_roles.fixture),
            (SourceRoleRule::generated, &self.source_roles.generated),
            (SourceRoleRule::vendored, &self.source_roles.vendored),
        ] {
            rules.extend(patterns.iter().cloned().map(make_rule));
        }
        rules
    }
    #[allow(clippy::type_complexity)]
    pub(crate) fn thresholds(
        &self,
    ) -> ((u32, u32), (u32, u32), (u32, u32), (u32, u32), (u32, u32)) {
        let pair = |value: Option<LimitConfig>, fallback| {
            value.map_or(fallback, |limit| (limit.watch, limit.high))
        };
        let thresholds = self.thresholds.as_ref();
        (
            pair(thresholds.and_then(|value| value.cognitive), (15, 25)),
            pair(thresholds.and_then(|value| value.cyclomatic), (11, 21)),
            pair(thresholds.and_then(|value| value.function_lines), (50, 100)),
            pair(thresholds.and_then(|value| value.nesting), (4, 7)),
            pair(thresholds.and_then(|value| value.parameters), (6, 9)),
        )
    }

    /// The configured file and container size thresholds.
    pub(crate) fn size_thresholds(&self) -> ((u32, u32), (u32, u32)) {
        let pair = |value: Option<LimitConfig>, fallback| {
            value.map_or(fallback, |limit| (limit.watch, limit.high))
        };
        let thresholds = self.thresholds.as_ref();
        (
            pair(thresholds.and_then(|value| value.file_lines), (400, 800)),
            pair(
                thresholds.and_then(|value| value.container_lines),
                (300, 600),
            ),
        )
    }

    /// The configured minimum touch count for a hotspot.
    pub(crate) fn minimum_hotspot_touches(&self) -> u32 {
        self.hotspots
            .as_ref()
            .and_then(|hotspots| hotspots.minimum_touches)
            .unwrap_or(DEFAULT_MINIMUM_TOUCHES)
    }

    fn validate(&self) -> Result<(), String> {
        if let Some(hotspots) = &self.hotspots
            && hotspots.minimum_touches == Some(0)
        {
            return Err("hotspots.minimum_touches requires at least one touch".to_owned());
        }
        if let Some(history) = &self.history {
            parse_days(history)?;
        }
        if let Some(thresholds) = &self.thresholds {
            for (name, limit) in [
                ("cognitive", thresholds.cognitive),
                ("cyclomatic", thresholds.cyclomatic),
                ("function_lines", thresholds.function_lines),
                ("file_lines", thresholds.file_lines),
                ("container_lines", thresholds.container_lines),
                ("nesting", thresholds.nesting),
                ("parameters", thresholds.parameters),
            ] {
                if let Some(limit) = limit
                    && (limit.watch == 0 || limit.high <= limit.watch)
                {
                    return Err(format!(
                        "thresholds.{name} requires watch greater than zero and high greater than watch"
                    ));
                }
            }
        }
        let mut assigned = std::collections::BTreeMap::new();
        for rule in self.role_rules() {
            if rule.pattern().trim().is_empty() {
                return Err("source_roles contains an empty pattern".to_owned());
            }
            if let Some(previous) = assigned.insert(rule.pattern().to_owned(), rule.role_name())
                && previous != rule.role_name()
            {
                return Err(format!(
                    "source_roles assigns pattern {:?} to both {previous} and {}",
                    rule.pattern(),
                    rule.role_name()
                ));
            }
        }
        Ok(())
    }
}
