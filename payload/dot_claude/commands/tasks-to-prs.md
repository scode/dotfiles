Perform the tasks listed in TASKS.md.

For each task:

  - If the task is already marked DONE, skip it.
  - Execute the task.
  - Consult the code reviewer agent and apply any feedback.
  - Run all tests/lints/formatters/etc.
  - Use the graphite skill to make a PR (NOT auto-merging) so the user can review.
    Create a sequence of stacked diffs in the order of the tasks. Start the work
    based on whatever branch the user is currently on.
  - When the PR has been created, mark it DONE in TASKS.md.

When given instructions to change something about what you produced in
a previous PR, make sure to stash away anything you are working on
before moving back and updating the older PR. Never let code you wrote
for a separate task be amended into the commit for an unrelated PR.
