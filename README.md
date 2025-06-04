# Multi MR

## TODO's

- [ ] place default user in config.toml, so it can be used to assign the MR.
- [ ] Actually interface with the glab cli as backend. Setup in such a way that we could also easilly switch to the gitlab native crate.
- [ ] Introduce option to create MR's as draft
- [ ] vizualize the branches of the src_repo's
- [ ] add logic to automatically create branches in the src_repo's

## Workflows

- **from main workflow::** starting from everything on main, make some quick changes on develop without commiting them, then automatically create new branches, commit the changes, push the branch, and create a merge request.
  - good for small cross cutting changes, like updating pre-commit tool versions or updating dependencies.
- **from ft/fx workflow::** we are already on various feature branches, we just want to create identical merge requests for all of them.
  - Good for when started working on a single feature branch, but then later realize that adding this feature will require changes in multiple repositories.
