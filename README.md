# Multi MR

## TODO's

- [ ] make a list of UI/UX improvements: 20mins
- [ ] Write a readme with a description of the project, how to install it, how to use it, and how to contribute: 20mins

## Features

- [x] run simplest verion at level of `gudlab`
  - [x] Actually interface with the glab cli as backend.
    - [ ] Setup in such a way that we could also easilly switch to the gitlab native crate.
  - [x] add logic to automatically create branches in the src_repo's
- [x] place default user in `config.toml`, so it can be used to assign the MR.
- [x] add dry run command to test the workflow without actually creating branches or merge requests.
- [ ] vizualize the branches of the src_repo's
- [ ] Introduce option to create MR's as draft
- [ ] add related merge requests in description of the merge request
- [x] add some logic to handle a pre-commit hook that changes some files.
- [ ] setup the CLI so it can be used to overwrite the TOML config file.
- [ ] use external crates for more natural user prompting. https://github.com/ratatui/ratatui/tree/main/examples/apps/input-form

## Fixes

- [x] currently `glab` output messes up the tui.
  - [x] find out what causes the output
  - [x] show the output in the tui window instead of the terminal or just close the tui and then run the commands afterwards.
- [ ] if I cd into src_repos and run `mmr`, I get the error "Assignee is required" this is not very user friendly.


## Workflows

- **from main workflow::** starting from everything on main, make some quick changes on develop without commiting them, then automatically create new branches, commit the changes, push the branch, and create a merge request.
  - good for small cross cutting changes, like updating pre-commit tool versions or updating dependencies.
- **from ft/fx workflow::** we are already on various feature branches, we just want to create identical merge requests for all of them.
  - Good for when started working on a single feature branch, but then later realize that adding this feature will require changes in multiple repositories.
  - [ ] is there any harm in creating a new branch anyway? I think not.
