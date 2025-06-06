//! Helper functions for the multimr application.

/// Getting the current branch is needed to determine if a new branch should be created for the merge request.
pub(crate) fn get_current_branch() -> String {
    let current_branch_output = std::process::Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        .expect("Failed to get current branch");

    String::from_utf8_lossy(&current_branch_output.stdout)
        .trim()
        .to_string()
}

/// Ensure that the `glab` CLI is installed, since it's essential for running multimr.
pub(crate) fn ensure_glab_installed() {
    if std::process::Command::new("glab")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!(
            "[Error] Gitlab CLI `glab` is not installed. Please install it to use this application."
        );
        std::process::exit(1);
    }
}
