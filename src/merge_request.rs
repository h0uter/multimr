use color_eyre::Result;

use super::utils;
use crate::config;

/// Represents a merge request to be created.
#[derive(Debug)]
pub struct MergeRequest {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) reviewers: Vec<String>,
    pub(crate) labels: Vec<String>,
    pub(crate) assignee: Option<String>,
}

impl MergeRequest {
    /// Construct a command to create a merge request for the cwd repo using the `glab` CLI.
    /// If the current branch is main or master, create a new branch
    pub(crate) fn create(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new("glab");
        cmd.arg("mr").arg("create");

        if let Some(assignee) = &self.assignee {
            cmd.arg("--assignee").arg(assignee);
        }

        if !self.reviewers.is_empty() {
            for reviewer in &self.reviewers {
                cmd.arg("--reviewer").arg(reviewer);
            }
        }

        if !self.labels.is_empty() {
            for label in &self.labels {
                cmd.arg("--label").arg(label);
            }
        }

        let current_branch = utils::get_current_branch();

        cmd.arg("--title").arg(&self.title);
        cmd.arg("--description").arg(&self.description);

        if config::DEFAULT_BRANCHES.contains(&current_branch.as_str()) {
            // If the current branch is main or master, create a new branch

            println!();

            std::process::Command::new("git")
                .arg("switch")
                .arg("-c")
                .arg(self.title.replace(' ', "-"))
                .status()
                .expect("Failed to create new branch");

            println!();

            std::process::Command::new("git")
                .arg("add")
                .arg(".")
                .status()
                .expect("Failed to add changes");

            std::process::Command::new("git")
                .arg("commit")
                .arg("-am")
                .arg(&self.title)
                .status()
                .or_else(|_e| -> Result<std::process::ExitStatus, std::io::Error> {
                    // Retry once if adding and committing fails, this might happen if the pre-commit hook formats the code
                    // TODO: test this.
                    std::process::Command::new("git")
                        .arg("add")
                        .arg(".")
                        .status()
                        .expect("Failed to add changes Second attempt");

                    println!();

                    let status = std::process::Command::new("git")
                        .arg("commit")
                        .arg("-am")
                        .arg(&self.title)
                        .status()
                        .expect("Failed to commit changes second attempt");

                    Ok(status)
                })
                .expect("Failed to commit changes twice.");

            cmd.arg("--push");
        } else {
            // If not, just use the current branch
            cmd.arg("--yes");
        }

        cmd
    }

    /// Run the command to create the merge request.
    pub(crate) fn run(&self, mut cmd: std::process::Command) {
        let status = cmd.status().expect("Failed to execute command");
        if !status.success() {
            eprintln!("Failed to create merge request: {:?}", status);
        } else {
            println!("Merge request created successfully.");
        }
    }

    /// Print the command that would be run, useful for dry runs.
    pub(crate) fn dry_run(&self, cmd: std::process::Command) {
        println!(
            "Current directory: {}",
            std::env::current_dir().unwrap().display()
        );

        println!("Dry run command: {:?}", cmd);
    }
}
