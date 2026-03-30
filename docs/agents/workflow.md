# Git and PR workflow

## Branching

- Do not commit directly to `main`.
- Start new work from `origin/main`.
- Rebase onto `origin/main` regularly while working to minimize merge conflicts.

## Pull requests

- All changes must go through a PR.
- Open a PR with a clear title and summary.
- Always check the PR for merge conflicts before merging.
- If the PR is dirty or conflicted, rebase onto the latest `origin/main` before merging.
- Handle merge conflicts with rebase, not merge commits.
- Only use squash merging.

## CI management

- After creating or updating a PR, use the scheduler to monitor CI until all required checks pass.
- If checks fail, fix them on the branch and push updates.
- Before merging, confirm required checks are green and the PR merge state is `CLEAN`.

## Merge policy

- Merge only after checks pass.
- If checks fail, continue iterating on the same branch until they pass.
