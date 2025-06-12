//! Handles loading the configuration for the multimr application from a TOML file .
use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

pub(crate) const CONFIG_FILE: &str = "multimr.toml";
pub(crate) const DEFAULT_BRANCHES: [&str; 2] = ["main", "master"];

/// Total Configuration for the application.
/// First read from a `multimr.toml` file, then overwritten with optional cli args.
#[derive(Debug, Default, Clone)]
pub(crate) struct Config {
    pub working_dir: PathBuf,
    pub reviewers: Vec<String>,
    pub labels: HashMap<String, String>,
    pub assignee: Option<String>,
    /// Is this a dry run? If true, no merge requests will be created.
    pub dry_run: bool,
}

/// User configuration is loaded from a `multimr.toml` file in the current working directory.
pub(crate) fn load_config_from_toml() -> Config {
    let content = std::fs::read_to_string(CONFIG_FILE).unwrap_or_default();

    /// This contains only the fields we need from the TOML file.
    #[derive(Deserialize)]
    struct ConfigToml {
        reviewers: Option<Vec<String>>,
        labels: Option<HashMap<String, String>>,
        working_dir: Option<String>,
        assignee: Option<String>,
    }

    // if the entire parsing fails return a config with None values
    let parsed: ConfigToml = toml::from_str(&content).unwrap_or(ConfigToml {
        reviewers: None,
        labels: None,
        working_dir: None,
        assignee: None,
    });

    // check if a root is specified in toml, if not use current directory
    let working_dir_str = parsed.working_dir.unwrap_or(".".to_string());

    // there is a root, now create a PathBuf
    let working_dir = if working_dir_str.starts_with('/') || working_dir_str.starts_with('\\') {
        // root // absolute path
        PathBuf::from(&working_dir_str)
            .canonicalize()
            .expect("Failed to resolve absolute path")
    } else {
        // working dir is specified as relative path
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(working_dir_str)
            .canonicalize()
            .expect("Failed to resolve relative path")
    };

    // if individual fields fail, we use default values
    Config {
        working_dir,
        reviewers: parsed.reviewers.unwrap_or_default(),
        labels: parsed
            .labels
            .map(|m| m.into_iter().collect())
            .unwrap_or_default(),
        assignee: parsed.assignee,
        dry_run: false, // Default to false, can be set later
    }
}
