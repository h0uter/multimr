//! Tests for the Multi MR application

use crate::app::App;
use crate::config::Config;
use crate::*;
use std::path::PathBuf;

#[test]
fn test_app_dirs_populated() {
    let app = app::App::new(Config::default());
    // Should at least have dirs as a Vec
    assert!(app.dirs.is_empty() || app.dirs.iter().all(|d| !d.is_empty()));
}

#[test]
fn test_config_default() {
    let cfg = Config::default();
    assert!(
        cfg.working_dir.as_os_str().is_empty()
            || cfg.working_dir.exists()
            || cfg.working_dir == PathBuf::new()
    );
    assert!(cfg.reviewers.is_empty());
    assert!(cfg.labels.is_empty());
}

#[test]
fn test_merge_request_fields() {
    let mr = merge_request::MergeRequest {
        title: "Test".to_string(),
        description: "Desc".to_string(),
        reviewers: vec!["alice".to_string()],
        labels: vec!["bug".to_string()],
        assignee: Some("bob".to_string()),
    };
    assert_eq!(mr.title, "Test");
    assert_eq!(mr.description, "Desc");
    assert_eq!(mr.reviewers, vec!["alice"]);
    assert_eq!(mr.labels, vec!["bug"]);
    assert_eq!(mr.assignee, Some("bob".to_string()));
}

#[test]
fn test_app_quit_sets_running_false() {
    let mut app = app::App::new(Config::default());
    app.running = true;
    app.quit();
    assert!(!app.running);
}

#[test]
fn test_app_selected_repos_toggle() {
    let mut app = App::new(Config::default());
    app.dirs = vec!["repo1".to_string(), "repo2".to_string()];
    app.selected_index = 0;
    app.selected_repos.insert(0);
    assert!(app.selected_repos.contains(&0));
    app.selected_repos.remove(&0);
    assert!(!app.selected_repos.contains(&0));
}

#[test]
fn test_app_selected_reviewers_toggle() {
    let mut app = App::new(Config::default());
    app.config.reviewers = vec!["alice".to_string(), "bob".to_string()];
    app.reviewer_index = 1;
    app.selected_reviewers.insert(1);
    assert!(app.selected_reviewers.contains(&1));
    app.selected_reviewers.remove(&1);
    assert!(!app.selected_reviewers.contains(&1));
}

// failing in ci due to no branch and no glab installed.

// #[test]
// fn test_get_current_branch_returns_string() {
//     let branch = utils::get_current_branch();
//     assert!(branch.is_ascii());
// }

// #[test]
// fn test_ensure_glab_installed_does_not_panic() {
//     // This will exit if glab is not installed, so just check it doesn't panic
//     let _ = std::panic::catch_unwind(utils::ensure_glab_installed);
// }

#[test]
fn test_app_new_with_dry_run() {
    let app = App::new(Config {
        dry_run: true,
        ..Config::default()
    });
    assert!(app.config.dry_run);
}
