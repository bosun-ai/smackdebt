use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::arguments::parse_days;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectConfig {
    pub(crate) history: Option<String>,
    #[serde(default)]
    pub(crate) exclude: Vec<String>,
    thresholds: Option<ThresholdConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThresholdConfig {
    cognitive: Option<LimitConfig>,
    cyclomatic: Option<LimitConfig>,
    function_lines: Option<LimitConfig>,
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
    pub(crate) fn thresholds(&self) -> ((u32, u32), (u32, u32), (u32, u32)) {
        let pair = |value: Option<LimitConfig>, fallback| {
            value.map_or(fallback, |limit| (limit.watch, limit.high))
        };
        let thresholds = self.thresholds.as_ref();
        (
            pair(thresholds.and_then(|value| value.cognitive), (15, 25)),
            pair(thresholds.and_then(|value| value.cyclomatic), (11, 21)),
            pair(thresholds.and_then(|value| value.function_lines), (50, 100)),
        )
    }

    fn validate(&self) -> Result<(), String> {
        if let Some(history) = &self.history {
            parse_days(history)?;
        }
        if let Some(thresholds) = &self.thresholds {
            for (name, limit) in [
                ("cognitive", thresholds.cognitive),
                ("cyclomatic", thresholds.cyclomatic),
                ("function_lines", thresholds.function_lines),
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
        Ok(())
    }
}
