use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;

use crate::offense::OffenseKind;

/// Raw YAML structure for `.rubyfast.yml` / `.fasterer.yml`.
#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    speedups: HashMap<String, bool>,
    #[serde(default)]
    exclude_paths: Vec<String>,
}

/// Parsed configuration controlling which offenses to report and which paths to skip.
#[derive(Debug, Clone, Default)]
pub struct Config {
    disabled_offenses: Vec<OffenseKind>,
    pub exclude_patterns: Vec<String>,
}

impl Config {
    /// Load config by searching for `.rubyfast.yml` / `.fasterer.yml` starting from `start_dir` and walking up.
    /// Returns default config if no file is found.
    pub fn load(start_dir: &Path) -> Result<Self> {
        match find_config_file(start_dir) {
            Some(path) => Self::from_file(&path),
            None => Ok(Self::default()),
        }
    }

    /// Parse a specific config file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Self::parse_yaml(&contents)
    }

    /// Parse config from a YAML string.
    pub fn parse_yaml(yaml: &str) -> Result<Self> {
        let raw: Option<RawConfig> = serde_yaml::from_str(yaml)?;
        let raw = raw.unwrap_or_default();

        let disabled_offenses = OffenseKind::all()
            .iter()
            .filter(|kind| raw.speedups.get(kind.config_key()) == Some(&false))
            .copied()
            .collect();

        Ok(Self {
            disabled_offenses,
            exclude_patterns: raw.exclude_paths,
        })
    }

    /// Check if an offense kind is enabled (not disabled by config).
    pub fn is_enabled(&self, kind: OffenseKind) -> bool {
        !self.disabled_offenses.contains(&kind)
    }
}

/// Walk up from `start_dir` looking for `.rubyfast.yml` (preferred) or `.fasterer.yml` (fallback).
fn find_config_file(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir.to_path_buf();
    loop {
        let preferred = dir.join(".rubyfast.yml");
        if preferred.is_file() {
            return Some(preferred);
        }
        let fallback = dir.join(".fasterer.yml");
        if fallback.is_file() {
            return Some(fallback);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_enables_all() {
        let config = Config::default();
        for kind in OffenseKind::all() {
            assert!(config.is_enabled(*kind));
        }
    }

    #[test]
    fn empty_yaml_enables_all() {
        let config = Config::parse_yaml("").unwrap();
        for kind in OffenseKind::all() {
            assert!(config.is_enabled(*kind));
        }
    }

    #[test]
    fn disable_specific_offense() {
        let yaml = "speedups:\n  for_loop_vs_each: false\n";
        let config = Config::parse_yaml(yaml).unwrap();
        assert!(!config.is_enabled(OffenseKind::ForLoopVsEach));
        assert!(config.is_enabled(OffenseKind::ShuffleFirstVsSample));
    }

    #[test]
    fn exclude_paths_parsed() {
        let yaml = "exclude_paths:\n  - 'vendor/**/*.rb'\n  - 'spec/**/*.rb'\n";
        let config = Config::parse_yaml(yaml).unwrap();
        assert_eq!(config.exclude_patterns.len(), 2);
        assert_eq!(config.exclude_patterns[0], "vendor/**/*.rb");
    }

    #[test]
    fn all_speedups_true_enables_all() {
        let yaml = "speedups:\n  for_loop_vs_each: true\n  gsub_vs_tr: true\n";
        let config = Config::parse_yaml(yaml).unwrap();
        assert!(config.is_enabled(OffenseKind::ForLoopVsEach));
        assert!(config.is_enabled(OffenseKind::GsubVsTr));
    }

    #[test]
    fn unknown_speedup_key_ignored() {
        let yaml = "speedups:\n  made_up_rule: false\n";
        let config = Config::parse_yaml(yaml).unwrap();
        for kind in OffenseKind::all() {
            assert!(config.is_enabled(*kind));
        }
    }

    #[test]
    fn invalid_yaml_returns_error() {
        let result = Config::parse_yaml("speedups: [invalid");
        assert!(result.is_err());
    }

    #[test]
    fn load_rubyfast_yml() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".rubyfast.yml"),
            "speedups:\n  gsub_vs_tr: false\n",
        )
        .unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert!(!config.is_enabled(OffenseKind::GsubVsTr));
    }

    #[test]
    fn load_fasterer_yml_fallback() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".fasterer.yml"),
            "speedups:\n  for_loop_vs_each: false\n",
        )
        .unwrap();
        let config = Config::load(dir.path()).unwrap();
        assert!(!config.is_enabled(OffenseKind::ForLoopVsEach));
    }

    #[test]
    fn load_no_config_returns_default() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = Config::load(dir.path()).unwrap();
        for kind in OffenseKind::all() {
            assert!(config.is_enabled(*kind));
        }
    }

    #[test]
    fn from_file_nonexistent_returns_error() {
        let result = Config::from_file(std::path::Path::new("/nonexistent/.rubyfast.yml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_walks_up_parent_directories() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".rubyfast.yml"),
            "speedups:\n  gsub_vs_tr: false\n",
        )
        .unwrap();
        let sub = dir.path().join("nested").join("deep");
        std::fs::create_dir_all(&sub).unwrap();
        let config = Config::load(&sub).unwrap();
        assert!(!config.is_enabled(OffenseKind::GsubVsTr));
    }
}
