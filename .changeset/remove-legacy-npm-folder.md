---
"@m-d-t/cli": refactor
"@m-d-t/skills": refactor
---

# Remove the legacy npm source folder

The old `npm/` tree has been removed now that npm packages live under `packages/`. Tests and repository metadata now point at the generated package launcher and package directories under `packages/`.
