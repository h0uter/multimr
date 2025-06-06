use serde::Deserialize;
use std::path::PathBuf;

use std::collections::HashMap;

use super::CONFIG_FILE;

/// Configuration for the application, loaded from a `multimr.toml` file.
#[derive(Debug, Default)]
pub(crate) struct Config {
    pub working_dir: PathBuf,
    pub reviewers: Vec<String>,
    pub labels: HashMap<String, String>,
    pub assignee: String,
}

/// User configuration is loaded from a `multimr.toml` file in the current working directory.
pub(crate) fn load_config_from_toml() -> Config {
    let content = std::fs::read_to_string(CONFIG_FILE).unwrap_or_default();

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
        assignee: parsed.assignee.expect("Assignee is required"),
    }
}
