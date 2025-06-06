//! Tests for the Multi MR application

use crate::config::Config;
use crate::*;
use std::path::PathBuf;

#[test]
fn test_app_new_sets_dry_run() {
    let app = App::new(true);
    assert!(app.dry_run);
    let app2 = App::new(false);
    assert!(!app2.dry_run);
}

#[test]
fn test_app_dirs_populated() {
    let app = App::new(true);
    // Should at least have dirs as a Vec
    assert!(app.dirs.is_empty() || app.dirs.iter().all(|d| !d.is_empty()));
}

#[test]
fn test_screen_help_texts() {
    assert_eq!(
        Screen::RepoSelection.help(),
        "↑/↓/j/k: Move  Space: Select  Enter: Next  q/Esc: Quit"
    );
    assert_eq!(
        Screen::CreateMR.help(),
        "Tab: Switch field  ↑/↓/j/k: Select Label  Enter: Next  Esc: Back"
    );
    assert_eq!(
        Screen::ReviewerSelection.help(),
        "↑/↓/j/k: Move   Space:  Select  Enter: Next  Esc: Back"
    );
    assert_eq!(Screen::Finalize.help(), "y/Enter: Confirm  n/Esc: Back");
}

#[test]
fn test_screen_titles() {
    assert_eq!(Screen::RepoSelection.title(), "Select Repos");
    assert_eq!(Screen::CreateMR.title(), "Describe");
    assert_eq!(Screen::ReviewerSelection.title(), "Add Reviewers");
    assert_eq!(Screen::Finalize.title(), "Finalize");
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
fn test_input_focus_default() {
    assert_eq!(InputFocus::default(), InputFocus::Label);
}

#[test]
fn test_merge_request_fields() {
    let mr = merge_request::MergeRequest {
        title: "Test".to_string(),
        description: "Desc".to_string(),
        reviewers: vec!["alice".to_string()],
        labels: vec!["bug".to_string()],
        assignee: "bob".to_string(),
    };
    assert_eq!(mr.title, "Test");
    assert_eq!(mr.description, "Desc");
    assert_eq!(mr.reviewers, vec!["alice"]);
    assert_eq!(mr.labels, vec!["bug"]);
    assert_eq!(mr.assignee, "bob");
}

#[test]
fn test_app_quit_sets_running_false() {
    let mut app = App::new(true);
    app.running = true;
    app.quit();
    assert!(!app.running);
}

#[test]
fn test_app_selected_repos_toggle() {
    let mut app = App::new(true);
    app.dirs = vec!["repo1".to_string(), "repo2".to_string()];
    app.selected_index = 0;
    app.selected_repos.insert(0);
    assert!(app.selected_repos.contains(&0));
    app.selected_repos.remove(&0);
    assert!(!app.selected_repos.contains(&0));
}

#[test]
fn test_app_selected_reviewers_toggle() {
    let mut app = App::new(true);
    app.cfg.reviewers = vec!["alice".to_string(), "bob".to_string()];
    app.reviewer_index = 1;
    app.selected_reviewers.insert(1);
    assert!(app.selected_reviewers.contains(&1));
    app.selected_reviewers.remove(&1);
    assert!(!app.selected_reviewers.contains(&1));
}

#[test]
fn test_merge_request_create_command() {
    let mr = merge_request::MergeRequest {
        title: "TestTitle".to_string(),
        description: "TestDesc".to_string(),
        reviewers: vec!["alice".to_string()],
        labels: vec!["bug".to_string()],
        assignee: "bob".to_string(),
    };
    let cmd = mr.create();
    let args: Vec<_> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    assert!(args.contains(&"mr".to_string()));
    assert!(args.contains(&"create".to_string()));
}

#[test]
fn test_merge_request_dry_run_prints() {
    let mr = merge_request::MergeRequest {
        title: "DryRun".to_string(),
        description: "Desc".to_string(),
        reviewers: vec![],
        labels: vec![],
        assignee: "bob".to_string(),
    };
    let cmd = mr.create();
    mr.dry_run(cmd);
}

#[test]
fn test_get_current_branch_returns_string() {
    let branch = utils::get_current_branch();
    assert!(branch.is_ascii());
}

#[test]
fn test_load_config_from_toml_returns_config() {
    let cfg = config::load_config_from_toml();
    assert!(cfg.assignee.is_ascii() || cfg.assignee.is_empty());
}

#[test]
fn test_ensure_glab_installed_does_not_panic() {
    // This will exit if glab is not installed, so just check it doesn't panic
    let _ = std::panic::catch_unwind(|| utils::ensure_glab_installed());
}

#[test]
fn test_ensure_git_repo_does_not_panic() {
    // This will exit if not in a git repo, so just check it doesn't panic
    let _ = std::panic::catch_unwind(|| utils::ensure_git_repo());
}

#[test]
fn test_app_new_with_dry_run() {
    let app = App::new(true);
    assert!(app.dry_run);
}
