[![pre-commit](https://img.shields.io/badge/pre--commit-enabled-brightgreen?logo=pre-commit)](https://pre-commit.com)

<br />
<div align="center">
    <div align="center">
    <img src="https://raw.githubusercontent.com/h0uter/multimr/main/.readme/screen.png" alt="alt text" width="350" height="whatever">
    </div>

  <p align="center">
    <b>MultiMR</b> is a Terminal User Interface (TUI) app to create identical merge requests across multiple repositories. Currently, it only supports Gitlab.
    <br />
    <a href="https://github.com/h0uter/multimr/issues/new?labels=bug&title=New+bug+report">Report Bug</a>
    ·
    <a href="https://github.com/h0uter/multimr/issues/new?labels=enhancement&title=New+feature+request">Request Feature</a>
  </p>
</div>

## Why?

Often in robotics projects certain subsystems are developed in separate repositories. When some update requires cross-cutting changes in multiple repositories, it is useful to create identical merge requests across all of them. This tool automates that process.

## Features

- Create identical merge requests across multiple repositories
- Specify reviewers, assignee and other settings in a `multimr.toml` file
- Override settings with command line arguments
- Preview branches of the repositories before creating merge requests

## CLI

```txt
Easily create identical MR/PRs on multiple repo's.

Usage: multimr [OPTIONS]

Options:
      --dry-run              Run in dry-run mode (do not actually create MRs)
      --assignee <ASSIGNEE>  Overwrite the assignee specified in multimr.toml
  -h, --help                 Print help
  -V, --version              Print version
```
